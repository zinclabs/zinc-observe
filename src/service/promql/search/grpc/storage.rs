// Copyright 2024 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;

use config::{
    get_config, is_local_disk_storage,
    meta::{
        search::{ScanStats, SearchType, Session as SearchSession, StorageType},
        stream::{FileKey, PartitionTimeLevel, StreamPartition, StreamType},
    },
};
use datafusion::{
    arrow::datatypes::Schema,
    error::{DataFusionError, Result},
    prelude::SessionContext,
};
use hashbrown::HashMap;
use infra::{
    cache::file_data,
    schema::{unwrap_partition_time_level, unwrap_stream_settings},
};
use tokio::sync::Semaphore;

use crate::{
    common::meta::stream::StreamParams,
    service::{
        db, file_list,
        search::{
            datafusion::{exec::register_table, file_type::FileType},
            match_source,
        },
    },
};

#[tracing::instrument(name = "promql:search:grpc:storage:create_context", skip_all, fields(org_id = org_id, stream_name = stream_name))]
pub(crate) async fn create_context(
    trace_id: &str,
    org_id: &str,
    stream_name: &str,
    time_range: (i64, i64),
    filters: &mut [(&str, Vec<String>)],
) -> Result<(SessionContext, Arc<Schema>, ScanStats)> {
    // check if we are allowed to search
    if db::compact::retention::is_deleting_stream(org_id, StreamType::Metrics, stream_name, None) {
        log::error!("stream [{}] is being deleted", stream_name);
        return Ok((
            SessionContext::new(),
            Arc::new(Schema::empty()),
            ScanStats::default(),
        ));
    }

    // get latest schema
    let stream_type = StreamType::Metrics;
    let schema = match infra::schema::get(org_id, stream_name, stream_type).await {
        Ok(schema) => schema,
        Err(err) => {
            log::error!("get schema error: {}", err);
            return Err(datafusion::error::DataFusionError::Execution(
                err.to_string(),
            ));
        }
    };
    let stream_settings = unwrap_stream_settings(&schema).unwrap_or_default();
    let partition_time_level =
        unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);

    // rewrite partition filters
    let partition_keys: HashMap<&str, &StreamPartition> = stream_settings
        .partition_keys
        .iter()
        .map(|v| (v.field.as_str(), v))
        .collect();
    for entry in filters.iter_mut() {
        if let Some(partition_key) = partition_keys.get(entry.0) {
            for val in entry.1.iter_mut() {
                *val = partition_key.get_partition_value(val);
            }
        }
    }

    // get file list
    let mut files = get_file_list(
        trace_id,
        org_id,
        stream_name,
        partition_time_level,
        time_range,
        filters,
    )
    .await?;
    if files.is_empty() {
        return Ok((
            SessionContext::new(),
            Arc::new(Schema::empty()),
            ScanStats::default(),
        ));
    }

    // calculate scan size
    let scan_stats = match file_list::calculate_files_size(&files.to_vec()).await {
        Ok(size) => size,
        Err(err) => {
            log::error!("calculate files size error: {}", err);
            return Err(datafusion::error::DataFusionError::Execution(
                "calculate files size error".to_string(),
            ));
        }
    };
    log::info!(
        "promql->search->storage: load files {}, scan_size {}, compressed_size {}",
        scan_stats.files,
        scan_stats.original_size,
        scan_stats.compressed_size
    );

    // load files to local cache
    let (cache_type, deleted_files) = cache_parquet_files(&files, &scan_stats).await?;
    if !deleted_files.is_empty() {
        // remove deleted files
        files.retain(|f| !deleted_files.contains(&f.key));
    }
    log::info!(
        "promql->search->storage: load files {}, into {:?} cache done",
        scan_stats.files,
        cache_type
    );

    // set target partitions based on cache type
    let cfg = get_config();
    let target_partitions = if cache_type == file_data::CacheType::None {
        cfg.limit.query_thread_num
    } else {
        cfg.limit.cpu_num
    };

    let schema = Arc::new(
        schema
            .to_owned()
            .with_metadata(std::collections::HashMap::new()),
    );

    let session = SearchSession {
        id: trace_id.to_string(),
        storage_type: StorageType::Memory,
        search_type: SearchType::Normal,
        work_group: None,
        target_partitions,
    };

    let ctx = register_table(
        &session,
        schema.clone(),
        stream_name,
        &files,
        FileType::PARQUET,
        HashMap::default(),
        false,
        &[],
        None,
    )
    .await?;
    Ok((ctx, schema, scan_stats))
}

#[tracing::instrument(
    name = "promql:search:grpc:storage:get_file_list",
    skip_all,
    fields(org_id, stream_name)
)]
async fn get_file_list(
    trace_id: &str,
    org_id: &str,
    stream_name: &str,
    time_level: PartitionTimeLevel,
    time_range: (i64, i64),
    filters: &[(&str, Vec<String>)],
) -> Result<Vec<FileKey>> {
    let (time_min, time_max) = time_range;
    let results = match file_list::query(
        org_id,
        stream_name,
        StreamType::Metrics,
        time_level,
        time_min,
        time_max,
        true,
    )
    .await
    {
        Ok(results) => results,
        Err(err) => {
            log::error!("[trace_id {trace_id}] get file list error: {}", err);
            return Err(DataFusionError::Execution(
                "get file list error".to_string(),
            ));
        }
    };

    let mut files = Vec::new();
    for file in results {
        if match_source(
            StreamParams::new(org_id, stream_name, StreamType::Metrics),
            Some(time_range),
            filters,
            &file,
            false,
            false,
        )
        .await
        {
            files.push(file.clone());
        }
    }
    Ok(files)
}

#[tracing::instrument(name = "promql:search:grpc:storage:cache_parquet_files", skip_all)]
async fn cache_parquet_files(
    files: &[FileKey],
    scan_stats: &ScanStats,
) -> Result<(file_data::CacheType, Vec<String>)> {
    let cfg = config::get_config();
    let cache_type = if cfg.memory_cache.enabled
        && scan_stats.compressed_size < cfg.memory_cache.skip_size as i64
    {
        // if scan_compressed_size < 80% of total memory cache, use memory cache
        file_data::CacheType::Memory
    } else if !is_local_disk_storage()
        && cfg.disk_cache.enabled
        && scan_stats.compressed_size < cfg.disk_cache.skip_size as i64
    {
        // if scan_compressed_size < 80% of total disk cache, use disk cache
        file_data::CacheType::Disk
    } else {
        // no cache
        return Ok((file_data::CacheType::None, vec![]));
    };

    let mut tasks = Vec::new();
    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.query_thread_num));
    for file in files.iter() {
        let trace_id = "";
        let file_name = file.key.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task: tokio::task::JoinHandle<Option<String>> = tokio::task::spawn(async move {
            let ret = match cache_type {
                file_data::CacheType::Memory => {
                    if !file_data::memory::exist(&file_name).await
                        && !file_data::disk::exist(&file_name).await
                    {
                        file_data::memory::download(trace_id, &file_name)
                            .await
                            .err()
                    } else {
                        None
                    }
                }
                file_data::CacheType::Disk => {
                    if !file_data::disk::exist(&file_name).await {
                        file_data::disk::download(trace_id, &file_name).await.err()
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let ret = if let Some(e) = ret {
                if e.to_string().to_lowercase().contains("not found")
                    || e.to_string().to_lowercase().contains("data size is zero")
                {
                    // delete file from file list
                    log::warn!("found invalid file: {}", file_name);
                    if let Err(e) = file_list::delete_parquet_file(&file_name, true).await {
                        log::error!("promql->search->storage: delete from file_list err: {}", e);
                    }
                    Some(file_name)
                } else {
                    log::error!("promql->search->storage: download file to cache err: {}", e);
                    None
                }
            } else {
                None
            };
            drop(permit);
            ret
        });
        tasks.push(task);
    }

    let mut delete_files = Vec::new();
    for task in tasks {
        match task.await {
            Ok(ret) => {
                if let Some(file) = ret {
                    delete_files.push(file);
                }
            }
            Err(e) => {
                log::error!("promql->search->storage: load file task err: {}", e);
            }
        }
    }

    Ok((cache_type, delete_files))
}
