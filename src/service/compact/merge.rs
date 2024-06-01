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

use std::{collections::HashMap, io::Write, sync::Arc};

use ::datafusion::{arrow::datatypes::Schema, common::FileType, error::DataFusionError};
use arrow::array::RecordBatch;
use bytes::Bytes;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use config::{
    cluster::LOCAL_NODE_UUID,
    get_config, ider,
    meta::stream::{FileKey, FileMeta, PartitionTimeLevel, StreamStats, StreamType},
    metrics,
    utils::{
        json,
        parquet::{
            parse_file_key_columns, read_recordbatch_from_bytes, write_recordbatch_to_parquet,
        },
        record_batch_ext::format_recordbatch_by_schema,
    },
    FILE_EXT_PARQUET,
};
use hashbrown::HashSet;
use infra::{
    cache, dist_lock, file_list as infra_file_list,
    schema::{
        get_stream_setting_bloom_filter_fields, get_stream_setting_fts_fields,
        unwrap_partition_time_level, unwrap_stream_settings, SchemaCache,
    },
    storage,
};
use tokio::{
    sync::{mpsc, Semaphore},
    task::JoinHandle,
};

use crate::{
    common::infra::cluster::get_node_by_uuid,
    job::files::parquet::generate_index_on_compactor,
    service::{
        db, file_list, schema::generate_schema_for_defined_schema_fields, search::datafusion,
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
) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();

    // get last compacted offset
    let (mut offset, node) = db::compact::files::get_offset(org_id, stream_type, stream_name).await;
    if !node.is_empty() && LOCAL_NODE_UUID.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        return Ok(()); // other node is processing
    }

    if node.is_empty() || LOCAL_NODE_UUID.ne(&node) {
        let lock_key = format!("/compact/merge/{}/{}/{}", org_id, stream_type, stream_name);
        let locker = dist_lock::lock(&lock_key, 0).await?;
        // check the working node again, maybe other node locked it first
        let (offset, node) = db::compact::files::get_offset(org_id, stream_type, stream_name).await;
        if !node.is_empty() && LOCAL_NODE_UUID.ne(&node) && get_node_by_uuid(&node).await.is_some()
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
            Some(&LOCAL_NODE_UUID.clone()),
        )
        .await;
        dist_lock::unlock(&locker).await?;
        drop(locker);
        ret?;
    }

    // get schema
    let schema = infra::schema::get(org_id, stream_name, stream_type).await?;
    let stream_settings = unwrap_stream_settings(&schema).unwrap_or_default();
    let partition_time_level =
        unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);
    let stream_created = stream::stream_created(&schema).unwrap_or_default();
    if offset == 0 {
        offset = stream_created
    }
    if offset == 0 {
        return Ok(()); // no data
    }

    log::debug!(
        "[COMPACTOR] merge_by_stream [{}/{}/{}] offset: {}",
        org_id,
        stream_type,
        stream_name,
        offset
    );

    let offset = offset;
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
    // 1. if step_secs less than 1 hour, must wait for at least max_file_retention_time
    // 2. if step_secs greater than 1 hour, must wait for at least 3 * max_file_retention_time
    // -- first period: the last hour local file upload to storage, write file list
    // -- second period, the last hour file list upload to storage
    // -- third period, we can do the merge, so, at least 3 times of
    // max_file_retention_time
    if (cfg.compact.step_secs < 3600
        && time_now.timestamp_micros() - offset
            <= Duration::try_seconds(cfg.limit.max_file_retention_time as i64)
                .unwrap()
                .num_microseconds()
                .unwrap())
        || (cfg.compact.step_secs >= 3600
            && (offset >= time_now_hour
                || time_now.timestamp_micros() - offset
                    <= Duration::try_seconds(cfg.limit.max_file_retention_time as i64)
                        .unwrap()
                        .num_microseconds()
                        .unwrap()
                        * 3))
    {
        return Ok(()); // the time is future, just wait
    }

    // get current hour(day) all files
    let (partition_offset_start, partition_offset_end) =
        if partition_time_level == PartitionTimeLevel::Daily {
            (
                offset_time_day,
                offset_time_day + Duration::try_hours(24).unwrap().num_microseconds().unwrap() - 1,
            )
        } else {
            (
                offset_time_hour,
                offset_time_hour + Duration::try_hours(1).unwrap().num_microseconds().unwrap() - 1,
            )
        };
    let mut files = file_list::query(
        org_id,
        stream_name,
        stream_type,
        partition_time_level,
        partition_offset_start,
        partition_offset_end,
        true,
    )
    .await
    .map_err(|e| anyhow::anyhow!("query file list failed: {}", e))?;

    // check lookback files
    if cfg.compact.lookback_hours > 0 {
        let lookback_offset = Duration::try_hours(cfg.compact.lookback_hours)
            .unwrap()
            .num_microseconds()
            .unwrap();
        let lookback_offset_start = partition_offset_start - lookback_offset;
        let mut lookback_offset_end = partition_offset_end - lookback_offset;
        if lookback_offset_end > partition_offset_start {
            // the lookback period is overlap with current period
            lookback_offset_end = partition_offset_start;
        }
        let lookback_files = file_list::query(
            org_id,
            stream_name,
            stream_type,
            partition_time_level,
            lookback_offset_start,
            lookback_offset_end,
            true,
        )
        .await
        .map_err(|e| anyhow::anyhow!("query lookback file list failed: {}", e))?;
        files.extend(lookback_files);
    }

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
        // this hour is no data, and check if pass allowed_upto, then just write new
        // offset if offset > 0 && offset_time_hour +
        // Duration::try_hours(cfg.limit.allowed_upto).unwrap().num_microseconds().unwrap() <
        // time_now_hour { -- no check it
        // }
        let offset = offset
            + Duration::try_seconds(cfg.compact.step_secs)
                .unwrap()
                .num_microseconds()
                .unwrap();
        db::compact::files::set_offset(
            org_id,
            stream_type,
            stream_name,
            offset,
            Some(&LOCAL_NODE_UUID.clone()),
        )
        .await?;
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

    // collect stream stats
    let mut stream_stats = StreamStats::default();

    // use mutiple threads to merge
    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.file_move_thread_num));
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
            files_with_size.sort_by(|a, b| a.meta.original_size.cmp(&b.meta.original_size));
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
                        break; // no files need to merge
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
                // metrics
                metrics::COMPACT_MERGED_FILES
                    .with_label_values(&[&org_id, stream_type.to_string().as_str()])
                    .inc();
                metrics::COMPACT_MERGED_BYTES
                    .with_label_values(&[&org_id, stream_type.to_string().as_str()])
                    .inc_by(file.meta.original_size as u64);
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
                    if new_file_list.is_empty() {
                        // no file need to merge
                        break;
                    } else {
                        // delete files from file_list and continue
                        files_with_size.retain(|f| !&new_file_list.contains(f));
                        continue;
                    }
                }

                // delete small files keys & write big files keys, use transaction
                let mut events = Vec::with_capacity(new_file_list.len() + 1);
                events.push(FileKey {
                    key: new_file_name.clone(),
                    meta: new_file_meta,
                    deleted: false,
                });
                for file in new_file_list.iter() {
                    stream_stats = stream_stats - file.meta.clone();
                    events.push(FileKey {
                        key: file.key.clone(),
                        meta: file.meta.clone(),
                        deleted: true,
                    });
                }
                events.sort_by(|a, b| a.key.cmp(&b.key));

                // write file list to storage
                match write_file_list(&org_id, &events).await {
                    Ok(_) => {}
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

    // write new offset
    let offset = offset
        + Duration::try_seconds(cfg.compact.step_secs)
            .unwrap()
            .num_microseconds()
            .unwrap();
    db::compact::files::set_offset(
        org_id,
        stream_type,
        stream_name,
        offset,
        Some(&LOCAL_NODE_UUID.clone()),
    )
    .await?;

    // update stream stats
    if stream_stats.doc_num != 0 {
        infra_file_list::set_stream_stats(
            org_id,
            &[(
                format!("{org_id}/{stream_type}/{stream_name}"),
                stream_stats,
            )],
        )
        .await?;
    }

    // metrics
    let time = start.elapsed().as_secs_f64();
    metrics::COMPACT_USED_TIME
        .with_label_values(&[org_id, stream_type.to_string().as_str()])
        .inc_by(time);
    metrics::COMPACT_DELAY_HOURS
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .set(
            (time_now_hour - offset_time_hour)
                / Duration::try_hours(1).unwrap().num_microseconds().unwrap(),
        );

    Ok(())
}

/// merge some small files into one big file, upload to storage, returns the big
/// file key and merged files
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
    let mut total_records = 0;
    let mut new_file_list = Vec::new();
    let mut deleted_files = Vec::new();
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

        total_records += file.meta.records;
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

    // write parquet files into tmpfs
    let tmp_dir = cache::tmpfs::Directory::default();
    for file in &new_file_list {
        log::info!("[COMPACT:{thread_id}] merge small file: {}", &file.key);
        let data = match storage::get(&file.key).await {
            Ok(body) => body,
            Err(err) => {
                log::error!(
                    "[COMPACT:{thread_id}] merge small file: {}, err: {}",
                    &file.key,
                    err
                );
                if err.to_string().to_lowercase().contains("not found") {
                    // delete file from file list
                    if let Err(err) = file_list::delete_parquet_file(&file.key, true).await {
                        log::error!(
                            "[COMPACT:{thread_id}] delete file: {}, from file_list err: {}",
                            &file.key,
                            err
                        );
                    }
                }
                deleted_files.push(file.key.clone());
                continue;
            }
        };
        tmp_dir.set(&file.key, data)?;
    }
    if !deleted_files.is_empty() {
        new_file_list.retain(|f| !deleted_files.contains(&f.key));
    }
    if new_file_list.len() <= 1 {
        return Ok((String::from(""), FileMeta::default(), retain_file_list));
    }

    // get time range for these files
    let min_ts = new_file_list.iter().map(|f| f.meta.min_ts).min().unwrap();
    let max_ts = new_file_list.iter().map(|f| f.meta.max_ts).max().unwrap();

    // convert the file to the latest version of schema
    let schema_latest = infra::schema::get(org_id, stream_name, stream_type).await?;
    let stream_setting = infra::schema::get_settings(org_id, stream_name, stream_type).await;
    let defined_schema_fields = stream_setting
        .and_then(|s| s.defined_schema_fields)
        .unwrap_or_default();
    let schema_latest = if !defined_schema_fields.is_empty() {
        let schema_latest = SchemaCache::new(schema_latest);
        let schema_latest =
            generate_schema_for_defined_schema_fields(&schema_latest, &defined_schema_fields);
        Arc::new(schema_latest.schema().clone())
    } else {
        Arc::new(schema_latest)
    };

    let schema_versions =
        infra::schema::get_versions(org_id, stream_name, stream_type, Some((min_ts, max_ts)))
            .await?;
    let schema_latest_id = schema_versions.len() - 1;
    let bloom_filter_fields = get_stream_setting_bloom_filter_fields(&schema_latest).unwrap();
    let full_text_search_fields = get_stream_setting_fts_fields(&schema_latest).unwrap();
    if cfg.common.widening_schema_evolution && schema_versions.len() > 1 {
        for file in &new_file_list {
            // get the schema version of the file
            let schema_ver_id = match db::schema::filter_schema_version_id(
                &schema_versions,
                file.meta.min_ts,
                file.meta.max_ts,
            ) {
                Some(id) => id,
                None => {
                    log::error!(
                        "[COMPACT:{thread_id}] merge small file: {}, schema version not found, min_ts: {}, max_ts: {}",
                        &file.key,
                        file.meta.min_ts,
                        file.meta.max_ts
                    );
                    // HACK: use the latest version if not found in schema versions
                    schema_latest_id
                }
            };
            if schema_ver_id == schema_latest_id {
                continue;
            }
            // cacluate the diff between latest schema and current schema
            let schema = schema_versions[schema_ver_id]
                .clone()
                .with_metadata(HashMap::new());
            let mut diff_fields = hashbrown::HashMap::new();
            let cur_fields = schema.fields();
            for field in cur_fields {
                if let Ok(v) = schema_latest.field_with_name(field.name()) {
                    if v.data_type() != field.data_type() {
                        diff_fields.insert(v.name().clone(), v.data_type().clone());
                    }
                }
            }
            if diff_fields.is_empty() {
                continue;
            }

            // do the convert
            let mut buf = Vec::new();
            let file_tmp_dir = cache::tmpfs::Directory::default();
            let file_data = storage::get(&file.key).await?;
            if file_data.is_empty() {
                // delete file from file list
                log::warn!("found invalid file: {}", file.key);
                if let Err(err) = file_list::delete_parquet_file(&file.key, true).await {
                    log::error!(
                        "[COMPACT:{thread_id}] delete file: {}, from file_list err: {}",
                        &file.key,
                        err
                    );
                }
                return Err(anyhow::anyhow!("merge_files error: file data is empty"));
            }
            file_tmp_dir.set(&file.key, file_data)?;
            datafusion::exec::convert_parquet_file(
                file_tmp_dir.name(),
                &mut buf,
                Arc::new(schema),
                &bloom_filter_fields,
                &full_text_search_fields,
                diff_fields,
                FileType::PARQUET,
            )
            .await
            .map_err(|e| {
                DataFusionError::Plan(format!("convert_parquet_file {}, err: {}", &file.key, e))
            })?;

            // replace the file in tmpfs
            tmp_dir.set(&file.key, buf.into())?;
        }
    }

    let mut new_file_meta = FileMeta {
        min_ts,
        max_ts,
        records: total_records,
        original_size: new_file_size,
        compressed_size: 0,
        flattened: false,
    };
    if new_file_meta.records == 0 {
        return Err(anyhow::anyhow!("merge_parquet_files error: records is 0"));
    }

    let start = std::time::Instant::now();
    let merge_result = if stream_type == StreamType::Logs {
        merge_parquet_files(thread_id, tmp_dir.name(), schema_latest.clone()).await
    } else {
        datafusion::exec::merge_parquet_files(
            tmp_dir.name(),
            stream_type,
            stream_name,
            schema_latest.clone(),
        )
        .await
    };
    let (new_schema, new_batches) = merge_result.map_err(|e| {
        let files = tmp_dir.list("all").unwrap();
        let files = files.into_iter().map(|f| f.location).collect::<Vec<_>>();
        log::error!(
            "merge_parquet_files err: {}, files: {:?}, schema: {:?}",
            e,
            files,
            schema_latest
        );

        DataFusionError::Plan(format!("merge_parquet_files err: {:?}", e))
    })?;

    let buf = write_recordbatch_to_parquet(
        new_schema.clone(),
        &new_batches,
        &bloom_filter_fields,
        &full_text_search_fields,
        &new_file_meta,
    )
    .await?;
    new_file_meta.compressed_size = buf.len() as i64;
    if new_file_meta.compressed_size == 0 {
        return Err(anyhow::anyhow!(
            "merge_parquet_files error: compressed_size is 0"
        ));
    }

    // generate inverted index RecordBatch
    let inverted_idx_batches = generate_inverted_idx_recordbatch(
        schema_latest.clone(),
        &new_batches,
        stream_type,
        &full_text_search_fields,
    );

    let id = ider::generate();
    let new_file_key = format!("{prefix}/{id}{}", FILE_EXT_PARQUET);
    log::info!(
        "[COMPACT:{thread_id}] merge file succeeded, {} files into a new file: {}, original_size: {}, compressed_size: {}, took: {} ms",
        retain_file_list.len(),
        new_file_key,
        new_file_meta.original_size,
        new_file_meta.compressed_size,
        start.elapsed().as_millis(),
    );

    let buf = Bytes::from(buf);
    // upload file
    match storage::put(&new_file_key, buf.clone()).await {
        Ok(_) => {
            if cfg.common.inverted_index_enabled && stream_type == StreamType::Logs {
                let (index_file_name, filemeta) = generate_index_on_compactor(
                    &retain_file_list,
                    inverted_idx_batches,
                    new_file_key.clone(),
                    org_id,
                    stream_name,
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "generate_index_on_compactor error: {}, need delete files: {:?}",
                        e,
                        retain_file_list
                    )
                })?;
                if !index_file_name.is_empty() {
                    log::info!("Created index file during compaction {}", index_file_name);
                    // Notify that we wrote the index file to the db.
                    if let Err(e) = write_file_list(
                        org_id,
                        &[FileKey {
                            key: index_file_name.clone(),
                            meta: filemeta,
                            deleted: false,
                        }],
                    )
                    .await
                    {
                        log::error!(
                            "generate_index_on_compactor write to file list: {}, error: {}, need delete files: {:?}",
                            index_file_name,
                            e.to_string(),
                            retain_file_list
                        );
                    }
                } else {
                    log::warn!(
                        "generate_index_on_compactor returned an empty index file name and need delete files: {:?}",
                        retain_file_list
                    );
                }
            }
            Ok((new_file_key, new_file_meta, retain_file_list))
        }
        Err(e) => Err(e),
    }
}

async fn write_file_list(org_id: &str, events: &[FileKey]) -> Result<(), anyhow::Error> {
    if events.is_empty() {
        return Ok(());
    }
    if get_config().common.meta_store_external {
        write_file_list_db_only(org_id, events).await
    } else {
        write_file_list_s3(org_id, events).await
    }
}

async fn write_file_list_db_only(org_id: &str, events: &[FileKey]) -> Result<(), anyhow::Error> {
    let put_items = events
        .iter()
        .filter(|v| !v.deleted)
        .map(|v| v.to_owned())
        .collect::<Vec<_>>();
    let del_items_need_flatten = events
        .iter()
        .filter(|v| v.deleted && v.meta.flattened)
        .map(|v| v.key.clone())
        .collect::<Vec<_>>();
    let del_items_noneed_flatten = events
        .iter()
        .filter(|v| v.deleted && !v.meta.flattened)
        .map(|v| v.key.clone())
        .collect::<Vec<_>>();
    // set to external db
    // retry 5 times
    let mut success = false;
    let created_at = Utc::now().timestamp_micros();
    for _ in 0..5 {
        if !del_items_need_flatten.is_empty() {
            if let Err(e) = infra_file_list::batch_add_deleted(
                org_id,
                true,
                created_at,
                &del_items_need_flatten,
            )
            .await
            {
                log::error!(
                    "[COMPACT] batch_add_deleted to external db failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        }
        if !del_items_noneed_flatten.is_empty() {
            if let Err(e) = infra_file_list::batch_add_deleted(
                org_id,
                false,
                created_at,
                &del_items_noneed_flatten,
            )
            .await
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
        if !del_items_need_flatten.is_empty() {
            if let Err(e) = infra_file_list::batch_remove(&del_items_need_flatten).await {
                log::error!(
                    "[COMPACT] batch_delete to external db failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        }
        if !del_items_noneed_flatten.is_empty() {
            if let Err(e) = infra_file_list::batch_remove(&del_items_noneed_flatten).await {
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

async fn write_file_list_s3(org_id: &str, events: &[FileKey]) -> Result<(), anyhow::Error> {
    // write new data into file_list
    let (_stream_key, date_key, _file_name) = parse_file_key_columns(&events.first().unwrap().key)?;
    // upload the new file_list to storage
    let new_file_list_key = format!("file_list/{}/{}.json.zst", date_key, ider::generate());
    let mut buf = zstd::Encoder::new(Vec::new(), 3)?;
    for file in events.iter() {
        let mut write_buf = json::to_vec(&file)?;
        write_buf.push(b'\n');
        buf.write_all(&write_buf)?;
    }
    let compressed_bytes = buf.finish().unwrap();
    storage::put(&new_file_list_key, compressed_bytes.into()).await?;

    // write deleted files into file_list_deleted
    let del_items = events
        .iter()
        .filter(|v| v.deleted)
        .map(|v| v.key.clone())
        .collect::<Vec<_>>();
    if !del_items.is_empty() {
        let deleted_file_list_key = format!(
            "file_list_deleted/{org_id}/{}/{}.json.zst",
            Utc::now().format("%Y/%m/%d/%H"),
            ider::generate()
        );
        let mut buf = zstd::Encoder::new(Vec::new(), 3)?;
        for file in del_items.iter() {
            buf.write_all((file.to_owned() + "\n").as_bytes())?;
        }
        let compressed_bytes = buf.finish().unwrap();
        storage::put(&deleted_file_list_key, compressed_bytes.into()).await?;
    }

    // set to local cache & send broadcast
    // retry 5 times
    for _ in 0..5 {
        // set to local cache
        let mut cache_success = true;
        for event in events.iter() {
            if let Err(e) =
                db::file_list::progress(&event.key, Some(&event.meta), event.deleted).await
            {
                cache_success = false;
                log::error!(
                    "[COMPACT] set local cache for file_list failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                break;
            }
        }
        if !cache_success {
            continue;
        }
        // send broadcast to other nodes
        if let Err(e) = db::file_list::broadcast::send(events, None).await {
            log::error!(
                "[COMPACT] send broadcast for file_list failed, retrying: {}",
                e
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        }
        // broadcast success
        break;
    }
    Ok(())
}

pub fn generate_inverted_idx_recordbatch(
    schema: Arc<Schema>,
    batches: &[RecordBatch],
    stream_type: StreamType,
    full_text_search_fields: &[String],
) -> Vec<RecordBatch> {
    let cfg = get_config();
    if !cfg.common.inverted_index_enabled || batches.is_empty() || stream_type != StreamType::Logs {
        return Vec::new();
    }

    let mut inverted_idx_columns = if !full_text_search_fields.is_empty() {
        full_text_search_fields.to_vec()
    } else {
        config::SQL_FULL_TEXT_SEARCH_FIELDS.to_vec()
    };

    let schema_fields = schema
        .fields()
        .iter()
        .map(|f| f.name())
        .collect::<HashSet<_>>();
    inverted_idx_columns.retain(|f| schema_fields.contains(f));
    if inverted_idx_columns.is_empty() {
        return Vec::new();
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

    inverted_idx_batches
}

pub async fn merge_parquet_files(
    thread_id: usize,
    trace_id: &str,
    schema: Arc<Schema>,
) -> ::datafusion::error::Result<(Arc<Schema>, Vec<RecordBatch>)> {
    let start = std::time::Instant::now();

    // 1. get record batches from tmpfs
    let temp_files = infra::cache::tmpfs::list(trace_id, "parquet").map_err(|e| {
        log::error!(
            "[INGESTER:JOB:{thread_id}] merge small files failed at getting temp files. Error {}",
            e
        );
        DataFusionError::Execution(e.to_string())
    })?;

    let mut record_batches = Vec::new();
    for file in temp_files {
        let bytes = infra::cache::tmpfs::get(&file.location).map_err(|e| {
            log::error!(
                "[INGESTER:JOB:{thread_id}] merge small files failed at reading temp files to bytes. Error {}",
                e
            );
            DataFusionError::Execution(e.to_string())
        })?;

        let (_, batches) = read_recordbatch_from_bytes(&bytes).await.map_err(|e| {
            log::error!("[INGESTER:JOB:{thread_id}] read_recordbatch_from_bytes error",);
            log::error!(
                "[INGESTER:JOB:{thread_id}] read_recordbatch_from_bytes error for file: {}, err: {}",
                file.location,
                e
            );
            DataFusionError::Execution(e.to_string())
        })?;
        record_batches.extend(batches);
    }

    // format recordbatch
    let record_batches = record_batches
        .into_iter()
        .map(|b| format_recordbatch_by_schema(schema.clone(), b))
        .collect::<Vec<_>>();

    // 2. concatenate all record batches into one single RecordBatch
    let concatenated_record_batch = arrow::compute::concat_batches(&schema, &record_batches)?;

    // 3. sort concatenated record batch by timestamp col in desc order
    let sort_indices = arrow::compute::sort_to_indices(
        concatenated_record_batch
            .column_by_name(&get_config().common.column_timestamp)
            .ok_or_else(|| {
                log::error!(
                    "[INGESTER:JOB:{thread_id}] merge small files failed to find _timestamp column from merged record batch.",
                );
                DataFusionError::Execution(
                    "No _timestamp column found in merged record batch".to_string(),
                )
            })?,
        Some(arrow_schema::SortOptions {
            descending: false,
            nulls_first: true,
        }),
        None,
    )?;

    let sorted_columns = concatenated_record_batch
        .columns()
        .iter()
        .map(|c| arrow::compute::take(c, &sort_indices, None))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let final_record_batch = RecordBatch::try_new(schema.clone(), sorted_columns)?;
    let schema = final_record_batch.schema();

    log::info!(
        "merge_parquet_files took {} ms",
        start.elapsed().as_millis()
    );

    Ok((schema, vec![final_record_batch]))
}
