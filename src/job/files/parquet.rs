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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use arrow::{
    array::{ArrayRef, BooleanArray, Int64Array, StringArray},
    record_batch::RecordBatch,
};
use arrow_schema::{DataType, Schema, SchemaRef};
use bytes::Bytes;
use chrono::{Duration, Utc};
use config::{
    cluster,
    meta::stream::{FileKey, FileMeta, PartitionTimeLevel, StreamType},
    metrics,
    utils::{
        arrow::record_batches_to_json_rows,
        asynchronism::file::{get_file_contents, get_file_meta},
        file::scan_files_for_move,
        json,
        parquet::{
            read_metadata_from_file, read_recordbatch_from_bytes, read_schema_from_file,
            write_recordbatch_to_parquet,
        },
        schema_ext::SchemaExt,
    },
    FxIndexMap, CONFIG, DEFAULT_INDEX_TRIM_CHARS, INDEX_MIN_CHAR_LEN,
};
use datafusion::{datasource::MemTable, prelude::*};
use hashbrown::HashSet;
use infra::{
    cache::{self, tmpfs},
    schema::SchemaCache,
    storage,
};
use ingester::WAL_PARQUET_METADATA;
use once_cell::sync::Lazy;
use tokio::{
    sync::{Mutex, RwLock},
    time,
};

use crate::{
    common::{infra::wal, meta::stream::SchemaRecords},
    job::files::idx::write_to_disk,
    service::{
        compact::merge::{generate_inverted_idx_recordbatch, merge_parquet_files},
        db,
        schema::generate_schema_for_defined_schema_fields,
        search::datafusion::{
            exec::merge_parquet_files as merge_parquet_files_by_datafusion,
            string_to_array_v2_udf::STRING_TO_ARRAY_V2_UDF,
        },
    },
};

static PROCESSING_FILES: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));
static LAST_SCANNED_DIR: Lazy<Arc<RwLock<PathBuf>>> = Lazy::new(|| {
    Arc::new(RwLock::new({
        let wal_dir = Path::new(&CONFIG.common.data_wal_dir)
            .canonicalize()
            .unwrap();
        wal_dir.join("files/")
    }))
});

pub async fn run() -> Result<(), anyhow::Error> {
    let (tx, rx) =
        tokio::sync::mpsc::channel::<(String, Vec<FileKey>)>(CONFIG.limit.file_move_thread_num);
    let rx = Arc::new(Mutex::new(rx));
    // move files
    for thread_id in 0..CONFIG.limit.file_move_thread_num {
        let rx = rx.clone();
        tokio::spawn(async move {
            loop {
                let ret = rx.lock().await.recv().await;
                match ret {
                    None => {
                        log::debug!("[INGESTER:JOB] Receiving files channel is closed");
                        break;
                    }
                    Some((prefix, files)) => {
                        println!("prefix: {}, files: {:?}", prefix, files);
                        if let Err(e) = move_files(thread_id, &prefix, files).await {
                            log::error!(
                                "[INGESTER:JOB] Error moving parquet files to remote: {}",
                                e
                            );
                        }
                    }
                }
            }
        });
    }

    // prepare files
    let mut interval = time::interval(time::Duration::from_secs(CONFIG.limit.file_push_interval));
    interval.tick().await; // trigger the first run
    loop {
        if cluster::is_offline() {
            break;
        }
        interval.tick().await;
        match prepare_files().await {
            Err(e) => {
                log::error!("[INGESTER:JOB] Error prepare parquet files: {}", e);
            }
            Ok((files, last_scanned_dir)) => {
                for (prefix, files) in files.into_iter() {
                    if let Err(e) = tx.send((prefix, files)).await {
                        log::error!("[INGESTER:JOB] Error sending parquet files to move: {}", e);
                    } else {
                        let mut w = LAST_SCANNED_DIR.write().await;
                        w.push(PathBuf::from(last_scanned_dir.clone()));
                        drop(w)
                    }
                }
            }
        }
    }
    log::info!("job::files::parquet is stopped");
    Ok(())
}

async fn prepare_files() -> Result<(FxIndexMap<String, Vec<FileKey>>, String), anyhow::Error> {
    let start = std::time::Instant::now();
    let wal_dir = Path::new(&CONFIG.common.data_wal_dir)
        .canonicalize()
        .unwrap();

    let pattern = wal_dir.join("files/").to_string_lossy().to_string();
    let r = LAST_SCANNED_DIR.read().await;
    let start_dir = LAST_SCANNED_DIR.read().await.clone();
    drop(r);

    let (files, last_scanned_dir) = scan_files_for_move(
        pattern,
        "parquet",
        Some(CONFIG.limit.file_push_limit),
        start_dir.to_str().unwrap(),
    )
    .unwrap_or_default();
    if files.is_empty() {
        let pattern = wal_dir.join("files/").to_string_lossy().to_string();
        let mut w = LAST_SCANNED_DIR.write().await;
        w.push(PathBuf::from(pattern.clone()));
        drop(w);

        return Ok((FxIndexMap::default(), pattern));
    }
    log::debug!(
        "[INGESTER:JOB] move files get: {}, took: {} ms",
        files.len(),
        start.elapsed().as_millis()
    );

    // do partition by partition key
    let mut partition_files_with_size: FxIndexMap<String, Vec<FileKey>> = FxIndexMap::default();
    for file in files {
        let file_key = {
            let file = match Path::new(&file).canonicalize() {
                Ok(v) => v,
                Err(_) => {
                    continue;
                }
            };
            let file = match file.strip_prefix(&wal_dir) {
                Ok(v) => v,
                Err(_) => {
                    continue;
                }
            };
            file.to_str().unwrap().replace('\\', "/")
        };
        // check if the file is processing
        if PROCESSING_FILES.read().await.contains(&file_key) {
            continue;
        }

        let parquet_meta = if let Some(meta) = WAL_PARQUET_METADATA.read().await.get(&file_key) {
            meta.clone()
        } else if let Ok(parquet_meta) = read_metadata_from_file(&(&file).into()).await {
            parquet_meta
        } else {
            continue;
        };
        if parquet_meta.eq(&FileMeta::default()) {
            log::warn!(
                "[INGESTER:JOB] the file is empty, just delete file: {}",
                file
            );
            if let Err(e) = tokio::fs::remove_file(wal_dir.join(&file)).await {
                log::error!(
                    "[INGESTER:JOB] Failed to remove parquet file from disk: {}, {}",
                    file,
                    e
                );
            }
            // delete metadata from cache
            WAL_PARQUET_METADATA.write().await.remove(&file_key);
            continue;
        }
        let prefix = file_key[..file_key.rfind('/').unwrap()].to_string();
        let partition = partition_files_with_size.entry(prefix).or_default();
        partition.push(FileKey::new(&file_key, parquet_meta, false));
        // mark the file as processing
        // log::debug!("Processing files created: {:?}", file_key);
        PROCESSING_FILES.write().await.insert(file_key);
    }
    log::debug!(
        "[INGESTER:JOB] move files get partitions: {}",
        partition_files_with_size.len()
    );

    Ok((partition_files_with_size, last_scanned_dir))
}

async fn move_files(
    thread_id: usize,
    prefix: &str,
    files: Vec<FileKey>,
) -> Result<(), anyhow::Error> {
    if files.is_empty() {
        return Ok(());
    }

    let columns = prefix.splitn(5, '/').collect::<Vec<&str>>();
    // eg: files/default/logs/olympics/0/2023/08/21/08/8b8a5451bbe1c44b/
    // eg: files/default/traces/default/0/2023/09/04/05/default/service_name=ingester/
    // let _ = columns[0].to_string(); // files/
    let org_id = columns[1].to_string();
    let stream_type = StreamType::from(columns[2]);
    let stream_name = columns[3].to_string();

    // log::debug!("[INGESTER:JOB:{thread_id}] check deletion for partition: {}", prefix);

    let wal_dir = Path::new(&CONFIG.common.data_wal_dir)
        .canonicalize()
        .unwrap();

    // check if we are allowed to ingest or just delete the file
    if db::compact::retention::is_deleting_stream(&org_id, stream_type, &stream_name, None) {
        for file in files {
            log::warn!(
                "[INGESTER:JOB:{thread_id}] the stream [{}/{}/{}] is deleting, just delete file: {}",
                &org_id,
                stream_type,
                &stream_name,
                file.key,
            );
            if let Err(e) = tokio::fs::remove_file(wal_dir.join(&file.key)).await {
                log::error!(
                    "[INGESTER:JOB:{thread_id}] Failed to remove parquet file from disk: {}, {}",
                    file.key,
                    e
                );
            }
            // delete metadata from cache
            WAL_PARQUET_METADATA.write().await.remove(&file.key);
            PROCESSING_FILES.write().await.remove(&file.key);
        }
        return Ok(());
    }

    log::debug!(
        "[INGESTER:JOB:{thread_id}] get schema for partition: {}",
        prefix
    );

    // get latest schema
    let latest_schema = match infra::schema::get(&org_id, &stream_name, stream_type).await {
        Ok(schema) => Arc::new(schema),
        Err(e) => {
            log::error!(
                "[INGESTER:JOB:{thread_id}] Failed to get latest schema for stream [{}/{}/{}]: {}",
                &org_id,
                stream_type,
                &stream_name,
                e
            );
            // need release all the files
            for file in files.iter() {
                PROCESSING_FILES.write().await.remove(&file.key);
            }
            return Err(e.into());
        }
    };

    // get group file schema
    let group_schema = read_schema_from_file(&wal_dir.join(&files.first().unwrap().key)).await?;
    let group_schema_field_num = group_schema.fields().len();

    // log::debug!("[INGESTER:JOB:{thread_id}] start processing for partition: {}", prefix);

    let wal_dir = wal_dir.clone();
    // sort by created time
    let mut files_with_size = files.to_owned();
    files_with_size.sort_by(|a, b| a.meta.min_ts.cmp(&b.meta.min_ts));
    // check the total size
    let total_original_size: i64 = files_with_size
        .iter()
        .map(|f| f.meta.original_size)
        .sum::<i64>();
    if total_original_size
        < std::cmp::min(
            CONFIG.limit.max_file_size_on_disk as i64,
            CONFIG.compact.max_file_size as i64,
        )
        && (CONFIG.limit.file_move_fields_limit == 0
            || group_schema_field_num < CONFIG.limit.file_move_fields_limit)
    {
        let mut has_expired_files = false;
        // not enough files to upload, check if some files are too old
        let min_ts = Utc::now().timestamp_micros()
            - Duration::try_seconds(CONFIG.limit.max_file_retention_time as i64)
                .unwrap()
                .num_microseconds()
                .unwrap();
        for file in files_with_size.iter() {
            let Ok(file_meta) = get_file_meta(&wal_dir.join(&file.key)).await else {
                continue;
            };
            let file_created = file_meta
                .created()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64;
            if file_created <= min_ts {
                has_expired_files = true;
                break;
            }
        }
        if !has_expired_files {
            // need release all the files
            for file in files_with_size.iter() {
                PROCESSING_FILES.write().await.remove(&file.key);
            }
            return Ok(());
        }
    }

    log::debug!(
        "[INGESTER:JOB:{thread_id}] start merging for partition: {}",
        prefix
    );

    // start merge files and upload to s3
    loop {
        // yield to other tasks
        tokio::task::yield_now().await;
        // merge file and get the big file key
        let (new_file_name, new_file_meta, new_file_list) = match merge_files(
            thread_id,
            latest_schema.clone(),
            group_schema_field_num,
            &wal_dir,
            &files_with_size,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                log::error!("[INGESTER:JOB] merge files failed: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
        };
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

        // write file list to storage
        let ret = db::file_list::local::set(&new_file_name, Some(new_file_meta), false).await;
        if let Err(e) = ret {
            log::error!(
                "[INGESTER:JOB] Failed write parquet file meta: {}, error: {}",
                new_file_name,
                e.to_string()
            );
            // need release all the files
            for file in files_with_size.iter() {
                PROCESSING_FILES.write().await.remove(&file.key);
            }
            return Ok(());
        }

        // check if allowed to delete the file
        for file in new_file_list.iter() {
            loop {
                if wal::lock_files_exists(&file.key).await {
                    log::warn!(
                        "[INGESTER:JOB:{thread_id}] the file is still in use, waiting for a few ms: {}",
                        file.key
                    );
                    time::sleep(time::Duration::from_millis(100)).await;
                } else {
                    break;
                }
            }

            let ret = tokio::fs::remove_file(&wal_dir.join(&file.key)).await;
            if let Err(e) = ret {
                log::error!(
                    "[INGESTER:JOB:{thread_id}] Failed to remove parquet file from disk: {}, {}",
                    file.key,
                    e.to_string()
                );
                // delete metadata from cache
                WAL_PARQUET_METADATA.write().await.remove(&file.key);
                // need release all the files
                for file in files_with_size.iter() {
                    PROCESSING_FILES.write().await.remove(&file.key);
                }
                return Ok(());
            }

            // delete metadata from cache
            WAL_PARQUET_METADATA.write().await.remove(&file.key);

            // remove the file from processing set
            // log::debug!("Processing files deleted: {:?}", file.key);
            PROCESSING_FILES.write().await.remove(&file.key);

            // metrics
            metrics::INGEST_WAL_READ_BYTES
                .with_label_values(&[&org_id, stream_type.to_string().as_str()])
                .inc_by(file.meta.compressed_size as u64);
            metrics::INGEST_WAL_USED_BYTES
                .with_label_values(&[&org_id, stream_type.to_string().as_str()])
                .sub(file.meta.compressed_size);
        }

        // delete files from file list
        let new_file_list = new_file_list.iter().map(|f| &f.key).collect::<Vec<_>>();
        files_with_size.retain(|f| !new_file_list.contains(&&f.key));
    }

    Ok(())
}

/// merge some small files into one big file, upload to storage, returns the big
/// file key and merged files
async fn merge_files(
    thread_id: usize,
    latest_schema: Arc<Schema>,
    group_schema_field_num: usize,
    wal_dir: &Path,
    files_with_size: &[FileKey],
) -> Result<(String, FileMeta, Vec<FileKey>), anyhow::Error> {
    if files_with_size.is_empty() {
        return Ok((String::from(""), FileMeta::default(), Vec::new()));
    }

    let mut new_file_size: i64 = 0;
    let mut new_file_list = Vec::new();
    let mut deleted_files = Vec::new();
    let mut min_ts = i64::MAX;
    let mut max_ts = i64::MIN;
    let mut total_records = 0;

    for file in files_with_size.iter() {
        if new_file_size > 0
            && ((new_file_size + file.meta.original_size > CONFIG.compact.max_file_size as i64)
                || (CONFIG.limit.file_move_fields_limit > 0
                    && group_schema_field_num > CONFIG.limit.file_move_fields_limit))
        {
            break;
        }
        new_file_size += file.meta.original_size;

        new_file_list.push(file.clone());
    }
    let mut retain_file_list = new_file_list.clone();

    // write parquet files into tmpfs
    let tmp_dir = cache::tmpfs::Directory::default();
    for file in retain_file_list.iter_mut() {
        log::info!("[INGESTER:JOB:{thread_id}] merge small file: {}", &file.key);
        let data = match get_file_contents(&wal_dir.join(&file.key)).await {
            Ok(body) => {
                min_ts = std::cmp::min(min_ts, file.meta.min_ts);
                max_ts = std::cmp::max(max_ts, file.meta.max_ts);
                total_records += file.meta.records;
                body
            }
            Err(err) => {
                log::error!(
                    "[INGESTER:JOB:{thread_id}] merge small file: {}, err: {}",
                    &file.key,
                    err
                );
                deleted_files.push(file.key.clone());
                continue;
            }
        };
        let file_size = data.len();
        file.meta.compressed_size = file_size as i64;
        tmp_dir.set(&file.key, data.into())?;
    }
    if !deleted_files.is_empty() {
        new_file_list.retain(|f| !deleted_files.contains(&f.key));
    }
    if new_file_list.is_empty() {
        return Ok((String::from(""), FileMeta::default(), retain_file_list));
    }

    // eg: files/default/logs/olympics/0/2023/08/21/08/8b8a5451bbe1c44b/
    // 7099303408192061440f3XQ2p.parquet
    // eg: files/default/traces/default/0/023/09/04/05/default/
    // service_name=ingester/7104328279989026816guOA4t.parquet
    // let _ = columns[0].to_string(); // files/
    let file = new_file_list.first().unwrap();
    let columns = file.key.splitn(5, '/').collect::<Vec<&str>>();
    let org_id = columns[1].to_string();
    let stream_type = StreamType::from(columns[2]);
    let stream_name = columns[3].to_string();
    let file_name = columns[4].to_string();

    // merge files
    let stream_setting = infra::schema::get_settings(&org_id, &stream_name, stream_type)
        .await
        .unwrap_or_default();
    let bloom_filter_fields = stream_setting.bloom_filter_fields;
    let full_text_search_fields = stream_setting.full_text_search_keys;
    let defined_schema_fields = stream_setting.defined_schema_fields.unwrap_or_default();
    let schema = if !defined_schema_fields.is_empty() {
        let latest_schema = SchemaCache::new(latest_schema.as_ref().clone());
        let latest_schema =
            generate_schema_for_defined_schema_fields(&latest_schema, &defined_schema_fields);
        Arc::new(latest_schema.schema().clone())
    } else {
        latest_schema.clone()
    };

    let mut new_file_meta = FileMeta {
        min_ts,
        max_ts,
        records: total_records,
        original_size: new_file_size,
        compressed_size: 0,
    };
    if new_file_meta.records == 0 {
        return Err(anyhow::anyhow!(
            "merge_parquet_files error: records is 0 for org {} stream type {} stream {}",
            org_id,
            stream_type,
            stream_name
        ));
    }

    let start = std::time::Instant::now();
    let mut buf = Vec::new();
    let single_file = new_file_list.len() == 1;
    let merge_result = if single_file {
        move_single_file(
            thread_id,
            tmp_dir.name(),
            file,
            stream_type,
            &stream_name,
            &mut buf,
        )
        .await
    } else if stream_type == StreamType::Logs {
        merge_parquet_files(thread_id, tmp_dir.name(), schema.clone()).await
    } else {
        merge_parquet_files_by_datafusion(tmp_dir.name(), stream_type, &stream_name, schema.clone())
            .await
    };
    let (new_schema, new_batches) = match merge_result {
        Ok(v) => v,
        Err(e) => {
            log::error!(
                "[INGESTER:JOB:{thread_id}] merge_parquet_files error for stream: {}/{}/{}, err: {}",
                org_id,
                stream_type,
                stream_name,
                e
            );
            log::error!(
                "[INGESTER:JOB:{thread_id}] merge_parquet_files error for files: {:?}",
                retain_file_list
            );
            return Err(e.into());
        }
    };
    if !single_file {
        buf = write_recordbatch_to_parquet(
            new_schema.clone(),
            &new_batches,
            &bloom_filter_fields,
            &full_text_search_fields,
            &new_file_meta,
        )
        .await?;
    }
    new_file_meta.compressed_size = buf.len() as i64;
    if new_file_meta.compressed_size == 0 {
        return Err(anyhow::anyhow!(
            "merge_parquet_files error: compressed_size is 0"
        ));
    }
    let new_file_key =
        super::generate_storage_file_name(&org_id, stream_type, &stream_name, &file_name);
    log::info!(
        "[INGESTER:JOB:{thread_id}] merge file succeeded, {} files into a new file: {}, original_size: {}, compressed_size: {}, took: {} ms",
        retain_file_list.len(),
        new_file_key,
        new_file_meta.original_size,
        new_file_meta.compressed_size,
        start.elapsed().as_millis(),
    );

    // generate inverted index RecordBatch
    let inverted_idx_batches = generate_inverted_idx_recordbatch(
        new_schema.clone(),
        &new_batches,
        stream_type,
        &full_text_search_fields,
    );

    // upload file
    let buf = Bytes::from(buf);
    match storage::put(&new_file_key, buf).await {
        Ok(_) => {
            if CONFIG.common.inverted_index_enabled && stream_type != StreamType::Index {
                generate_index_on_ingester(
                    inverted_idx_batches,
                    new_file_key.clone(),
                    &org_id,
                    &stream_name,
                )
                .await
                .map_err(|e| anyhow::anyhow!("generate_index_on_ingester error: {}", e))?;
            }
            Ok((new_file_key, new_file_meta, retain_file_list))
        }
        Err(e) => Err(e),
    }
}

/// Create an inverted index file for the given file
pub(crate) async fn generate_index_on_ingester(
    batches: Vec<RecordBatch>,
    new_file_key: String,
    org_id: &str,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    let mut data_buf: HashMap<String, SchemaRecords> = HashMap::new();

    let index_record_batches =
        prepare_index_record_batches(batches, org_id, stream_name, &new_file_key).await?;
    let record_batches: Vec<RecordBatch> = index_record_batches.into_iter().flatten().collect();
    if record_batches.is_empty() || record_batches.iter().all(|b| b.num_rows() == 0) {
        return Ok(());
    }
    let idx_schema: SchemaRef = record_batches.first().unwrap().schema();

    let mut schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let schema_chk = crate::service::schema::stream_schema_exists(
        org_id,
        stream_name,
        StreamType::Index,
        &mut schema_map,
    )
    .await;

    if !schema_chk.has_fields {
        db::schema::merge(
            org_id,
            stream_name,
            StreamType::Index,
            idx_schema.as_ref(),
            Some(Utc::now().timestamp_micros()),
        )
        .await
        .unwrap();
    }
    let schema_key = idx_schema.hash_key();
    let schema_key_str = schema_key.as_str();

    let json_rows = record_batches_to_json_rows(&record_batches.iter().collect::<Vec<_>>())?;
    if json_rows.is_empty() {
        return Ok(());
    }
    let recs: Vec<json::Value> = json_rows.into_iter().map(json::Value::Object).collect();
    for record_val in recs {
        let timestamp: i64 = record_val
            .get(&CONFIG.common.column_timestamp)
            .unwrap()
            .as_i64()
            .unwrap();

        let hour_key = crate::service::ingestion::get_wal_time_key(
            timestamp,
            &Vec::new(),
            PartitionTimeLevel::Hourly,
            &json::Map::new(),
            Some(schema_key_str),
        );

        let hour_buf = data_buf.entry(hour_key).or_insert_with(|| SchemaRecords {
            schema_key: schema_key.to_string(),
            schema: idx_schema.clone(),
            records: vec![],
            records_size: 0,
        });

        let record_size = json::estimate_json_bytes(&record_val);
        hour_buf.records.push(Arc::new(record_val));
        hour_buf.records_size += record_size;
    }
    let writer = ingester::get_writer(0, org_id, &StreamType::Index.to_string()).await;
    let _ = crate::service::ingestion::write_file(&writer, stream_name, data_buf).await;
    if let Err(e) = writer.sync().await {
        log::error!("ingestion error while syncing writer: {}", e);
    }
    log::info!("[INGESTER:JOB] Written index wal file successfully");
    Ok(())
}

/// Create an inverted index file for the given file
pub(crate) async fn generate_index_on_compactor(
    file_list_to_invalidate: &[FileKey],
    batches: Vec<RecordBatch>,
    new_file_key: String,
    org_id: &str,
    stream_name: &str,
) -> Result<(String, FileMeta), anyhow::Error> {
    let index_record_batches =
        prepare_index_record_batches(batches, org_id, stream_name, &new_file_key).await?;
    let mut record_batches: Vec<RecordBatch> = index_record_batches.into_iter().flatten().collect();
    if record_batches.is_empty() || record_batches.iter().all(|b| b.num_rows() == 0) {
        return Ok((String::new(), FileMeta::default()));
    }
    let schema = record_batches.first().unwrap().schema();

    let prefix_to_remove = format!("files/{}/logs/{}/", org_id, stream_name);
    let len_of_columns_to_invalidate = file_list_to_invalidate.len();
    let empty_terms: ArrayRef = Arc::new(StringArray::from(vec![
        None::<String>;
        len_of_columns_to_invalidate
    ]));
    let file_names: ArrayRef = Arc::new(StringArray::from(
        file_list_to_invalidate
            .iter()
            .map(|x| x.key.trim_start_matches(&prefix_to_remove).to_string())
            .collect::<Vec<String>>(),
    ));
    let _timestamp: ArrayRef = Arc::new(Int64Array::from(
        file_list_to_invalidate
            .iter()
            .map(|x| x.meta.min_ts)
            .collect::<Vec<i64>>(),
    ));
    let count: ArrayRef = Arc::new(Int64Array::from(vec![
        None::<i64>;
        len_of_columns_to_invalidate
    ]));
    let deleted: ArrayRef = Arc::new(BooleanArray::from(vec![true; len_of_columns_to_invalidate]));
    let columns = vec![empty_terms, file_names, _timestamp, count, deleted];
    let batch = RecordBatch::try_new(schema, columns).unwrap();
    record_batches.push(batch);

    let original_file_size = 0; // The file never existed before this function was called
    let (filename, filemeta, _stream_type) = write_to_disk(
        record_batches,
        original_file_size,
        org_id,
        stream_name,
        StreamType::Index,
        &new_file_key,
        "index_creator",
    )
    .await?;

    log::debug!("[COMPACTOR:JOB] Written index file successfully");
    Ok((filename, filemeta))
}

async fn prepare_index_record_batches(
    batches: Vec<RecordBatch>, // 10 fields index
    org_id: &str,
    stream_name: &str,
    new_file_key: &str,
) -> Result<Vec<Vec<RecordBatch>>, anyhow::Error> {
    if batches.is_empty() {
        return Ok(vec![]);
    }

    // filter null columns
    let schema = batches.first().unwrap().schema();
    let batches = batches.iter().collect::<Vec<_>>();
    let mut new_batch = arrow::compute::concat_batches(&schema, batches)?;
    let mut null_columns = 0;
    for i in 0..new_batch.num_columns() {
        let ni = i - null_columns;
        if new_batch.column(ni).null_count() == new_batch.num_rows() {
            new_batch.remove_column(ni);
            null_columns += 1;
        }
    }
    let schema = new_batch.schema();
    if schema.fields().is_empty()
        || (schema.fields().len() == 1 && schema.field(0).name() == &CONFIG.common.column_timestamp)
    {
        return Ok(vec![]);
    }

    let ctx = SessionContext::new();
    let provider = MemTable::try_new(schema.clone(), vec![vec![new_batch]])?;
    ctx.register_table("_tbl_raw_data", Arc::new(provider))?;
    let prefix_to_remove = format!("files/{}/logs/{}/", org_id, stream_name);
    let file_name_without_prefix = new_file_key.trim_start_matches(&prefix_to_remove);
    let mut indexed_record_batches_to_merge = Vec::new();
    for column in schema.fields().iter() {
        if column.data_type() != &DataType::Utf8 {
            continue;
        }

        // 1500 columns --> 100 columns of inverted index
        let index_df = ctx.table("_tbl_raw_data").await?;

        let column_name = column.name();
        let split_chars = &CONFIG.common.inverted_index_split_chars;
        let remove_chars_btrim = DEFAULT_INDEX_TRIM_CHARS;
        let lower_case_expr = lower(concat(vec![col(column_name), lit("")]));
        let split_arr = STRING_TO_ARRAY_V2_UDF.call(vec![lower_case_expr, lit(split_chars)]);
        let distinct_terms = array_distinct(split_arr);

        let record_batch = index_df
            .with_column("terms", distinct_terms)?
            .unnest_columns(&["terms"])?
            .with_column_renamed("terms", "term")?
            .with_column("term", btrim(vec![col("term"), lit(remove_chars_btrim)]))?
            .with_column("file_name", lit(file_name_without_prefix))?
            .aggregate(
                vec![col("term"), col("file_name")],
                vec![
                    min(col("_timestamp")).alias("_timestamp"),
                    count(col("term")).alias("_count"),
                ],
            )?
            .with_column("character_len", character_length(col("term")))?
            .with_column("deleted", lit(false))?
            .filter(col("character_len").gt_eq(lit(INDEX_MIN_CHAR_LEN as i32)))?
            .select_columns(&["term", "file_name", "_timestamp", "_count", "deleted"])?
            .collect()
            .await?;

        indexed_record_batches_to_merge.push(record_batch);
    }
    ctx.deregister_table("_tbl_raw_data")?;
    Ok(indexed_record_batches_to_merge)
}

#[allow(clippy::too_many_arguments)]
async fn move_single_file(
    thread_id: usize,
    trace_id: &str,
    file: &FileKey,
    stream_type: StreamType,
    stream_name: &str,
    buf: &mut Vec<u8>,
) -> datafusion::error::Result<(Arc<Schema>, Vec<RecordBatch>)> {
    let data = tmpfs::get(format!("{trace_id}/{}", &file.key).as_str()).map_err(|e| {
        log::error!(
            "[INGESTER:JOB:{thread_id}] merge small file: {}, err: {}",
            file.key,
            e
        );
        datafusion::error::DataFusionError::Execution(e.to_string())
    })?;

    // copy data to buf
    buf.extend_from_slice(&data);

    read_recordbatch_from_bytes(&data).await.map_err(|e| {
        log::error!(
            "[INGESTER:JOB:{thread_id}] read_recordbatch_from_bytes error for stream -> '{}/{}/{}'",
            trace_id,
            stream_type,
            stream_name
        );
        log::error!(
            "[INGESTER:JOB:{thread_id}] read recordbatch for file: {}, err: {}",
            file.key,
            e
        );
        datafusion::error::DataFusionError::Execution(e.to_string())
    })
}
