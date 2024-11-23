// Copyright 2024 OpenObserve Inc.
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

use ::datafusion::{arrow::datatypes::Schema, error::DataFusionError};
use arrow::array::RecordBatch;
use arrow_schema::{DataType, Field};
use bytes::Bytes;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use config::{
    cluster::LOCAL_NODE,
    get_config, ider, is_local_disk_storage,
    meta::{
        inverted_index::InvertedIndexFormat,
        search::StorageType,
        stream::{
            FileKey, FileListDeleted, FileMeta, MergeStrategy, PartitionTimeLevel, StreamStats,
            StreamType,
        },
    },
    metrics,
    utils::{
        parquet::{get_recordbatch_reader_from_bytes, read_schema_from_bytes},
        record_batch_ext::concat_batches,
        schema_ext::SchemaExt,
        time::hour_micros,
    },
    FILE_EXT_PARQUET,
};
use hashbrown::{HashMap, HashSet};
use infra::{
    cache::file_data,
    dist_lock, file_list as infra_file_list,
    schema::{
        get_stream_setting_bloom_filter_fields, get_stream_setting_fts_fields,
        get_stream_setting_index_fields, unwrap_partition_time_level, unwrap_stream_settings,
        SchemaCache,
    },
    storage,
};
use tokio::{
    sync::{mpsc, Semaphore},
    task::JoinHandle,
};

use crate::{
    common::infra::cluster::get_node_by_uuid,
    job::files::parquet::{create_tantivy_index, generate_index_on_compactor},
    service::{
        db, file_list, schema::generate_schema_for_defined_schema_fields, search::datafusion::exec,
        stream,
    },
};

#[derive(Clone)]
pub struct MergeBatch {
    pub batch_id: usize,
    pub org_id: String,
    pub stream_type: StreamType,
    pub stream_name: String,
    pub prefix: String,
    pub files: Vec<FileKey>,
}

pub struct MergeResult {
    pub batch_id: usize,
    pub new_file: FileKey,
}

pub type MergeSender = mpsc::Sender<Result<(usize, FileKey), anyhow::Error>>;

/// Generate merging job by stream
/// 1. get offset from db
/// 2. check if other node is processing
/// 3. create job or return
pub async fn generate_job_by_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    // get last compacted offset
    let (mut offset, node) = db::compact::files::get_offset(org_id, stream_type, stream_name).await;
    if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        return Ok(()); // other node is processing
    }

    if node.is_empty() || LOCAL_NODE.uuid.ne(&node) {
        let lock_key = format!("/compact/merge/{}/{}/{}", org_id, stream_type, stream_name);
        let locker = dist_lock::lock(&lock_key, 0, None).await?;
        // check the working node again, maybe other node locked it first
        let (offset, node) = db::compact::files::get_offset(org_id, stream_type, stream_name).await;
        if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some()
        {
            dist_lock::unlock(&locker).await?;
            return Ok(()); // other node is processing
        }
        // set to current node
        let ret = db::compact::files::set_offset(
            org_id,
            stream_type,
            stream_name,
            offset,
            Some(&LOCAL_NODE.uuid.clone()),
        )
        .await;
        dist_lock::unlock(&locker).await?;
        drop(locker);
        ret?;
    }

    // get schema
    let schema = infra::schema::get(org_id, stream_name, stream_type).await?;
    let stream_created = stream::stream_created(&schema).unwrap_or_default();
    if offset == 0 {
        offset = stream_created
    }
    if offset == 0 {
        return Ok(()); // no data
    }

    let cfg = get_config();
    // check offset
    let time_now: DateTime<Utc> = Utc::now();
    let time_now_hour = Utc
        .with_ymd_and_hms(
            time_now.year(),
            time_now.month(),
            time_now.day(),
            time_now.hour(),
            0,
            0,
        )
        .unwrap()
        .timestamp_micros();
    // must wait for at least 3 * max_file_retention_time
    // -- first period: the last hour local file upload to storage, write file list
    // -- second period, the last hour file list upload to storage
    // -- third period, we can do the merge, so, at least 3 times of
    // max_file_retention_time
    if offset >= time_now_hour
        || time_now.timestamp_micros() - offset
            <= Duration::try_seconds(cfg.limit.max_file_retention_time as i64)
                .unwrap()
                .num_microseconds()
                .unwrap()
                * 3
    {
        return Ok(()); // the time is future, just wait
    }

    log::debug!(
        "[COMPACTOR] generate_job_by_stream [{}/{}/{}] offset: {}",
        org_id,
        stream_type,
        stream_name,
        offset
    );

    // generate merging job
    if let Err(e) = infra_file_list::add_job(org_id, stream_type, stream_name, offset).await {
        return Err(anyhow::anyhow!(
            "[COMAPCT] add file_list_jobs failed: {}",
            e
        ));
    }

    // write new offset
    let offset = offset + hour_micros(1);
    // format to hour with zero minutes, seconds
    let offset = offset - offset % hour_micros(1);
    db::compact::files::set_offset(
        org_id,
        stream_type,
        stream_name,
        offset,
        Some(&LOCAL_NODE.uuid.clone()),
    )
    .await?;

    Ok(())
}

/// Generate merging job by stream
/// 1. get old data by hour
/// 2. check if other node is processing
/// 3. create job or return
pub async fn generate_old_data_job_by_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    // get last compacted offset
    let (offset, node) = db::compact::files::get_offset(org_id, stream_type, stream_name).await;
    if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        return Ok(()); // other node is processing
    }

    if node.is_empty() || LOCAL_NODE.uuid.ne(&node) {
        let lock_key = format!("/compact/merge/{}/{}/{}", org_id, stream_type, stream_name);
        let locker = dist_lock::lock(&lock_key, 0, None).await?;
        // check the working node again, maybe other node locked it first
        let (offset, node) = db::compact::files::get_offset(org_id, stream_type, stream_name).await;
        if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some()
        {
            dist_lock::unlock(&locker).await?;
            return Ok(()); // other node is processing
        }
        // set to current node
        let ret = db::compact::files::set_offset(
            org_id,
            stream_type,
            stream_name,
            offset,
            Some(&LOCAL_NODE.uuid.clone()),
        )
        .await;
        dist_lock::unlock(&locker).await?;
        drop(locker);
        ret?;
    }

    if offset == 0 {
        return Ok(()); // no data
    }

    let cfg = get_config();
    let stream_settings = infra::schema::get_settings(org_id, stream_name, stream_type)
        .await
        .unwrap_or_default();
    let mut stream_data_retention_days = cfg.compact.data_retention_days;
    if stream_settings.data_retention > 0 {
        stream_data_retention_days = stream_settings.data_retention;
    }
    if stream_data_retention_days > cfg.compact.old_data_max_days {
        stream_data_retention_days = cfg.compact.old_data_max_days;
    }
    if stream_data_retention_days == 0 {
        return Ok(()); // no need to check old data
    }

    // get old data by hour, `offset - 2 hours` as old data
    let end_time = offset - hour_micros(2);
    let start_time = end_time
        - Duration::try_days(stream_data_retention_days as i64)
            .unwrap()
            .num_microseconds()
            .unwrap();
    let hours = infra_file_list::query_old_data_hours(
        org_id,
        stream_type,
        stream_name,
        Some((start_time, end_time)),
    )
    .await?;

    // generate merging job
    for hour in hours {
        let column = hour.split('/').collect::<Vec<_>>();
        if column.len() != 4 {
            return Err(anyhow::anyhow!(
                "Unexpected hour format in {}, Expected format YYYY/MM/DD/HH",
                hour
            ));
        }
        let offset = DateTime::parse_from_rfc3339(&format!(
            "{}-{}-{}T{}:00:00Z",
            column[0], column[1], column[2], column[3]
        ))?
        .with_timezone(&Utc);
        let offset = offset.timestamp_micros();
        log::debug!(
            "[COMPACTOR] generate_old_data_job_by_stream [{}/{}/{}] hours: {}, offset: {}",
            org_id,
            stream_type,
            stream_name,
            hour,
            offset
        );
        if let Err(e) = infra_file_list::add_job(org_id, stream_type, stream_name, offset).await {
            return Err(anyhow::anyhow!(
                "[COMAPCT] add file_list_jobs for old data failed: {}",
                e
            ));
        }
    }

    Ok(())
}

/// compactor run steps on a stream:
/// 3. get a cluster lock for compactor stream
/// 4. read last compacted offset: year/month/day/hour
/// 5. read current hour all files
/// 6. compact small files to big files -> COMPACTOR_MAX_FILE_SIZE
/// 7. write to storage
/// 8. delete small files keys & write big files keys, use transaction
/// 9. delete small files from storage
/// 10. update last compacted offset
/// 11. release cluster lock
pub async fn merge_by_stream(
    worker_tx: mpsc::Sender<(MergeSender, MergeBatch)>,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    job_id: i64,
    offset: i64,
) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();

    // get schema
    let schema = infra::schema::get(org_id, stream_name, stream_type).await?;
    let stream_settings = unwrap_stream_settings(&schema).unwrap_or_default();
    let partition_time_level =
        unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);

    log::debug!(
        "[COMPACTOR] merge_by_stream [{}/{}/{}] offset: {}",
        org_id,
        stream_type,
        stream_name,
        offset
    );

    let offset_time: DateTime<Utc> = Utc.timestamp_nanos(offset * 1000);
    let offset_time_hour = Utc
        .with_ymd_and_hms(
            offset_time.year(),
            offset_time.month(),
            offset_time.day(),
            offset_time.hour(),
            0,
            0,
        )
        .unwrap()
        .timestamp_micros();
    let offset_time_day = Utc
        .with_ymd_and_hms(
            offset_time.year(),
            offset_time.month(),
            offset_time.day(),
            0,
            0,
            0,
        )
        .unwrap()
        .timestamp_micros();

    let cfg = get_config();
    // check offset
    let time_now: DateTime<Utc> = Utc::now();
    let time_now_hour = Utc
        .with_ymd_and_hms(
            time_now.year(),
            time_now.month(),
            time_now.day(),
            time_now.hour(),
            0,
            0,
        )
        .unwrap()
        .timestamp_micros();

    // get current hour(day) all files
    let (partition_offset_start, partition_offset_end) =
        if partition_time_level == PartitionTimeLevel::Daily {
            (offset_time_day, offset_time_day + hour_micros(24) - 1)
        } else {
            (offset_time_hour, offset_time_hour + hour_micros(1) - 1)
        };
    let files = file_list::query(
        org_id,
        stream_name,
        stream_type,
        partition_time_level,
        partition_offset_start,
        partition_offset_end,
    )
    .await
    .map_err(|e| anyhow::anyhow!("query file list failed: {}", e))?;

    log::debug!(
        "[COMPACTOR] merge_by_stream [{}/{}/{}] time range: [{},{}], files: {}",
        org_id,
        stream_type,
        stream_name,
        partition_offset_start,
        partition_offset_end,
        files.len(),
    );

    if files.is_empty() {
        // update job status
        if let Err(e) = infra_file_list::set_job_done(job_id).await {
            log::error!("[COMPACT] set_job_done failed: {e}");
        }
        return Ok(());
    }

    // do partition by partition key
    let mut partition_files_with_size: HashMap<String, Vec<FileKey>> = HashMap::default();
    for file in files {
        let file_name = file.key.clone();
        let prefix = file_name[..file_name.rfind('/').unwrap()].to_string();
        let partition = partition_files_with_size.entry(prefix).or_default();
        partition.push(file.to_owned());
    }

    // use multiple threads to merge
    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.file_merge_thread_num));
    let mut tasks = Vec::with_capacity(partition_files_with_size.len());
    for (prefix, files_with_size) in partition_files_with_size.into_iter() {
        let org_id = org_id.to_string();
        let stream_name = stream_name.to_string();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let worker_tx = worker_tx.clone();
        let task: JoinHandle<Result<(), anyhow::Error>> = tokio::task::spawn(async move {
            let cfg = get_config();
            // sort by file size
            let mut files_with_size = files_with_size.to_owned();
            let job_strategy = MergeStrategy::from(&cfg.compact.strategy);
            match job_strategy {
                MergeStrategy::FileSize => {
                    files_with_size.sort_by(|a, b| a.meta.original_size.cmp(&b.meta.original_size));
                }
                MergeStrategy::FileTime => {
                    files_with_size.sort_by(|a, b| a.meta.min_ts.cmp(&b.meta.min_ts));
                }
            }
            // delete duplicated files
            files_with_size.dedup_by(|a, b| a.key == b.key);
            // partition files by size
            if files_with_size.len() <= 1 {
                return Ok(());
            }

            // group files need to merge
            let mut batch_groups = Vec::new();
            let mut new_file_list = Vec::new();
            let mut new_file_size = 0;
            for file in files_with_size.iter() {
                if new_file_size + file.meta.original_size > cfg.compact.max_file_size as i64 {
                    if new_file_list.len() <= 1 {
                        if job_strategy == MergeStrategy::FileSize {
                            break;
                        }
                        new_file_size = 0;
                        new_file_list.clear();
                        continue; // this batch don't need to merge, skip
                    }
                    batch_groups.push(MergeBatch {
                        batch_id: batch_groups.len(),
                        org_id: org_id.clone(),
                        stream_type,
                        stream_name: stream_name.clone(),
                        prefix: prefix.clone(),
                        files: new_file_list.clone(),
                    });
                    new_file_size = 0;
                    new_file_list.clear();
                }
                new_file_size += file.meta.original_size;
                new_file_list.push(file.clone());
            }
            if new_file_list.len() > 1 {
                batch_groups.push(MergeBatch {
                    batch_id: batch_groups.len(),
                    org_id: org_id.clone(),
                    stream_type,
                    stream_name: stream_name.clone(),
                    prefix: prefix.clone(),
                    files: new_file_list.clone(),
                });
            }

            if batch_groups.is_empty() {
                return Ok(()); // no files need to merge
            }

            // send to worker
            let batch_group_len = batch_groups.len();
            let (inner_tx, mut inner_rx) = mpsc::channel(batch_group_len);
            for batch in batch_groups.iter() {
                if let Err(e) = worker_tx.send((inner_tx.clone(), batch.clone())).await {
                    log::error!("[COMPACT] send batch to worker failed: {}", e);
                    return Err(anyhow::Error::msg("send batch to worker failed"));
                }
            }
            let mut worker_results = Vec::with_capacity(batch_group_len);
            for _ in 0..batch_group_len {
                let result = inner_rx.recv().await.unwrap();
                worker_results.push(result);
            }

            let mut last_error = None;
            for ret in worker_results {
                let (batch_id, mut new_file) = match ret {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("[COMPACT] merge files failed: {}", e);
                        last_error = Some(e);
                        continue;
                    }
                };
                let new_file_name = std::mem::take(&mut new_file.key);
                let new_file_meta = std::mem::take(&mut new_file.meta);
                let new_file_list = batch_groups.get(batch_id).unwrap().files.as_slice();
                if new_file_name.is_empty() {
                    continue;
                }

                // delete small files keys & write big files keys, use transaction
                let mut events = Vec::with_capacity(new_file_list.len() + 1);
                events.push(FileKey {
                    key: new_file_name,
                    meta: new_file_meta,
                    deleted: false,
                    segment_ids: None,
                });

                // collect stream stats
                let mut stream_stats: StreamStats = StreamStats::default();
                for file in new_file_list.iter() {
                    stream_stats = stream_stats - file.meta.clone();
                    events.push(FileKey {
                        key: file.key.clone(),
                        meta: file.meta.clone(),
                        deleted: true,
                        segment_ids: None,
                    });
                }
                events.sort_by(|a, b| a.key.cmp(&b.key));

                // write file list to storage
                match write_file_list(&org_id, &events).await {
                    Ok(_) => {
                        // update stream stats
                        if stream_stats.doc_num != 0 {
                            let stream_key = format!("{org_id}/{stream_type}/{stream_name}");
                            if let Err(e) = infra_file_list::set_stream_stats(
                                &org_id,
                                &[(stream_key.clone(), stream_stats)],
                            )
                            .await
                            {
                                log::error!(
                                    "[COMPACT] set_stream_stats failed: {}, err: {}",
                                    stream_key,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[COMPACT] write file list failed: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
            drop(permit);
            if let Some(e) = last_error {
                return Err(e);
            }
            Ok(())
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await??;
    }

    // update job status
    if let Err(e) = infra_file_list::set_job_done(job_id).await {
        log::error!("[COMPACT] set_job_done failed: {e}");
    }

    // metrics
    let time = start.elapsed().as_secs_f64();
    metrics::COMPACT_USED_TIME
        .with_label_values(&[org_id, stream_type.to_string().as_str()])
        .inc_by(time);
    metrics::COMPACT_DELAY_HOURS
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .set((time_now_hour - offset_time_hour) / hour_micros(1));

    Ok(())
}

/// merge small files into big file, upload to storage, returns the big file key and merged files
pub async fn merge_files(
    thread_id: usize,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    prefix: &str,
    files_with_size: &[FileKey],
) -> Result<(String, FileMeta, Vec<FileKey>), anyhow::Error> {
    if files_with_size.len() <= 1 {
        return Ok((String::from(""), FileMeta::default(), Vec::new()));
    }

    let mut new_file_size = 0;
    let mut new_compressed_file_size = 0;
    let mut new_file_list = Vec::new();
    let cfg = get_config();
    for file in files_with_size.iter() {
        if new_file_size + file.meta.original_size > cfg.compact.max_file_size as i64
            || new_compressed_file_size + file.meta.compressed_size
                > cfg.compact.max_file_size as i64
        {
            break;
        }
        new_file_size += file.meta.original_size;
        new_compressed_file_size += file.meta.compressed_size;
        new_file_list.push(file.clone());
        // metrics
        metrics::COMPACT_MERGED_FILES
            .with_label_values(&[org_id, stream_type.to_string().as_str()])
            .inc();
        metrics::COMPACT_MERGED_BYTES
            .with_label_values(&[org_id, stream_type.to_string().as_str()])
            .inc_by(file.meta.original_size as u64);
    }
    // no files need to merge
    if new_file_list.len() <= 1 {
        return Ok((String::from(""), FileMeta::default(), Vec::new()));
    }

    let retain_file_list = new_file_list.clone();

    // cache parquet files
    let deleted_files = cache_remote_files(&new_file_list).await?;
    if !deleted_files.is_empty() {
        new_file_list.retain(|f| !deleted_files.contains(&f.key));
    }
    if new_file_list.len() <= 1 {
        return Ok((String::from(""), FileMeta::default(), retain_file_list));
    }

    // get time range for these files
    let min_ts = new_file_list.iter().map(|f| f.meta.min_ts).min().unwrap();
    let max_ts = new_file_list.iter().map(|f| f.meta.max_ts).max().unwrap();
    let total_records = new_file_list.iter().map(|f| f.meta.records).sum();
    let new_file_size = new_file_list.iter().map(|f| f.meta.original_size).sum();
    let mut new_file_meta = FileMeta {
        min_ts,
        max_ts,
        records: total_records,
        original_size: new_file_size,
        compressed_size: 0,
        flattened: false,
        index_size: 0,
    };
    if new_file_meta.records == 0 {
        return Err(anyhow::anyhow!("merge_files error: records is 0"));
    }

    // get latest version of schema
    let schema_latest = infra::schema::get(org_id, stream_name, stream_type).await?;
    let stream_settings = infra::schema::get_settings(org_id, stream_name, stream_type).await;
    let bloom_filter_fields = get_stream_setting_bloom_filter_fields(&stream_settings);
    let full_text_search_fields = get_stream_setting_fts_fields(&stream_settings);
    let index_fields = get_stream_setting_index_fields(&stream_settings);
    let (defined_schema_fields, need_original) = match stream_settings {
        Some(s) => (
            s.defined_schema_fields.unwrap_or_default(),
            s.store_original_data,
        ),
        None => (Vec::new(), false),
    };
    let schema_latest = if !defined_schema_fields.is_empty() {
        let schema_latest = SchemaCache::new(schema_latest);
        let schema_latest = generate_schema_for_defined_schema_fields(
            &schema_latest,
            &defined_schema_fields,
            need_original,
        );
        schema_latest.schema().as_ref().clone()
    } else {
        schema_latest
    };

    // read schema from parquet file and group files by schema
    let mut schemas = HashMap::new();
    let mut file_groups = HashMap::new();
    let mut fi = 0;
    for file in new_file_list.iter() {
        fi += 1;
        log::info!("[COMPACT:{thread_id}:{fi}] merge small file: {}", &file.key);
        let buf = file_data::get("", &file.key, None).await?;
        let schema = read_schema_from_bytes(&buf).await?;
        let schema = schema.as_ref().clone().with_metadata(Default::default());
        let schema_key = schema.hash_key();
        if !schemas.contains_key(&schema_key) {
            schemas.insert(schema_key.clone(), schema);
            file_groups.insert(schema_key.clone(), vec![]);
        }
        let entry = file_groups.get_mut(&schema_key).unwrap();
        entry.push(file.clone());
    }

    // generate the final schema
    let all_fields = schemas
        .values()
        .flat_map(|s| s.fields().iter().map(|f| f.name().to_string()))
        .collect::<HashSet<_>>();
    let schema_latest = Arc::new(schema_latest.retain(all_fields));
    let mut schema_latest_fields = HashMap::with_capacity(schema_latest.fields().len());
    for field in schema_latest.fields() {
        schema_latest_fields.insert(field.name(), field);
    }

    // generate datafusion tables
    let mut tables = Vec::new();
    let trace_id = ider::generate();
    for (schema_key, files) in file_groups {
        if files.is_empty() {
            continue;
        }
        let schema = schemas.get(&schema_key).unwrap().clone();
        let session = config::meta::search::Session {
            id: format!("{trace_id}-{schema_key}"),
            storage_type: StorageType::Memory,
            work_group: None,
            target_partitions: 0,
        };

        let diff_fields = generate_schema_diff(&schema, &schema_latest_fields)?;
        let table = match exec::create_parquet_table(
            &session,
            schema_latest.clone(),
            &files,
            diff_fields,
            true,
            None,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "create_parquet_table err: {}, files: {:?}, schema: {:?}",
                    e,
                    files,
                    schema
                );
                return Err(DataFusionError::Plan(format!("create_parquet_table err: {e}")).into());
            }
        };
        tables.push(table);
    }

    let start = std::time::Instant::now();
    let merge_result = exec::merge_parquet_files(
        stream_type,
        stream_name,
        schema_latest.clone(),
        tables,
        &bloom_filter_fields,
        &new_file_meta,
    )
    .await;

    // clear session data
    crate::service::search::datafusion::storage::file_list::clear(&trace_id);
    // clear cached data
    let files = new_file_list.into_iter().map(|f| f.key).collect::<Vec<_>>();
    for file in files.iter() {
        let _ = file_data::disk::remove("", file).await;
    }

    let (_new_schema, buf) = match merge_result {
        Ok(v) => v,
        Err(e) => {
            log::error!(
                "merge_parquet_files err: {}, files: {:?}, schema: {:?}",
                e,
                files,
                schema_latest
            );
            return Err(DataFusionError::Plan(format!("merge_parquet_files err: {e}",)).into());
        }
    };

    new_file_meta.compressed_size = buf.len() as i64;
    if new_file_meta.compressed_size == 0 {
        return Err(anyhow::anyhow!(
            "merge_parquet_files error: compressed_size is 0"
        ));
    }

    let id = ider::generate();
    let new_file_key = format!("{prefix}/{id}{}", FILE_EXT_PARQUET);
    log::info!(
        "[COMPACT:{thread_id}] merge file successfully, {} files into a new file: {}, original_size: {}, compressed_size: {}, took: {} ms",
        retain_file_list.len(),
        new_file_key,
        new_file_meta.original_size,
        new_file_meta.compressed_size,
        start.elapsed().as_millis(),
    );

    // upload file to storage
    let buf = Bytes::from(buf);
    storage::put(&new_file_key, buf.clone()).await?;

    if !cfg.common.inverted_index_enabled || !stream_type.is_basic_type() {
        return Ok((new_file_key, new_file_meta, retain_file_list));
    }

    // generate parquet format inverted index
    let index_format = InvertedIndexFormat::from(&cfg.common.inverted_index_store_format);
    if matches!(
        index_format,
        InvertedIndexFormat::Parquet | InvertedIndexFormat::Both
    ) {
        let (schema, mut reader) = get_recordbatch_reader_from_bytes(&buf).await?;
        let files = generate_index_on_compactor(
            &retain_file_list,
            &new_file_key,
            org_id,
            stream_type,
            stream_name,
            &full_text_search_fields,
            &index_fields,
            schema,
            &mut reader,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "generate_index_on_compactor for file: {}, error: {}, need delete files: {:?}",
                new_file_key,
                e,
                retain_file_list
            )
        })?;
        for (file_name, filemeta) in files {
            if file_name.is_empty() {
                continue;
            }
            log::info!(
                "created parquet index file during compaction: {}",
                file_name
            );
            // Notify that we wrote the index file to the db.
            if let Err(e) = write_file_list(
                org_id,
                &[FileKey {
                    key: file_name.clone(),
                    meta: filemeta,
                    deleted: false,
                    segment_ids: None,
                }],
            )
            .await
            {
                log::error!(
                    "generate_index_on_compactor write to file list: {}, error: {}, need delete files: {:?}",
                    file_name,
                    e.to_string(),
                    retain_file_list
                );
            }
        }
    }

    if matches!(
        index_format,
        InvertedIndexFormat::Tantivy | InvertedIndexFormat::Both
    ) {
        let (schema, mut reader) = get_recordbatch_reader_from_bytes(&buf).await?;
        let index_size =  create_tantivy_index(
            "COMPACTOR",
            &new_file_key,
            &full_text_search_fields,
            &index_fields,
            schema,
            &mut reader,
        )
        .await.map_err(|e| {
            anyhow::anyhow!(
                "create_tantivy_index_on_compactor for file: {}, error: {}, need delete files: {:?}",
                new_file_key,
                e,
                retain_file_list
            )
        })?;
        new_file_meta.index_size = index_size as i64;
    }

    Ok((new_file_key, new_file_meta, retain_file_list))
}

async fn write_file_list(org_id: &str, events: &[FileKey]) -> Result<(), anyhow::Error> {
    if events.is_empty() {
        return Ok(());
    }

    let put_items = events
        .iter()
        .filter(|v| !v.deleted)
        .map(|v| v.to_owned())
        .collect::<Vec<_>>();
    let del_items = events
        .iter()
        .filter(|v| v.deleted)
        .map(|v| FileListDeleted {
            file: v.key.clone(),
            index_file: v.meta.index_size > 0,
            flattened: v.meta.flattened,
        })
        .collect::<Vec<_>>();
    // set to external db
    // retry 5 times
    let mut success = false;
    let created_at = config::utils::time::now_micros();
    for _ in 0..5 {
        if !del_items.is_empty() {
            if let Err(e) = infra_file_list::batch_add_deleted(org_id, created_at, &del_items).await
            {
                log::error!(
                    "[COMPACT] batch_add_deleted to external db failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        }
        if let Err(e) = infra_file_list::batch_add(&put_items).await {
            log::error!("[COMPACT] batch_add to external db failed, retrying: {}", e);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
        if !del_items.is_empty() {
            let del_files = del_items.iter().map(|v| v.file.clone()).collect::<Vec<_>>();
            if let Err(e) = infra_file_list::batch_remove(&del_files).await {
                log::error!(
                    "[COMPACT] batch_delete to external db failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        }
        // send broadcast to other nodes
        if get_config().memory_cache.cache_latest_files {
            if let Err(e) = db::file_list::broadcast::send(events, None).await {
                log::error!("[COMPACT] send broadcast for file_list failed: {}", e);
            }
        }
        // broadcast success
        success = true;
        break;
    }
    if !success {
        Err(anyhow::anyhow!("batch_write to external db failed"))
    } else {
        Ok(())
    }
}

pub fn generate_inverted_idx_recordbatch(
    schema: Arc<Schema>,
    batches: &[RecordBatch],
    stream_type: StreamType,
    full_text_search_fields: &[String],
    index_fields: &[String],
) -> Result<Option<RecordBatch>, anyhow::Error> {
    let cfg = get_config();
    if !cfg.common.inverted_index_enabled || batches.is_empty() || !stream_type.is_basic_type() {
        return Ok(None);
    }

    let schema_fields = schema
        .fields()
        .iter()
        .map(|f| f.name())
        .collect::<HashSet<_>>();

    let mut inverted_idx_columns = if !full_text_search_fields.is_empty() {
        full_text_search_fields.to_vec()
    } else {
        config::SQL_FULL_TEXT_SEARCH_FIELDS.to_vec()
    };
    inverted_idx_columns.extend(index_fields.to_vec());
    inverted_idx_columns.sort();
    inverted_idx_columns.dedup();
    inverted_idx_columns.retain(|f| schema_fields.contains(f));
    if inverted_idx_columns.is_empty() {
        return Ok(None);
    }
    // add _timestamp column to columns_to_index
    if !inverted_idx_columns.contains(&cfg.common.column_timestamp) {
        inverted_idx_columns.push(cfg.common.column_timestamp.to_string());
    }

    let mut inverted_idx_batches = Vec::with_capacity(batches.len());
    for batch in batches {
        let selected_column_indices: Vec<usize> = inverted_idx_columns
            .iter()
            .filter_map(|name| batch.schema().index_of(name).ok())
            .collect();

        // Use the found indices to select the columns
        let selected_columns = selected_column_indices
            .iter()
            .map(|&i| batch.column(i).clone())
            .collect();

        // Create a new schema for the new RecordBatch based on the selected columns
        let selected_fields: Vec<arrow_schema::Field> = selected_column_indices
            .iter()
            .map(|&i| batch.schema().field(i).clone())
            .collect();
        let new_schema = Arc::new(Schema::new(selected_fields));

        // Create a new RecordBatch with the selected columns
        inverted_idx_batches.push(RecordBatch::try_new(new_schema, selected_columns).unwrap());
    }

    if inverted_idx_batches.is_empty() {
        Ok(None)
    } else {
        let new_batch = if inverted_idx_batches.len() == 1 {
            inverted_idx_batches.remove(0)
        } else {
            let new_schema = inverted_idx_batches.first().unwrap().schema();
            concat_batches(new_schema, inverted_idx_batches).map_err(anyhow::Error::from)?
        };

        if matches!(
            new_batch.schema().fields().len(),
            0 | 1 if new_batch.schema().field(0).name() == &cfg.common.column_timestamp
        ) {
            Ok(None)
        } else {
            Ok(Some(new_batch))
        }
    }
}

async fn cache_remote_files(files: &[FileKey]) -> Result<Vec<String>, anyhow::Error> {
    let cfg = get_config();
    let scan_size = files.iter().map(|f| f.meta.compressed_size).sum::<i64>();
    if is_local_disk_storage()
        || !cfg.disk_cache.enabled
        || scan_size >= cfg.disk_cache.skip_size as i64
    {
        return Ok(Vec::new());
    };

    let mut tasks = Vec::new();
    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.cpu_num));
    for file in files.iter() {
        let file_name = file.key.to_string();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task: tokio::task::JoinHandle<Option<String>> = tokio::task::spawn(async move {
            let ret = if !file_data::disk::exist(&file_name).await {
                file_data::disk::download("", &file_name).await.err()
            } else {
                None
            };
            // In case where the parquet file is not found or has no data, we assume that it
            // must have been deleted by some external entity, and hence we
            // should remove the entry from file_list table.
            let file_name = if let Some(e) = ret {
                if e.to_string().to_lowercase().contains("not found")
                    || e.to_string().to_lowercase().contains("data size is zero")
                {
                    // delete file from file list
                    log::error!("found invalid file: {}", file_name);
                    if let Err(e) = file_list::delete_parquet_file(&file_name, true).await {
                        log::error!("[COMPACT] delete from file_list err: {}", e);
                    }
                    Some(file_name)
                } else {
                    log::error!("[COMPACT] download file to cache err: {}", e);
                    // remove downloaded file
                    let _ = file_data::disk::remove("", &file_name).await;
                    None
                }
            } else {
                None
            };
            drop(permit);
            file_name
        });
        tasks.push(task);
    }

    let mut delete_files = Vec::new();
    for task in tasks {
        match task.await {
            Ok(file) => {
                if let Some(file) = file {
                    delete_files.push(file);
                }
            }
            Err(e) => {
                log::error!("[COMPACT] load file task err: {}", e);
            }
        }
    }

    Ok(delete_files)
}

// generate parquet file compact schema
fn generate_schema_diff(
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
) -> Result<HashMap<String, DataType>, anyhow::Error> {
    // calculate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();

    for field in schema.fields().iter() {
        if let Some(latest_field) = schema_latest_map.get(field.name()) {
            if field.data_type() != latest_field.data_type() {
                diff_fields.insert(field.name().clone(), latest_field.data_type().clone());
            }
        }
    }

    Ok(diff_fields)
}
