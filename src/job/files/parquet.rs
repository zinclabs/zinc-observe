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

use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::Arc,
    time::UNIX_EPOCH,
};

use anyhow::Context;
use arrow::{
    array::{
        new_null_array, ArrayRef, BinaryBuilder, BooleanArray, BooleanBuilder, Int64Array,
        Int64Builder, StringArray, StringBuilder,
    },
    datatypes::Field,
    record_batch::RecordBatch,
};
use arrow_schema::{DataType, Schema, SchemaRef};
use bytes::Bytes;
use chrono::{Duration, Utc};
use config::{
    cluster, get_config,
    meta::{
        bitvec::BitVec,
        inverted_index::{writer::ColumnIndexer, IndexFileMetas, InvertedIndexFormat},
        puffin::writer::PuffinBytesWriter,
        stream::{FileKey, FileMeta, PartitionTimeLevel, StreamSettings, StreamType},
    },
    metrics,
    utils::{
        arrow::record_batches_to_json_rows,
        asynchronism::file::{get_file_contents, get_file_meta},
        file::scan_files_with_channel,
        inverted_index::{convert_parquet_idx_file_name, split_token},
        json,
        parquet::{
            read_metadata_from_file, read_recordbatch_from_bytes, write_recordbatch_to_parquet,
        },
        schema_ext::SchemaExt,
    },
    FxIndexMap, INDEX_FIELD_NAME_FOR_ALL, INDEX_SEGMENT_LENGTH,
};
use hashbrown::HashSet;
use infra::{
    cache::tmpfs,
    schema::{
        get_stream_setting_bloom_filter_fields, get_stream_setting_fts_fields,
        get_stream_setting_index_fields, unwrap_stream_settings, SchemaCache,
    },
    storage,
};
use ingester::WAL_PARQUET_METADATA;
use once_cell::sync::Lazy;
use tokio::{
    sync::{Mutex, RwLock},
    time,
};

use crate::{
    common::{
        infra::wal,
        meta::{authz::Authz, stream::SchemaRecords},
    },
    job::files::idx::write_parquet_index_to_disk,
    service::{
        compact::merge::{generate_inverted_idx_recordbatch, merge_parquet_files},
        db,
        schema::generate_schema_for_defined_schema_fields,
        search::datafusion::exec::merge_parquet_files as merge_parquet_files_by_datafusion,
    },
};

static PROCESSING_FILES: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));
static SKIPPED_LOCK_FILES: Lazy<RwLock<HashSet<String>>> =
    Lazy::new(|| RwLock::new(HashSet::new()));

pub async fn run() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    let (tx, rx) =
        tokio::sync::mpsc::channel::<(String, Vec<FileKey>)>(cfg.limit.file_move_thread_num);
    let rx = Arc::new(Mutex::new(rx));
    // move files
    for thread_id in 0..cfg.limit.file_move_thread_num {
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
                        if let Err(e) = move_files(thread_id, &prefix, files).await {
                            log::error!("[INGESTER:JOB] Error moving parquet files to remote: {e}");
                        }
                    }
                }
            }
        });
    }

    // prepare files
    loop {
        if cluster::is_offline() {
            break;
        }
        time::sleep(time::Duration::from_secs(cfg.limit.file_push_interval)).await;
        if let Err(e) = scan_wal_files(tx.clone()).await {
            log::error!("[INGESTER:JOB] Error prepare parquet files: {}", e);
        }
    }
    log::info!("[INGESTER:JOB] job::files::parquet is stopped");
    Ok(())
}

async fn scan_wal_files(
    worker_tx: tokio::sync::mpsc::Sender<(String, Vec<FileKey>)>,
) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let wal_dir = Path::new(&cfg.common.data_wal_dir).canonicalize().unwrap();
    let pattern = wal_dir.join("files/");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<String>>(1);
    tokio::spawn(async move {
        if let Err(e) = scan_files_with_channel(
            pattern.as_path(),
            "parquet",
            Some(cfg.limit.file_push_limit),
            tx,
        )
        .await
        {
            if !e.to_string().contains("No such file or directory") {
                log::error!("[INGESTER:JOB] Failed to scan files: {}", e);
            }
        }
    });
    let mut files_num = 0;
    let mut last_time = start.elapsed().as_millis();
    loop {
        match rx.recv().await {
            None => {
                break;
            }
            Some(files) => {
                log::debug!(
                    "[INGESTER:JOB] scan files get batch: {}, took: {} ms",
                    files.len(),
                    start.elapsed().as_millis() - last_time
                );
                last_time = start.elapsed().as_millis();
                files_num += files.len();
                match prepare_files(files).await {
                    Err(e) => {
                        log::error!("[INGESTER:JOB] Error prepare parquet files: {}", e);
                    }
                    Ok(files) => {
                        for (prefix, files) in files.into_iter() {
                            if let Err(e) = worker_tx.send((prefix, files)).await {
                                log::error!(
                                    "[INGESTER:JOB] Error sending parquet files to move: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    if files_num > 0 {
        log::info!(
            "[INGESTER:JOB] scan files get total: {}, took: {} ms",
            files_num,
            start.elapsed().as_millis()
        );
    }
    Ok(())
}

async fn prepare_files(
    files: Vec<String>,
) -> Result<FxIndexMap<String, Vec<FileKey>>, anyhow::Error> {
    let cfg = get_config();
    let wal_dir = Path::new(&cfg.common.data_wal_dir).canonicalize().unwrap();

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
            // check if the file is still locking
            if SKIPPED_LOCK_FILES.read().await.contains(&file_key)
                && !wal::lock_files_exists(&file_key)
            {
                log::warn!(
                    "[INGESTER:JOB] the file was released, delete it: {}",
                    file_key
                );
                if tokio::fs::remove_file(&wal_dir.join(&file_key))
                    .await
                    .is_ok()
                {
                    // delete metadata from cache
                    WAL_PARQUET_METADATA.write().await.remove(&file_key);
                    // need release all the files
                    PROCESSING_FILES.write().await.remove(&file_key);
                    // delete from skip list
                    SKIPPED_LOCK_FILES.write().await.remove(&file_key);
                }
            }
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

    Ok(partition_files_with_size)
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

    let cfg = get_config();
    let wal_dir = Path::new(&cfg.common.data_wal_dir).canonicalize().unwrap();

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
    let stream_fields_num = latest_schema.fields().len();

    // check stream is existing
    if stream_fields_num == 0 {
        for file in files {
            log::warn!(
                "[INGESTER:JOB:{thread_id}] the stream [{}/{}/{}] was deleted, just delete file: {}",
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
            cfg.limit.max_file_size_on_disk as i64,
            cfg.compact.max_file_size as i64,
        )
        && (cfg.limit.file_move_fields_limit == 0
            || stream_fields_num < cfg.limit.file_move_fields_limit)
    {
        let mut has_expired_files = false;
        // not enough files to upload, check if some files are too old
        let min_ts = Utc::now().timestamp_micros()
            - Duration::try_seconds(cfg.limit.max_file_retention_time as i64)
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
        let (new_file_name, new_file_meta, new_file_list) =
            match merge_files(thread_id, latest_schema.clone(), &wal_dir, &files_with_size).await {
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
            let mut need_skip = true;
            // wait for 5s
            for _ in 0..50 {
                if wal::lock_files_exists(&file.key) {
                    log::warn!(
                        "[INGESTER:JOB:{thread_id}] the file is still in use, waiting for a few ms: {}",
                        file.key
                    );
                    time::sleep(time::Duration::from_millis(100)).await;
                } else {
                    need_skip = false;
                    break;
                }
            }
            if need_skip {
                log::warn!(
                    "[INGESTER:JOB:{thread_id}] the file is still in use, add it to the skip_list: {}",
                    file.key
                );
                SKIPPED_LOCK_FILES.write().await.insert(file.key.clone());
                continue;
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
    wal_dir: &Path,
    files_with_size: &[FileKey],
) -> Result<(String, FileMeta, Vec<FileKey>), anyhow::Error> {
    if files_with_size.is_empty() {
        return Ok((String::from(""), FileMeta::default(), Vec::new()));
    }

    let cfg = get_config();
    let mut new_file_size: i64 = 0;
    let mut new_compressed_file_size = 0;
    let mut new_file_list = Vec::new();
    let mut deleted_files = Vec::new();
    let mut min_ts = i64::MAX;
    let mut max_ts = i64::MIN;
    let mut total_records = 0;

    let stream_fields_num = latest_schema.fields().len();
    let max_file_size = std::cmp::min(
        cfg.limit.max_file_size_on_disk as i64,
        cfg.compact.max_file_size as i64,
    );
    for file in files_with_size.iter() {
        if new_file_size > 0
            && (new_file_size + file.meta.original_size > max_file_size
                || new_compressed_file_size + file.meta.compressed_size > max_file_size
                || (cfg.limit.file_move_fields_limit > 0
                    && stream_fields_num >= cfg.limit.file_move_fields_limit))
        {
            break;
        }
        new_file_size += file.meta.original_size;
        new_compressed_file_size += file.meta.compressed_size;
        new_file_list.push(file.clone());
    }
    let mut retain_file_list = new_file_list.clone();

    // write parquet files into tmpfs
    let tmp_dir = tmpfs::Directory::default();
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
    let stream_setting = infra::schema::get_settings(&org_id, &stream_name, stream_type).await;
    let bloom_filter_fields = get_stream_setting_bloom_filter_fields(&stream_setting);
    let full_text_search_fields = get_stream_setting_fts_fields(&stream_setting);
    let index_fields = get_stream_setting_index_fields(&stream_setting);
    let (defined_schema_fields, need_original) = match stream_setting {
        Some(s) => (
            s.defined_schema_fields.unwrap_or_default(),
            s.store_original_data,
        ),
        None => (Vec::new(), false),
    };
    let schema = if !defined_schema_fields.is_empty() {
        let latest_schema = SchemaCache::new(latest_schema.as_ref().clone());
        let latest_schema = generate_schema_for_defined_schema_fields(
            &latest_schema,
            &defined_schema_fields,
            need_original,
        );
        latest_schema.schema().clone()
    } else {
        latest_schema.clone()
    };

    let mut new_file_meta = FileMeta {
        min_ts,
        max_ts,
        records: total_records,
        original_size: new_file_size,
        compressed_size: 0,
        flattened: false,
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

    // upload file
    let buf = Bytes::from(buf);
    match storage::put(&new_file_key, buf).await {
        Ok(_) => {
            if cfg.common.inverted_index_enabled && stream_type.is_basic_type() {
                // generate inverted index RecordBatch
                if let Some(inverted_idx_batch) = generate_inverted_idx_recordbatch(
                    new_schema.clone(),
                    &new_batches,
                    stream_type,
                    &full_text_search_fields,
                    &index_fields,
                )? {
                    let index_format =
                        InvertedIndexFormat::from(&cfg.common.inverted_index_store_format);
                    if matches!(
                        index_format,
                        InvertedIndexFormat::Parquet | InvertedIndexFormat::Both
                    ) {
                        generate_index_on_ingester(
                            inverted_idx_batch.clone(),
                            new_file_key.clone(),
                            &org_id,
                            &stream_name,
                            stream_type,
                            &full_text_search_fields,
                            &index_fields,
                        )
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("generate_parquet_index_on_ingester error: {}", e)
                        })?;
                    }
                    if matches!(
                        index_format,
                        InvertedIndexFormat::FST | InvertedIndexFormat::Both
                    ) {
                        // generate fst inverted index and write to storage
                        generate_fst_inverted_index(
                            inverted_idx_batch,
                            &new_file_key,
                            &full_text_search_fields,
                            &index_fields,
                            None,
                        )
                        .await?;
                    }
                }
            }
            Ok((new_file_key, new_file_meta, retain_file_list))
        }
        Err(e) => Err(e),
    }
}

/// Create an inverted index file for the given file
#[allow(clippy::too_many_arguments)]
pub(crate) async fn generate_index_on_ingester(
    inverted_idx_batch: RecordBatch,
    new_file_key: String,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    full_text_search_fields: &[String],
    index_fields: &[String],
) -> Result<(), anyhow::Error> {
    let index_stream_name =
        if get_config().common.inverted_index_old_format && stream_type == StreamType::Logs {
            stream_name.to_string()
        } else {
            format!("{}_{}", stream_name, stream_type)
        };
    let record_batches = prepare_index_record_batches(
        inverted_idx_batch,
        org_id,
        stream_name,
        &new_file_key,
        full_text_search_fields,
        index_fields,
    )
    .map_err(|e| anyhow::anyhow!("prepare_index_record_batches error: {}", e))?;
    if record_batches.is_empty() || record_batches.iter().all(|b| b.num_rows() == 0) {
        return Ok(());
    }
    let idx_schema: SchemaRef = record_batches.first().unwrap().schema();

    let mut schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let schema_chk = crate::service::schema::stream_schema_exists(
        org_id,
        &index_stream_name,
        StreamType::Index,
        &mut schema_map,
    )
    .await;
    let mut stream_setting = schema_map
        .get(&index_stream_name)
        .and_then(|schema| unwrap_stream_settings(schema.schema()));

    if !schema_chk.has_fields {
        // create schema
        let Some((schema, _)) = db::schema::merge(
            org_id,
            &index_stream_name,
            StreamType::Index,
            idx_schema.as_ref(),
            Some(Utc::now().timestamp_micros()),
        )
        .await?
        else {
            return Err(anyhow::anyhow!(
                "generate_index_on_ingester create schema error: schema not found"
            ));
        };
        // update schema to enable bloomfilter for field: term
        let settings = StreamSettings {
            bloom_filter_fields: vec!["term".to_string()],
            ..Default::default()
        };
        let mut metadata = schema.metadata().clone();
        metadata.insert("settings".to_string(), json::to_string(&settings).unwrap());
        db::schema::update_setting(org_id, &index_stream_name, StreamType::Index, metadata).await?;
        // update stream setting
        stream_setting = Some(settings);

        crate::common::utils::auth::set_ownership(
            org_id,
            &StreamType::Index.to_string(),
            Authz::new(&index_stream_name),
        )
        .await;
    } else if let Some(schema) = schema_map.get(&index_stream_name) {
        // check if the schema has been updated <= v0.10.8-rc4
        if get_config().common.inverted_index_old_format
            && stream_type == StreamType::Logs
            && !schema.fields_map().contains_key("segment_ids")
        {
            if let Err(e) = db::schema::merge(
                org_id,
                &index_stream_name,
                StreamType::Index,
                idx_schema.as_ref(),
                Some(Utc::now().timestamp_micros()),
            )
            .await
            {
                return Err(anyhow::anyhow!(
                    "generate_index_on_ingester update schema error: {}",
                    e
                ));
            };
        }

        // TODO: disable it, because the prefix partition key will cause the file_list much bigger
        // add prefix partition for index <= v0.12.1
        // if let Some(settings) = stream_setting.as_mut() {
        //     let term_partition_exists = settings
        //         .partition_keys
        //         .iter()
        //         .any(|partition| partition.field == "term");
        //     if !term_partition_exists {
        //         settings
        //             .partition_keys
        //             .push(StreamPartition::new_prefix("term"));

        //         let mut metadata = schema.schema().metadata().clone();
        //         metadata.insert("settings".to_string(), json::to_string(&settings).unwrap());
        //         db::schema::update_setting(org_id, &index_stream_name, StreamType::Index,
        // metadata)             .await?;
        //     }
        // }
    }

    let schema_key = idx_schema.hash_key();
    let schema_key_str = schema_key.as_str();
    let stream_setting = stream_setting.unwrap_or_default();

    let json_rows = record_batches_to_json_rows(&record_batches.iter().collect::<Vec<_>>())?;
    if json_rows.is_empty() {
        return Ok(());
    }

    let mut data_buf: HashMap<String, SchemaRecords> = HashMap::new();
    for row in json_rows {
        let timestamp: i64 = row
            .get(&get_config().common.column_timestamp)
            .unwrap()
            .as_i64()
            .unwrap();

        let hour_key = crate::service::ingestion::get_write_partition_key(
            timestamp,
            &stream_setting.partition_keys,
            PartitionTimeLevel::Hourly,
            &row,
            Some(schema_key_str),
        );

        let hour_buf = data_buf.entry(hour_key).or_insert_with(|| SchemaRecords {
            schema_key: schema_key.to_string(),
            schema: idx_schema.clone(),
            records: vec![],
            records_size: 0,
        });
        let record_val: json::Value = json::Value::Object(row);
        let record_size = json::estimate_json_bytes(&record_val);
        hour_buf.records.push(Arc::new(record_val));
        hour_buf.records_size += record_size;
    }
    let writer = ingester::get_writer(
        0,
        org_id,
        &StreamType::Index.to_string(),
        &index_stream_name,
    )
    .await;
    let _ = crate::service::ingestion::write_file(&writer, &index_stream_name, data_buf).await;
    if let Err(e) = writer.sync().await {
        log::error!("ingestion error while syncing writer: {}", e);
    }
    log::info!("[INGESTER:JOB] Written index wal file successfully");
    Ok(())
}

/// Create an inverted index file for the given file
#[allow(clippy::too_many_arguments)]
pub(crate) async fn generate_index_on_compactor(
    file_list_to_invalidate: &[FileKey],
    inverted_idx_batch: RecordBatch,
    new_file_key: String,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    full_text_search_fields: &[String],
    index_fields: &[String],
) -> Result<Vec<(String, FileMeta)>, anyhow::Error> {
    let index_stream_name =
        if get_config().common.inverted_index_old_format && stream_type == StreamType::Logs {
            stream_name.to_string()
        } else {
            format!("{}_{}", stream_name, stream_type)
        };
    let mut record_batches = prepare_index_record_batches(
        inverted_idx_batch,
        org_id,
        stream_name,
        &new_file_key,
        full_text_search_fields,
        index_fields,
    )?;
    if record_batches.is_empty() || record_batches.iter().all(|b| b.num_rows() == 0) {
        return Ok(vec![(String::new(), FileMeta::default())]);
    }
    let schema = record_batches.first().unwrap().schema();

    let prefix_to_remove = format!("files/{}/logs/{}/", org_id, stream_name);
    let len_of_columns_to_invalidate = file_list_to_invalidate.len();

    let _timestamp: ArrayRef = Arc::new(Int64Array::from(
        file_list_to_invalidate
            .iter()
            .map(|x| x.meta.min_ts)
            .collect::<Vec<i64>>(),
    ));
    let min_ts: ArrayRef = Arc::new(Int64Array::from(
        file_list_to_invalidate
            .iter()
            .map(|x| x.meta.min_ts)
            .collect::<Vec<i64>>(),
    ));
    let max_ts: ArrayRef = Arc::new(Int64Array::from(
        file_list_to_invalidate
            .iter()
            .map(|x| x.meta.max_ts)
            .collect::<Vec<i64>>(),
    ));
    let empty_fields: ArrayRef = Arc::new(new_null_array(
        &DataType::Utf8,
        len_of_columns_to_invalidate,
    ));
    let empty_terms: ArrayRef = Arc::new(new_null_array(
        &DataType::Utf8,
        len_of_columns_to_invalidate,
    ));
    let file_names: ArrayRef = Arc::new(StringArray::from(
        file_list_to_invalidate
            .iter()
            .map(|x| x.key.trim_start_matches(&prefix_to_remove).to_string())
            .collect::<Vec<String>>(),
    ));
    let count: ArrayRef = Arc::new(Int64Array::from(vec![0; len_of_columns_to_invalidate]));
    let deleted: ArrayRef = Arc::new(BooleanArray::from(vec![true; len_of_columns_to_invalidate]));
    let empty_segment: ArrayRef = Arc::new(new_null_array(
        &DataType::Binary,
        len_of_columns_to_invalidate,
    ));
    let columns = vec![
        _timestamp,
        min_ts,
        max_ts,
        empty_fields,
        empty_terms,
        file_names,
        count,
        deleted,
        empty_segment,
    ];
    let batch = RecordBatch::try_new(schema, columns)
        .map_err(|e| anyhow::anyhow!("RecordBatch::try_new error: {}", e))?;
    record_batches.push(batch);

    let original_file_size = 0; // The file never existed before this function was called
    let files = write_parquet_index_to_disk(
        record_batches,
        original_file_size,
        org_id,
        &index_stream_name,
        StreamType::Index,
        &new_file_key,
        "index_creator",
    )
    .await?;

    log::debug!("[COMPACTOR:JOB] Written index files successfully");
    Ok(files)
}

fn prepare_index_record_batches(
    inverted_idx_batch: RecordBatch,
    org_id: &str,
    stream_name: &str,
    new_file_key: &str,
    full_text_search_fields: &[String],
    index_fields: &[String],
) -> Result<Vec<RecordBatch>, anyhow::Error> {
    let cfg = get_config();
    let schema = inverted_idx_batch.schema();

    let new_schema = Arc::new(Schema::new(vec![
        Field::new(cfg.common.column_timestamp.as_str(), DataType::Int64, false),
        Field::new("min_ts", DataType::Int64, true),
        Field::new("max_ts", DataType::Int64, true),
        Field::new("field", DataType::Utf8, true),
        Field::new("term", DataType::Utf8, true),
        Field::new("file_name", DataType::Utf8, false),
        Field::new("_count", DataType::Int64, false),
        Field::new("deleted", DataType::Boolean, false),
        Field::new("segment_ids", DataType::Binary, true), // bitmap
    ]));

    let prefix_to_remove = format!("files/{}/logs/{}/", org_id, stream_name);
    let file_name_without_prefix = new_file_key.trim_start_matches(&prefix_to_remove);
    let mut indexed_record_batches_to_merge = Vec::new();

    // get _timestamp column
    let Some(time_data) = inverted_idx_batch
        .column_by_name(&cfg.common.column_timestamp)
        .unwrap()
        .as_any()
        .downcast_ref::<Int64Array>()
    else {
        return Ok(vec![]);
    };

    let num_rows = inverted_idx_batch.num_rows();
    // process full text search fields
    for column in schema.fields().iter() {
        let column_name = column.name();
        if !full_text_search_fields.contains(column_name) || column.data_type() != &DataType::Utf8 {
            continue;
        }

        // get full text search column
        let Some(column_data) = inverted_idx_batch
            .column_by_name(column_name)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
        else {
            continue;
        };

        // split the column into terms
        let terms = (0..num_rows)
            .flat_map(|i| {
                split_token(column_data.value(i), &cfg.common.inverted_index_split_chars)
                    .into_iter()
                    .map(|s| (s, i))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if terms.is_empty() {
            continue;
        }

        // unique terms and get the min & max _timestamp
        let mut uniq_terms = BTreeMap::new();
        for (term, idx) in terms {
            let term_time = time_data.value(idx);
            let (min_ts, max_ts, ids) =
                uniq_terms
                    .entry(term.to_string())
                    .or_insert((term_time, term_time, Vec::new()));
            if *min_ts > term_time {
                *min_ts = term_time;
            }
            if *max_ts < term_time {
                *max_ts = term_time;
            }
            ids.push(idx);
        }

        // build record batch
        let records_len = uniq_terms.len();
        let mut field_timestamp = Int64Builder::with_capacity(records_len);
        let mut field_min_ts = Int64Builder::with_capacity(records_len);
        let mut field_max_ts = Int64Builder::with_capacity(records_len);
        let mut field_field =
            StringBuilder::with_capacity(records_len, INDEX_FIELD_NAME_FOR_ALL.len() * records_len);
        let mut field_term = StringBuilder::with_capacity(
            records_len,
            uniq_terms.iter().map(|x| x.0.len()).sum::<usize>(),
        );
        let mut field_file_name =
            StringBuilder::with_capacity(records_len, file_name_without_prefix.len() * records_len);
        let mut field_count = Int64Builder::with_capacity(records_len);
        let mut field_deleted = BooleanBuilder::with_capacity(records_len);
        let mut field_segment_ids = BinaryBuilder::with_capacity(records_len, records_len);
        for (term, (min_ts, max_ts, ids)) in uniq_terms {
            field_timestamp.append_value(min_ts);
            field_min_ts.append_value(min_ts);
            field_max_ts.append_value(max_ts);
            field_field.append_value(INDEX_FIELD_NAME_FOR_ALL);
            field_term.append_value(term);
            field_file_name.append_value(file_name_without_prefix);
            field_count.append_value(ids.len() as i64);
            field_deleted.append_value(false);
            // calculate segment ids
            let segment_ids = ids
                .iter()
                .map(|i| i / INDEX_SEGMENT_LENGTH)
                .collect::<HashSet<_>>();
            let segment_num = (num_rows + INDEX_SEGMENT_LENGTH - 1) / INDEX_SEGMENT_LENGTH;
            let mut bv = BitVec::with_capacity(segment_num);
            for i in 0..segment_num {
                bv.push(segment_ids.contains(&i));
            }
            field_segment_ids.append_value(bv.into_vec());
        }

        let record_batch = RecordBatch::try_new(
            new_schema.clone(),
            vec![
                Arc::new(field_timestamp.finish()),
                Arc::new(field_min_ts.finish()),
                Arc::new(field_max_ts.finish()),
                Arc::new(field_field.finish()),
                Arc::new(field_term.finish()),
                Arc::new(field_file_name.finish()),
                Arc::new(field_count.finish()),
                Arc::new(field_deleted.finish()),
                Arc::new(field_segment_ids.finish()),
            ],
        )
        .map_err(|e| anyhow::anyhow!("RecordBatch::try_new error: {}", e))?;
        indexed_record_batches_to_merge.push(record_batch);
    }

    // process index fields
    for column in schema.fields().iter() {
        let column_name = column.name();
        if !index_fields.contains(column_name) || column.data_type() != &DataType::Utf8 {
            continue;
        }

        // get index column
        let Some(column_data) = inverted_idx_batch
            .column_by_name(column_name)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
        else {
            continue;
        };

        // split the column into terms
        let terms = (0..num_rows)
            .map(|i| (column_data.value(i), i))
            .collect::<Vec<_>>();
        if terms.is_empty() {
            continue;
        }

        // unique terms and get the min & max _timestamp
        let mut uniq_terms = BTreeMap::new();
        for (term, idx) in terms {
            let term_time = time_data.value(idx);
            let (min_ts, max_ts, ids) =
                uniq_terms
                    .entry(term.to_string())
                    .or_insert((term_time, term_time, Vec::new()));
            if *min_ts > term_time {
                *min_ts = term_time;
            }
            if *max_ts < term_time {
                *max_ts = term_time;
            }
            ids.push(idx);
        }

        // build record batch
        let records_len = uniq_terms.len();
        let mut field_timestamp = Int64Builder::with_capacity(records_len);
        let mut field_min_ts = Int64Builder::with_capacity(records_len);
        let mut field_max_ts = Int64Builder::with_capacity(records_len);
        let mut field_field =
            StringBuilder::with_capacity(records_len, column_name.len() * records_len);
        let mut field_term = StringBuilder::with_capacity(
            records_len,
            uniq_terms.iter().map(|x| x.0.len()).sum::<usize>(),
        );
        let mut field_file_name =
            StringBuilder::with_capacity(records_len, file_name_without_prefix.len() * records_len);
        let mut field_count = Int64Builder::with_capacity(records_len);
        let mut field_deleted = BooleanBuilder::with_capacity(records_len);
        let mut field_segment_ids = BinaryBuilder::with_capacity(records_len, records_len);
        for (term, (min_ts, max_ts, ids)) in uniq_terms {
            field_timestamp.append_value(min_ts);
            field_min_ts.append_value(min_ts);
            field_max_ts.append_value(max_ts);
            field_field.append_value(column_name);
            field_term.append_value(term);
            field_file_name.append_value(file_name_without_prefix);
            field_count.append_value(ids.len() as i64);
            field_deleted.append_value(false);
            // calculate segment ids
            let segment_ids = ids
                .iter()
                .map(|i| i / INDEX_SEGMENT_LENGTH)
                .collect::<HashSet<_>>();
            let segment_num = (num_rows + INDEX_SEGMENT_LENGTH - 1) / INDEX_SEGMENT_LENGTH;
            let mut bv = BitVec::with_capacity(segment_num);
            for i in 0..segment_num {
                bv.push(segment_ids.contains(&i));
            }
            field_segment_ids.append_value(bv.into_vec());
        }

        let record_batch = RecordBatch::try_new(
            new_schema.clone(),
            vec![
                Arc::new(field_timestamp.finish()),
                Arc::new(field_min_ts.finish()),
                Arc::new(field_max_ts.finish()),
                Arc::new(field_field.finish()),
                Arc::new(field_term.finish()),
                Arc::new(field_file_name.finish()),
                Arc::new(field_count.finish()),
                Arc::new(field_deleted.finish()),
                Arc::new(field_segment_ids.finish()),
            ],
        )
        .map_err(|e| anyhow::anyhow!("RecordBatch::try_new error: {}", e))?;
        indexed_record_batches_to_merge.push(record_batch);
    }

    Ok(indexed_record_batches_to_merge)
}

/// Creates fst inverted index bytes and writes to storage.
/// Called by both ingester and compactor. Compactor needs to provide `file_list_to_invalidate`
/// to delete previously created small index files
pub(crate) async fn generate_fst_inverted_index(
    inverted_idx_batch: RecordBatch,
    parquet_file_name: &str,
    full_text_search_fields: &[String],
    index_fields: &[String],
    file_list_to_invalidate: Option<&[FileKey]>, /* for compactor to delete corresponding small
                                                  * .idx files */
) -> Result<(), anyhow::Error> {
    let Some((compressed_bytes, file_meta)) =
        prepare_fst_index_bytes(inverted_idx_batch, full_text_search_fields, index_fields)?
    else {
        log::info!("generate_fst_index_on_compactor creates empty index. skip");
        return Ok(());
    };

    // delete corresponding small .puffin files
    if let Some(file_list) = file_list_to_invalidate {
        for old_parquet_file in file_list {
            let Some(old_idx_file) = convert_parquet_idx_file_name(&old_parquet_file.key) else {
                continue;
            };
            if let Err(e) = storage::del(&[&old_idx_file]).await {
                log::info!(
                    "[COMPACTOR:JOB] Failed to remove merged fst idx file from disk: {}, {}",
                    old_idx_file,
                    e
                );
            }
        }
    }

    // write fst bytes into disk
    let Some(idx_file_name) = convert_parquet_idx_file_name(parquet_file_name) else {
        return Ok(());
    };
    let caller = if file_list_to_invalidate.is_some() {
        "[COMPACTOR:JOB]"
    } else {
        "[INGESTER:JOB]"
    };
    match storage::put(&idx_file_name, Bytes::from(compressed_bytes)).await {
        Ok(_) => {
            log::info!(
                "{} Written fst index file successfully compressed size {}, original size {}",
                caller,
                file_meta.compressed_size,
                file_meta.original_size,
            );
            Ok(())
        }
        Err(e) => {
            log::error!("{} Written fst index file error: {}", caller, e.to_string());
            Err(e)
        }
    }
}

/// Create and compressed inverted index bytes using FST solution for the given RecordBatch
pub(crate) fn prepare_fst_index_bytes(
    inverted_idx_batch: RecordBatch,
    full_text_search_fields: &[String],
    index_fields: &[String],
) -> Result<Option<(Vec<u8>, FileMeta)>, anyhow::Error> {
    let schema = inverted_idx_batch.schema();

    let mut writer = Vec::new();
    let mut index_file_metas = IndexFileMetas::new();

    let num_rows = inverted_idx_batch.num_rows();
    // Process full text search fields
    let mut ft_indexer = ColumnIndexer::new();
    for column in schema.fields() {
        let column_name = column.name();
        if !full_text_search_fields.contains(column_name) || column.data_type() != &DataType::Utf8 {
            continue;
        }

        let Some(column_data) = inverted_idx_batch
            .column_by_name(column_name)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
        else {
            continue;
        };

        // split the column into terms
        let terms = (0..num_rows)
            .flat_map(|i| {
                split_token(
                    column_data.value(i),
                    &config::get_config().common.inverted_index_split_chars,
                )
                .into_iter()
                .map(|s| (s, i))
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        if terms.is_empty() {
            continue;
        }

        for (term, row_id) in terms {
            let segment_id = row_id / INDEX_SEGMENT_LENGTH;
            ft_indexer.push(term.as_bytes(), segment_id, term.len());
        }
    }
    // finish ft_indexer
    if !ft_indexer.is_empty() {
        let ft_index_meta = ft_indexer
            .write(&mut writer)
            .context("Error constructing FST ColumnIndex for full text search fields")?;
        // add ft_indexer_meta to IndexFileMetas
        index_file_metas
            .metas
            .insert(INDEX_FIELD_NAME_FOR_ALL.to_string(), ft_index_meta);
    }

    // Process secondary index fields
    for column in schema.fields() {
        let column_name = column.name();
        if !index_fields.contains(column_name) || column.data_type() != &DataType::Utf8 {
            continue;
        }

        let Some(column_data) = inverted_idx_batch
            .column_by_name(column_name)
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
        else {
            continue;
        };

        let terms = (0..num_rows)
            .map(|i| (column_data.value(i), i))
            .collect::<Vec<_>>();
        if terms.is_empty() {
            continue;
        }

        let mut col_indexer = ColumnIndexer::new();
        for (term, row_id) in terms {
            let segment_id = row_id / INDEX_SEGMENT_LENGTH;
            col_indexer.push(term.as_bytes(), segment_id, term.len());
        }

        // finish col_indexer
        if !col_indexer.is_empty() {
            let col_index_meta = col_indexer.write(&mut writer).context(format!(
                "Error constructing FST ColumnIndex for field {}",
                column_name
            ))?;
            // add ft_indexer_meta to IndexFileMetas
            index_file_metas
                .metas
                .insert(column_name.to_string(), col_index_meta);
        }
    }

    // TODO(taiming): left for future improvement - write index files into file_list to show total
    // index size
    let mut file_meta = FileMeta {
        min_ts: 0,
        max_ts: 0,
        records: 0,
        original_size: 0,
        compressed_size: 0,
        flattened: false,
    };

    let _ = index_file_metas.finish(&mut writer)?;
    let original_size = writer.len();

    let mut puffin_buf: Vec<u8> = Vec::new();
    let mut puffin_writer = PuffinBytesWriter::new(&mut puffin_buf);
    puffin_writer.add_blob(writer)?;
    puffin_writer.finish()?;

    file_meta.original_size = original_size as _;
    file_meta.compressed_size = puffin_buf.len() as _;

    Ok(Some((puffin_buf, file_meta)))
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
