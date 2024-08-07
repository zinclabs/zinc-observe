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

use std::{path::Path, sync::Arc};

use arrow::array::{new_null_array, ArrayRef};
use config::{
    get_config,
    meta::{
        search::{ScanStats, SearchType, StorageType},
        stream::{FileKey, PartitionTimeLevel, StreamPartition, StreamType},
    },
    utils::{
        file::scan_files,
        parquet::{parse_time_range_from_filename, read_metadata_from_file},
    },
};
use datafusion::arrow::{datatypes::Schema, record_batch::RecordBatch};
use futures::{future::try_join_all, StreamExt};
use hashbrown::HashMap;
use infra::{
    errors::{Error, ErrorCodes},
    schema::{unwrap_partition_time_level, unwrap_stream_settings},
};
use ingester::WAL_PARQUET_METADATA;
use tokio::time::Duration;
use tracing::{info_span, Instrument};

use crate::{
    common::infra::wal,
    service::{
        db, file_list,
        search::{
            datafusion::{exec, file_type::FileType},
            generate_search_schema, generate_select_start_search_schema,
            sql::{Sql, RE_SELECT_WILDCARD},
        },
    },
};

/// search in local WAL, which haven't been sync to object storage
#[tracing::instrument(name = "service:search:wal:parquet", skip_all, fields(org_id = sql.org_id, stream_name = sql.stream_name))]
pub async fn search_parquet(
    trace_id: &str,
    sql: Arc<Sql>,
    stream_type: StreamType,
    work_group: &str,
    timeout: u64,
) -> super::SearchResult {
    let schema_latest = match infra::schema::get(&sql.org_id, &sql.stream_name, stream_type).await {
        Ok(schema) => schema,
        Err(err) => {
            log::error!("[trace_id {trace_id}] get schema error: {}", err);
            return Err(Error::ErrorCode(ErrorCodes::SearchStreamNotFound(
                sql.stream_name.clone(),
            )));
        }
    };

    let stream_settings = unwrap_stream_settings(&schema_latest).unwrap_or_default();
    let partition_time_level =
        unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);
    let defined_schema_fields = stream_settings.defined_schema_fields.unwrap_or_default();

    // get file list
    let files = get_file_list(
        trace_id,
        &sql,
        stream_type,
        &partition_time_level,
        &stream_settings.partition_keys,
    )
    .await?;
    if files.is_empty() {
        return Ok((vec![], ScanStats::new()));
    }

    let mut scan_stats = ScanStats::new();
    let mut lock_files = files.iter().map(|f| f.key.clone()).collect::<Vec<_>>();
    let cfg = get_config();
    // get file metadata to build file_list
    let files_num = files.len();
    let mut new_files = Vec::with_capacity(files_num);
    let files_metadata = futures::stream::iter(files)
        .map(|file| async move {
            let cfg = get_config();
            let r = WAL_PARQUET_METADATA.read().await;
            if let Some(meta) = r.get(file.key.as_str()) {
                let mut file = file;
                file.meta = meta.clone();
                return file;
            }
            drop(r);
            let source_file = cfg.common.data_wal_dir.to_string() + file.key.as_str();
            let meta = read_metadata_from_file(&source_file.into())
                .await
                .unwrap_or_default();
            let mut file = file;
            file.meta = meta;
            WAL_PARQUET_METADATA
                .write()
                .await
                .insert(file.key.clone(), file.meta.clone());
            file
        })
        .buffer_unordered(cfg.limit.cpu_num)
        .collect::<Vec<FileKey>>()
        .await;
    for file in files_metadata {
        if let Some((min_ts, max_ts)) = sql.meta.time_range {
            if file.meta.is_empty() {
                wal::release_files(&[file.key.clone()]).await;
                lock_files.retain(|f| f != &file.key);
                continue;
            }
            if file.meta.min_ts > max_ts || file.meta.max_ts < min_ts {
                log::debug!(
                    "[trace_id {trace_id}] skip wal parquet file: {} time_range: [{},{}]",
                    &file.key,
                    file.meta.min_ts,
                    file.meta.max_ts
                );
                wal::release_files(&[file.key.clone()]).await;
                lock_files.retain(|f| f != &file.key);
                continue;
            }
        }
        new_files.push(file);
    }
    let files = new_files;

    scan_stats.files = files.len() as i64;
    if scan_stats.files == 0 {
        // release all files
        wal::release_files(&lock_files).await;
        return Ok((vec![], scan_stats));
    }

    // fetch all schema versions, group files by version
    let schema_versions = match infra::schema::get_versions(
        &sql.org_id,
        &sql.stream_name,
        stream_type,
        sql.meta.time_range,
    )
    .await
    {
        Ok(versions) => versions,
        Err(err) => {
            log::error!("[trace_id {trace_id}] get schema error: {}", err);
            return Err(Error::ErrorCode(ErrorCodes::SearchStreamNotFound(
                sql.stream_name.clone(),
            )));
        }
    };
    if schema_versions.is_empty() {
        return Ok((vec![], ScanStats::new()));
    }
    let schema_latest_id = schema_versions.len() - 1;

    let mut files_group: HashMap<usize, Vec<FileKey>> =
        HashMap::with_capacity(schema_versions.len());
    let mut scan_stats = ScanStats::new();
    if !cfg.common.widening_schema_evolution || schema_versions.len() == 1 {
        let files = files.to_vec();
        scan_stats = match file_list::calculate_files_size(&files).await {
            Ok(size) => size,
            Err(err) => {
                // release all files
                wal::release_files(&lock_files).await;
                log::error!("[trace_id {trace_id}] calculate files size error: {}", err);
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    "calculate files size error".to_string(),
                )));
            }
        };
        files_group.insert(schema_latest_id, files);
    } else {
        scan_stats.files = files.len() as i64;
        for file in files.iter() {
            // calculate scan size
            scan_stats.records += file.meta.records;
            scan_stats.original_size += file.meta.original_size;
            scan_stats.compressed_size += file.meta.compressed_size;
            // check schema version
            let schema_ver_id = match db::schema::filter_schema_version_id(
                &schema_versions,
                file.meta.min_ts,
                file.meta.max_ts,
            ) {
                Some(id) => id,
                None => {
                    log::error!(
                        "[trace_id {trace_id}] wal->parquet->search: file {} schema version not found, will use the latest schema, min_ts: {}, max_ts: {}",
                        &file.key,
                        file.meta.min_ts,
                        file.meta.max_ts
                    );
                    // HACK: use the latest version if not found in schema versions
                    schema_latest_id
                }
            };
            let group = files_group.entry(schema_ver_id).or_default();
            group.push(file.clone());
        }
    }

    log::info!(
        "[trace_id {trace_id}] wal->parquet->search: load groups {}, files {}, scan_size {}, compressed_size {}",
        files_group.len(),
        scan_stats.files,
        scan_stats.original_size,
        scan_stats.compressed_size
    );

    if cfg.common.memory_circuit_breaker_enable {
        if let Err(e) = super::check_memory_circuit_breaker(trace_id, &scan_stats) {
            // release all files
            wal::release_files(&lock_files).await;
            return Err(e);
        }
    }

    // construct latest schema map
    let mut schema_latest_map = HashMap::with_capacity(schema_latest.fields().len());
    for field in schema_latest.fields() {
        schema_latest_map.insert(field.name(), field);
    }
    let select_wildcard = RE_SELECT_WILDCARD.is_match(sql.origin_sql.as_str());

    let mut tasks = Vec::new();
    for (ver, files) in files_group {
        let schema = schema_versions[ver]
            .clone()
            .with_metadata(std::collections::HashMap::new());
        let sql = sql.clone();
        let session = config::meta::search::Session {
            id: format!("{trace_id}-wal-{ver}"),
            storage_type: StorageType::Wal,
            search_type: if !sql.meta.group_by.is_empty() {
                SearchType::Aggregation
            } else {
                SearchType::Normal
            },
            work_group: Some(work_group.to_string()),
            target_partitions: 0,
        };

        // cacluate the diff between latest schema and group schema
        let (schema, diff_fields) = if select_wildcard {
            generate_select_start_search_schema(
                &sql,
                &schema,
                &schema_latest_map,
                &defined_schema_fields,
            )?
        } else {
            generate_search_schema(&sql, &schema, &schema_latest_map)?
        };

        let datafusion_span = info_span!(
            "service:search:grpc:wal:parquet:datafusion",
            org_id = sql.org_id,
            stream_name = sql.stream_name,
            stream_type = stream_type.to_string(),
        );

        #[cfg(feature = "enterprise")]
        let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
        #[cfg(feature = "enterprise")]
        if crate::service::search::SEARCH_SERVER
            .insert_sender(trace_id, abort_sender)
            .await
            .is_err()
        {
            log::info!(
                "[trace_id {}] wal->parquet->search: search canceled before call wal->parquet->search",
                session.id
            );
            return Err(Error::Message(format!(
                "[trace_id {}] wal->parquet->search: search canceled before call wal->parquet->search",
                session.id
            )));
        }

        let task = tokio::task::spawn(
            async move {
                tokio::select! {
                    ret = exec::sql(
                        &session,
                        schema,
                        diff_fields,
                        &sql,
                        &files,
                        None,
                        FileType::PARQUET,
                    ) => ret,
                    _ = tokio::time::sleep(Duration::from_secs(timeout)) => {
                        log::error!("[trace_id {}] wal->parquet->search: search timeout", session.id);
                        Err(datafusion::error::DataFusionError::ResourcesExhausted(format!(
                            "[trace_id {}] wal->parquet->search: task timeout", session.id
                        )))
                    },
                    _ = async {
                        #[cfg(feature = "enterprise")]
                        let _ = abort_receiver.await;
                        #[cfg(not(feature = "enterprise"))]
                        futures::future::pending::<()>().await;
                    } => {
                        log::info!("[trace_id {}] wal->parquet->search: search canceled", session.id);
                        Err(datafusion::error::DataFusionError::Execution(format!(
                            "[trace_id {}] wal->parquet->search: task is cancel", session.id
                        )))
                    }
                }
            }
            .instrument(datafusion_span),
        );

        tasks.push(task);
    }

    let task_results = match try_join_all(tasks).await {
        Ok(v) => v,
        Err(e) => {
            // release all files
            wal::release_files(&lock_files).await;
            return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                e.to_string(),
            )));
        }
    };

    let mut results = vec![];
    for ret in task_results {
        match ret {
            Ok(v) => {
                results.extend(v);
            }
            Err(err) => {
                // release all files
                wal::release_files(&lock_files).await;
                log::error!(
                    "[trace_id {trace_id}] wal->parquet->search: datafusion execute error: {}",
                    err
                );
                match err {
                    datafusion::error::DataFusionError::ResourcesExhausted(e) => {
                        return Err(Error::ErrorCode(ErrorCodes::SearchTimeout(e)));
                    }
                    _ => return Err(err.into()),
                }
            }
        };
    }

    // release all files
    wal::release_files(&lock_files).await;

    Ok((results, scan_stats))
}

/// search in local WAL, which haven't been sync to object storage
#[tracing::instrument(name = "service:search:wal:memtable", skip_all, fields(org_id = sql.org_id, stream_name = sql.stream_name))]
pub async fn search_memtable(
    trace_id: &str,
    sql: Arc<Sql>,
    stream_type: StreamType,
    work_group: &str,
    timeout: u64,
) -> super::SearchResult {
    let schema_latest = infra::schema::get(&sql.org_id, &sql.stream_name, stream_type)
        .await
        .unwrap_or(Schema::empty());
    let stream_settings = unwrap_stream_settings(&schema_latest).unwrap_or_default();
    let defined_schema_fields = stream_settings.defined_schema_fields.unwrap_or_default();

    let mut scan_stats = ScanStats::new();

    let mut batches = ingester::read_from_memtable(
        &sql.org_id,
        &stream_type.to_string(),
        &sql.stream_name,
        sql.meta.time_range,
    )
    .await
    .unwrap_or_default();
    batches.extend(
        ingester::read_from_immutable(
            &sql.org_id,
            &stream_type.to_string(),
            &sql.stream_name,
            sql.meta.time_range,
        )
        .await
        .unwrap_or_default(),
    );
    scan_stats.files = batches.iter().map(|(_, k)| k.len()).sum::<usize>() as i64;
    if scan_stats.files == 0 {
        return Ok((vec![], ScanStats::new()));
    }

    let mut batch_groups: HashMap<Arc<Schema>, Vec<RecordBatch>> = HashMap::with_capacity(2);
    for (schema, batch) in batches {
        let entry = batch_groups.entry(schema).or_default();
        for r in batch.iter() {
            scan_stats.records += r.data.num_rows() as i64;
            scan_stats.original_size += r.data_json_size as i64;
            scan_stats.compressed_size += r.data_arrow_size as i64;
        }
        entry.extend(batch.into_iter().map(|r| r.data.clone()));
    }

    log::info!(
        "[trace_id {trace_id}] wal->mem->search: load groups {}, files {}, scan_size {}, compressed_size {}",
        batch_groups.len(),
        scan_stats.files,
        scan_stats.original_size,
        scan_stats.compressed_size,
    );

    let cfg = get_config();
    if cfg.common.memory_circuit_breaker_enable {
        super::check_memory_circuit_breaker(trace_id, &scan_stats)?;
    }

    // fetch all schema versions, get latest schema
    let schema_latest = Arc::new(
        schema_latest
            .to_owned()
            .with_metadata(std::collections::HashMap::new()),
    );

    // construct latest schema map
    let mut schema_latest_map = HashMap::with_capacity(schema_latest.fields().len());
    for field in schema_latest.fields() {
        schema_latest_map.insert(field.name(), field);
    }
    let select_wildcard = RE_SELECT_WILDCARD.is_match(sql.origin_sql.as_str());

    let mut tasks = Vec::new();
    for (ver, (schema, mut record_batches)) in batch_groups.into_iter().enumerate() {
        let sql = sql.clone();
        let session = config::meta::search::Session {
            id: format!("{trace_id}-mem-{ver}"),
            storage_type: StorageType::Tmpfs,
            search_type: if !sql.meta.group_by.is_empty() {
                SearchType::Aggregation
            } else {
                SearchType::Normal
            },
            work_group: Some(work_group.to_string()),
            target_partitions: 0,
        };

        // cacluate the diff between latest schema and group schema
        let (schema, diff_fields) = if select_wildcard {
            generate_select_start_search_schema(
                &sql,
                &schema,
                &schema_latest_map,
                &defined_schema_fields,
            )?
        } else {
            generate_search_schema(&sql, &schema, &schema_latest_map)?
        };

        for batch in record_batches.iter_mut() {
            *batch = adapt_batch(&schema, batch);
        }

        let datafusion_span = info_span!(
            "service:search:grpc:wal:mem:datafusion",
            org_id = sql.org_id,
            stream_name = sql.stream_name,
            stream_type = stream_type.to_string(),
        );

        #[cfg(feature = "enterprise")]
        let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
        #[cfg(feature = "enterprise")]
        if crate::service::search::SEARCH_SERVER
            .insert_sender(trace_id, abort_sender)
            .await
            .is_err()
        {
            log::info!(
                "[trace_id {}] wal->mem->search: search canceled before call wal->mem->search",
                session.id
            );
            return Err(Error::Message(format!(
                "[trace_id {}] wal->mem->search: search canceled before call wal->mem->search",
                session.id
            )));
        }

        let task = tokio::task::spawn(
            async move {
                let files = vec![];
                tokio::select! {
                    ret = exec::sql(
                        &session,
                        schema,
                        diff_fields,
                        &sql,
                        &files,
                        Some(record_batches),
                        FileType::ARROW,
                    ) => ret,
                    _ = tokio::time::sleep(Duration::from_secs(timeout)) => {
                        log::error!("[trace_id {}] wal->mem->search: search timeout", session.id);
                        Err(datafusion::error::DataFusionError::ResourcesExhausted(format!(
                            "[trace_id {}] wal->mem->search: task timeout", session.id
                        )))
                    },
                    _ = async {
                        #[cfg(feature = "enterprise")]
                        let _ = abort_receiver.await;
                        #[cfg(not(feature = "enterprise"))]
                        futures::future::pending::<()>().await;
                    } => {
                        log::info!("[trace_id {}] wal->mem->search: search canceled", session.id);
                        Err(datafusion::error::DataFusionError::Execution(format!(
                            "[trace_id {}] wal->mem->search: task is cancel", session.id
                        )))
                    }
                }
            }
            .instrument(datafusion_span),
        );

        tasks.push(task);
    }

    let mut results = vec![];
    let task_results = try_join_all(tasks)
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?;
    for ret in task_results {
        match ret {
            Ok(v) => results.extend(v),
            Err(err) => {
                log::error!(
                    "[trace_id {trace_id}] wal->mem->search: datafusion execute error: {}",
                    err
                );
                match err {
                    datafusion::error::DataFusionError::ResourcesExhausted(e) => {
                        return Err(Error::ErrorCode(ErrorCodes::SearchTimeout(e)));
                    }
                    _ => return Err(err.into()),
                }
            }
        };
    }

    Ok((results, scan_stats))
}

#[tracing::instrument(name = "service:search:grpc:wal:get_file_list_inner", skip_all, fields(org_id = sql.org_id, stream_name = sql.stream_name))]
async fn get_file_list_inner(
    trace_id: &str,
    sql: &Sql,
    stream_type: StreamType,
    _partition_time_level: &PartitionTimeLevel,
    partition_keys: &[StreamPartition],
    wal_dir: &str,
    file_ext: &str,
) -> Result<Vec<FileKey>, Error> {
    let wal_dir = match Path::new(wal_dir).canonicalize() {
        Ok(path) => {
            let mut path = path.to_str().unwrap().to_string();
            // Hack for windows
            if path.starts_with("\\\\?\\") {
                path = path[4..].to_string();
                path = path.replace('\\', "/");
            }
            path
        }
        Err(_) => {
            return Ok(vec![]);
        }
    };

    // get all files
    let pattern = format!(
        "{}/files/{}/{stream_type}/{}/",
        wal_dir, &sql.org_id, &sql.stream_name
    );
    let files = scan_files(&pattern, file_ext, None).unwrap_or_default();
    if files.is_empty() {
        return Ok(vec![]);
    }

    // lock theses files
    let files = files
        .iter()
        .map(|f| {
            f.strip_prefix(&wal_dir)
                .unwrap()
                .to_string()
                .replace('\\', "/")
                .trim_start_matches('/')
                .to_string()
        })
        .collect::<Vec<_>>();
    wal::lock_files(&files).await;

    let mut result = Vec::with_capacity(files.len());
    let (min_ts, max_ts) = sql.meta.time_range.unwrap_or((0, 0));
    for file in files.iter() {
        let file_key = FileKey::from_file_name(file);
        if (min_ts, max_ts) != (0, 0) {
            let (file_min_ts, file_max_ts) = parse_time_range_from_filename(file);
            if (file_min_ts > 0 && file_max_ts > 0)
                && ((max_ts > 0 && file_min_ts > max_ts) || (min_ts > 0 && file_max_ts < min_ts))
            {
                log::debug!(
                    "[trace_id {trace_id}] skip wal parquet file: {} time_range: [{},{}]",
                    &file,
                    file_min_ts,
                    file_max_ts
                );
                wal::release_files(&[file.clone()]).await;
                continue;
            }
        }
        if sql
            .match_source(&file_key, false, true, stream_type, partition_keys)
            .await
        {
            result.push(file_key);
        } else {
            wal::release_files(&[file.clone()]).await;
        }
    }
    Ok(result)
}

/// get file list from local wal, no need match_source, each file will be
/// searched
#[tracing::instrument(name = "service:search:grpc:wal:get_file_list", skip_all, fields(org_id = sql.org_id, stream_name = sql.stream_name))]
async fn get_file_list(
    trace_id: &str,
    sql: &Sql,
    stream_type: StreamType,
    _partition_time_level: &PartitionTimeLevel,
    partition_keys: &[StreamPartition],
) -> Result<Vec<FileKey>, Error> {
    get_file_list_inner(
        trace_id,
        sql,
        stream_type,
        _partition_time_level,
        partition_keys,
        &get_config().common.data_wal_dir,
        "parquet",
    )
    .await
}

pub fn adapt_batch(table_schema: &Schema, batch: &RecordBatch) -> RecordBatch {
    let batch_schema = &*batch.schema();
    let batch_cols = batch.columns().to_vec();

    let mut cols: Vec<ArrayRef> = Vec::with_capacity(table_schema.fields().len());
    for table_field in table_schema.fields() {
        if let Some((batch_idx, _)) = batch_schema.column_with_name(table_field.name().as_str()) {
            cols.push(Arc::clone(&batch_cols[batch_idx]));
        } else {
            cols.push(new_null_array(table_field.data_type(), batch.num_rows()))
        }
    }

    let merged_schema = Arc::new(table_schema.clone());
    RecordBatch::try_new(merged_schema, cols).unwrap()
}
