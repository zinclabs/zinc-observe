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
        new_null_array, Array, ArrayRef, BinaryBuilder, BooleanArray, BooleanBuilder, Int64Array,
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
        inverted_index::InvertedIndexFormat,
        puffin_directory::writer::PuffinDirWriter,
        search::StorageType,
        stream::{FileKey, FileMeta, PartitionTimeLevel, StreamSettings, StreamType},
    },
    metrics,
    utils::{
        arrow::record_batches_to_json_rows,
        asynchronism::file::get_file_meta,
        file::scan_files_with_channel,
        inverted_index::{convert_parquet_idx_file_name_to_tantivy_file, split_token},
        json,
        parquet::{
            get_recordbatch_reader_from_bytes, read_metadata_from_file, read_schema_from_file,
        },
        schema_ext::SchemaExt,
    },
    FxIndexMap, INDEX_FIELD_NAME_FOR_ALL, INDEX_SEGMENT_LENGTH, PARQUET_BATCH_SIZE,
};
use futures::TryStreamExt;
use hashbrown::HashSet;
use infra::{
    schema::{
        get_stream_setting_bloom_filter_fields, get_stream_setting_fts_fields,
        get_stream_setting_index_fields, unwrap_stream_settings, SchemaCache,
    },
    storage,
};
use ingester::WAL_PARQUET_METADATA;
use once_cell::sync::Lazy;
use parquet::arrow::async_reader::ParquetRecordBatchStream;
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
        db,
        schema::generate_schema_for_defined_schema_fields,
        search::datafusion::exec::{
            self, merge_parquet_files as merge_parquet_files_by_datafusion,
        },
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
    // let mut last_time = start.elapsed().as_millis();
    loop {
        match rx.recv().await {
            None => {
                break;
            }
            Some(files) => {
                // log::debug!(
                //     "[INGESTER:JOB] scan files get batch: {}, took: {} ms",
                //     files.len(),
                //     start.elapsed().as_millis() - last_time
                // );
                // last_time = start.elapsed().as_millis();
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
        log::debug!(
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
        // remove thread_id from prefix
        // eg: files/default/logs/olympics/0/2023/08/21/08/8b8a5451bbe1c44b/
        let mut columns = prefix.split('/').collect::<Vec<&str>>();
        columns.remove(4);
        let prefix = columns.join("/");
        let partition = partition_files_with_size.entry(prefix).or_default();
        partition.push(FileKey::new(&file_key, parquet_meta, false));
        // mark the file as processing
        // log::debug!("Processing files created: {:?}", file_key);
        PROCESSING_FILES.write().await.insert(file_key);
    }
    // log::debug!(
    //     "[INGESTER:JOB] move files get partitions: {}",
    //     partition_files_with_size.len()
    // );

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

    let columns = prefix.split('/').collect::<Vec<&str>>();
    // removed thread_id from prefix, so there is no thread_id in the path
    // eg: files/default/logs/olympics/2023/08/21/08/8b8a5451bbe1c44b/
    // eg: files/default/traces/default/2023/09/04/05/default/service_name=ingester/
    // let _ = columns[0].to_string(); // files/
    let org_id = columns[1].to_string();
    let stream_type = StreamType::from(columns[2]);
    let stream_name = columns[3].to_string();
    let prefix_date = format!("{}-{}-{}", columns[4], columns[5], columns[6]);

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

    // check data retention
    let stream_settings = infra::schema::get_settings(&org_id, &stream_name, stream_type)
        .await
        .unwrap_or_default();
    let mut stream_data_retention_days = cfg.compact.data_retention_days;
    if stream_settings.data_retention > 0 {
        stream_data_retention_days = stream_settings.data_retention;
    }
    if stream_data_retention_days > 0 {
        let date =
            config::utils::time::now() - Duration::try_days(stream_data_retention_days).unwrap();
        let stream_data_retention_end = date.format("%Y-%m-%d").to_string();
        if prefix_date < stream_data_retention_end {
            for file in files {
                log::warn!(
                    "[INGESTER:JOB:{thread_id}] the file [{}/{}/{}] was exceed the data retention, just delete file: {}",
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

    // log::debug!(
    //     "[INGESTER:JOB:{thread_id}] start merging for partition: {}",
    //     prefix
    // );

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
        log::info!("[INGESTER:JOB:{thread_id}] merge small file: {}", &file.key);
    }
    // no files need to merge
    if new_file_list.is_empty() {
        return Ok((String::from(""), FileMeta::default(), Vec::new()));
    }

    let retain_file_list = new_file_list.clone();

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

    // eg: files/default/logs/olympics/0/2023/08/21/08/8b8a5451bbe1c44b/7099303408192061440f3XQ2p.
    // parquet eg: files/default/traces/default/2/2023/09/04/05/default/service_name=ingester/
    // 7104328279989026816guOA4t.parquet let _ = columns[0].to_string(); // files/
    let file = new_file_list.first().unwrap();
    let columns = file.key.splitn(5, '/').collect::<Vec<&str>>();
    let org_id = columns[1].to_string();
    let stream_type = StreamType::from(columns[2]);
    let stream_name = columns[3].to_string();
    let file_name = columns[4].to_string();

    // get latest version of schema
    let stream_settings = infra::schema::get_settings(&org_id, &stream_name, stream_type).await;
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
    let _latest_schema = if !defined_schema_fields.is_empty() {
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

    // read schema from parquet file, there files have the same schema because they are under the
    // same prefix
    let schema = read_schema_from_file(&(&wal_dir.join(&file.key)).into()).await?;
    let schema_key = schema
        .as_ref()
        .clone()
        .with_metadata(Default::default())
        .hash_key();

    // generate datafusion tables
    let session = config::meta::search::Session {
        id: format!("ingester-{schema_key}"),
        storage_type: StorageType::Wal,
        work_group: None,
        target_partitions: 0,
    };
    let rules = hashbrown::HashMap::new();
    let table =
        exec::create_parquet_table(&session, schema.clone(), &new_file_list, rules, true, None)
            .await?;
    let tables = vec![table];

    let start = std::time::Instant::now();
    let merge_result = merge_parquet_files_by_datafusion(
        stream_type,
        &stream_name,
        schema,
        tables,
        &bloom_filter_fields,
        &new_file_meta,
    )
    .await;
    let (_new_schema, buf) = match merge_result {
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

    new_file_meta.compressed_size = buf.len() as i64;
    if new_file_meta.compressed_size == 0 {
        return Err(anyhow::anyhow!(
            "merge_parquet_files error: compressed_size is 0"
        ));
    }
    let new_file_key =
        super::generate_storage_file_name(&org_id, stream_type, &stream_name, &file_name);
    log::info!(
        "[INGESTER:JOB:{thread_id}] merge file successfully, {} files into a new file: {}, original_size: {}, compressed_size: {}, took: {} ms",
        retain_file_list.len(),
        new_file_key,
        new_file_meta.original_size,
        new_file_meta.compressed_size,
        start.elapsed().as_millis(),
    );

    // upload file
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
        generate_index_on_ingester(
            &new_file_key,
            &org_id,
            stream_type,
            &stream_name,
            &full_text_search_fields,
            &index_fields,
            schema,
            &mut reader,
        )
        .await
        .map_err(|e| anyhow::anyhow!("generate_parquet_index_on_ingester error: {}", e))?;
    }

    // generate tantivy inverted index and write to storage
    if matches!(
        index_format,
        InvertedIndexFormat::Tantivy | InvertedIndexFormat::Both
    ) {
        let (schema, mut reader) = get_recordbatch_reader_from_bytes(&buf).await?;
        let index_size = create_tantivy_index(
            "INGESTER",
            &new_file_key,
            &full_text_search_fields,
            &index_fields,
            schema,
            &mut reader,
        )
        .await
        .map_err(|e| anyhow::anyhow!("generate_tantivy_index_on_ingester error: {}", e))?;
        new_file_meta.index_size = index_size as i64;
    }

    Ok((new_file_key, new_file_meta, retain_file_list))
}

/// Create an inverted index file for the given file
#[allow(clippy::too_many_arguments)]
pub(crate) async fn generate_index_on_ingester(
    new_file_key: &str,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    full_text_search_fields: &[String],
    index_fields: &[String],
    schema: Arc<Schema>,
    reader: &mut ParquetRecordBatchStream<std::io::Cursor<Bytes>>,
) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();

    if full_text_search_fields.is_empty() && index_fields.is_empty() {
        return Ok(());
    }

    let index_stream_name =
        if get_config().common.inverted_index_old_format && stream_type == StreamType::Logs {
            stream_name.to_string()
        } else {
            format!("{}_{}", stream_name, stream_type)
        };
    let record_batches = prepare_index_record_batches(
        org_id,
        stream_type,
        stream_name,
        new_file_key,
        full_text_search_fields,
        index_fields,
        schema,
        reader,
    )
    .await?;
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

    log::info!(
        "[INGESTER:JOB] Written index wal file successfully, took: {} ms",
        start.elapsed().as_millis(),
    );

    Ok(())
}

/// Create an inverted index file for the given file
#[allow(clippy::too_many_arguments)]
pub(crate) async fn generate_index_on_compactor(
    file_list_to_invalidate: &[FileKey],
    new_file_key: &str,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    full_text_search_fields: &[String],
    index_fields: &[String],
    schema: Arc<Schema>,
    reader: &mut ParquetRecordBatchStream<std::io::Cursor<Bytes>>,
) -> Result<Vec<(String, FileMeta)>, anyhow::Error> {
    let start = std::time::Instant::now();

    if full_text_search_fields.is_empty() && index_fields.is_empty() {
        return Ok(vec![]);
    }

    let index_stream_name =
        if get_config().common.inverted_index_old_format && stream_type == StreamType::Logs {
            stream_name.to_string()
        } else {
            format!("{}_{}", stream_name, stream_type)
        };
    let mut record_batches = prepare_index_record_batches(
        org_id,
        stream_type,
        stream_name,
        new_file_key,
        full_text_search_fields,
        index_fields,
        schema,
        reader,
    )
    .await?;
    if record_batches.is_empty() || record_batches.iter().all(|b| b.num_rows() == 0) {
        return Ok(vec![]);
    }

    let schema = record_batches.first().unwrap().schema();
    let prefix_to_remove = format!("files/{}/{}/{}/", org_id, stream_type, stream_name);
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

    let files = write_parquet_index_to_disk(
        record_batches,
        org_id,
        StreamType::Index,
        &index_stream_name,
        new_file_key,
    )
    .await?;

    log::info!(
        "[COMPACT:JOB] generate index successfully, data file: {}, index files: {:?}, took: {} ms",
        new_file_key,
        files.iter().map(|(k, _)| k).collect::<Vec<_>>(),
        start.elapsed().as_millis(),
    );

    Ok(files)
}

#[allow(clippy::too_many_arguments)]
async fn prepare_index_record_batches(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    new_file_key: &str,
    full_text_search_fields: &[String],
    index_fields: &[String],
    schema: Arc<Schema>,
    reader: &mut ParquetRecordBatchStream<std::io::Cursor<Bytes>>,
) -> Result<Vec<RecordBatch>, anyhow::Error> {
    let cfg = get_config();
    let schema_fields = schema
        .fields()
        .iter()
        .map(|f| (f.name(), f))
        .collect::<HashMap<_, _>>();

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

    let mut total_num_rows = 0;
    let mut uniq_terms: HashMap<String, BTreeMap<String, _>> = HashMap::new();
    loop {
        let batch = reader.try_next().await?;
        let Some(batch) = batch else {
            break;
        };
        let num_rows = batch.num_rows();
        if num_rows == 0 {
            continue;
        }

        // update total_num_rows
        let prev_total_num_rows = total_num_rows;
        total_num_rows += num_rows;

        // get _timestamp column
        let Some(time_data) = batch
            .column_by_name(&cfg.common.column_timestamp)
            .unwrap()
            .as_any()
            .downcast_ref::<Int64Array>()
        else {
            continue;
        };

        // process full text search fields
        for column_name in full_text_search_fields.iter() {
            if !schema_fields.contains_key(column_name)
                || schema_fields.get(column_name).unwrap().data_type() != &DataType::Utf8
            {
                continue;
            }

            // get full text search column
            let Some(column_data) = batch
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
            let column_uniq_terms = uniq_terms
                .entry(INDEX_FIELD_NAME_FOR_ALL.to_string())
                .or_insert(BTreeMap::new());
            for (term, idx) in terms {
                let term_time = time_data.value(idx);
                let (min_ts, max_ts, ids) = column_uniq_terms.entry(term.to_string()).or_insert((
                    term_time,
                    term_time,
                    Vec::new(),
                ));
                if *min_ts > term_time {
                    *min_ts = term_time;
                }
                if *max_ts < term_time {
                    *max_ts = term_time;
                }
                ids.push(idx + prev_total_num_rows);
            }
        }

        // process index fields
        for column_name in index_fields.iter() {
            if !schema_fields.contains_key(column_name)
                || schema_fields.get(column_name).unwrap().data_type() != &DataType::Utf8
            {
                continue;
            }

            // get index column
            let Some(column_data) = batch
                .column_by_name(column_name)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
            else {
                continue;
            };

            // collect terms
            let terms = (0..num_rows)
                .map(|i| (column_data.value(i), i))
                .collect::<Vec<_>>();
            if terms.is_empty() {
                continue;
            }

            // unique terms and get the min & max _timestamp
            let column_uniq_terms = uniq_terms
                .entry(column_name.to_string())
                .or_insert(BTreeMap::new());
            for (term, idx) in terms {
                let term_time = time_data.value(idx);
                let (min_ts, max_ts, ids) = column_uniq_terms.entry(term.to_string()).or_insert((
                    term_time,
                    term_time,
                    Vec::new(),
                ));
                if *min_ts > term_time {
                    *min_ts = term_time;
                }
                if *max_ts < term_time {
                    *max_ts = term_time;
                }
                ids.push(idx + prev_total_num_rows);
            }
        }

        // yield, this operation is time-consuming
        tokio::task::yield_now().await;
    }

    // build record batch
    let prefix_to_remove = format!("files/{}/{}/{}/", org_id, stream_type, stream_name);
    let file_name_without_prefix = new_file_key.trim_start_matches(&prefix_to_remove);
    let mut indexed_record_batches_to_merge = Vec::new();
    for (column_name, column_uniq_terms) in uniq_terms {
        let records_len = column_uniq_terms.len();
        let batch_size = PARQUET_BATCH_SIZE.min(records_len);

        let mut field_timestamp = Int64Builder::with_capacity(batch_size);
        let mut field_min_ts = Int64Builder::with_capacity(batch_size);
        let mut field_max_ts = Int64Builder::with_capacity(batch_size);
        let mut field_field =
            StringBuilder::with_capacity(batch_size, column_name.len() * batch_size);
        let mut field_term = StringBuilder::with_capacity(
            batch_size,
            column_uniq_terms
                .iter()
                .take(batch_size)
                .map(|x| x.0.len())
                .sum::<usize>(),
        );
        let mut field_file_name =
            StringBuilder::with_capacity(batch_size, file_name_without_prefix.len() * batch_size);
        let mut field_count = Int64Builder::with_capacity(batch_size);
        let mut field_deleted = BooleanBuilder::with_capacity(batch_size);
        let mut field_segment_ids = BinaryBuilder::with_capacity(batch_size, batch_size);

        for (i, (term, (min_ts, max_ts, ids))) in column_uniq_terms.into_iter().enumerate() {
            field_timestamp.append_value(min_ts);
            field_min_ts.append_value(min_ts);
            field_max_ts.append_value(max_ts);
            field_field.append_value(&column_name);
            field_term.append_value(term);
            field_file_name.append_value(file_name_without_prefix);
            field_count.append_value(ids.len() as i64);
            field_deleted.append_value(false);
            // calculate segment ids
            let segment_ids = ids
                .iter()
                .map(|i| i / INDEX_SEGMENT_LENGTH)
                .collect::<HashSet<_>>();
            let segment_num = (total_num_rows + INDEX_SEGMENT_LENGTH - 1) / INDEX_SEGMENT_LENGTH;
            let mut bv = BitVec::with_capacity(segment_num);
            for i in 0..segment_num {
                bv.push(segment_ids.contains(&i));
            }
            field_segment_ids.append_value(bv.into_vec());

            // build record batch
            if i == records_len - 1 || i % batch_size == 0 {
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

                // yield, this operation is time-consuming
                tokio::task::yield_now().await;
            }
        }
    }

    Ok(indexed_record_batches_to_merge)
}

pub(crate) async fn create_tantivy_index(
    caller: &str,
    parquet_file_name: &str,
    full_text_search_fields: &[String],
    index_fields: &[String],
    schema: Arc<Schema>,
    reader: &mut ParquetRecordBatchStream<std::io::Cursor<Bytes>>,
) -> Result<usize, anyhow::Error> {
    let start = std::time::Instant::now();
    let caller = format!("[{caller}:JOB]");

    let dir = PuffinDirWriter::new();
    let _index = generate_tantivy_index(
        dir.clone(),
        reader,
        full_text_search_fields,
        index_fields,
        schema,
    )
    .await?;
    let puffin_bytes = dir.to_puffin_bytes()?;
    let index_size = puffin_bytes.len();

    // write fst bytes into disk
    let Some(idx_file_name) = convert_parquet_idx_file_name_to_tantivy_file(parquet_file_name)
    else {
        return Ok(0);
    };

    match storage::put(&idx_file_name, Bytes::from(puffin_bytes)).await {
        Ok(_) => {
            log::info!(
                "{} Written tantivy index file successfully: {}, index size {}, took: {} ms",
                caller,
                idx_file_name,
                index_size,
                start.elapsed().as_millis()
            );
        }
        Err(e) => {
            log::error!(
                "{} Written tantivy index file error: {}",
                caller,
                e.to_string()
            );
            return Err(e);
        }
    }

    Ok(index_size)
}

/// Create a tantivy index in the given directory for the record batch
pub(crate) async fn generate_tantivy_index<D: tantivy::Directory>(
    tantivy_dir: D,
    reader: &mut ParquetRecordBatchStream<std::io::Cursor<Bytes>>,
    full_text_search_fields: &[String],
    index_fields: &[String],
    schema: Arc<Schema>,
) -> Result<Option<tantivy::Index>, anyhow::Error> {
    let mut tantivy_schema_builder = tantivy::schema::SchemaBuilder::new();
    let schema_fields = schema
        .fields()
        .iter()
        .map(|f| (f.name(), f))
        .collect::<HashMap<_, _>>();

    // filter out fields that are not in schema & not of type Utf8
    let fts_fields = full_text_search_fields
        .iter()
        .filter(|f| {
            schema_fields
                .get(f)
                .map(|v| v.data_type() == &DataType::Utf8)
                .is_some()
        })
        .map(|f| f.to_string())
        .collect::<HashSet<_>>();
    let index_fields = index_fields
        .iter()
        .filter(|f| {
            schema_fields
                .get(f)
                .map(|v| v.data_type() == &DataType::Utf8)
                .is_some()
        })
        .map(|f| f.to_string())
        .collect::<HashSet<_>>();
    let tantivy_fields = fts_fields.union(&index_fields).collect::<HashSet<_>>();
    if tantivy_fields.is_empty() {
        return Ok(None);
    }

    // add fields to tantivy schema
    if !fts_fields.is_empty() {
        let fts_opts = tantivy::schema::TextOptions::default().set_indexing_options(
            tantivy::schema::TextFieldIndexing::default()
                .set_index_option(tantivy::schema::IndexRecordOption::Basic)
                .set_fieldnorms(false),
        );
        tantivy_schema_builder.add_text_field(INDEX_FIELD_NAME_FOR_ALL, fts_opts);
    }
    for field in index_fields.iter() {
        let index_opts = tantivy::schema::TextOptions::default().set_indexing_options(
            tantivy::schema::TextFieldIndexing::default()
                .set_index_option(tantivy::schema::IndexRecordOption::Basic)
                .set_tokenizer("raw")
                .set_fieldnorms(false),
        );
        tantivy_schema_builder.add_text_field(field, index_opts);
    }
    let tantivy_schema = tantivy_schema_builder.build();
    let fts_field = tantivy_schema.get_field(INDEX_FIELD_NAME_FOR_ALL).ok();

    let mut total_num_rows = 0;
    // docs per row to be added in the tantivy index
    let mut docs = BTreeMap::new();

    loop {
        let batch = reader.try_next().await?;
        let Some(inverted_idx_batch) = batch else {
            break;
        };
        let num_rows = inverted_idx_batch.num_rows();
        if num_rows == 0 {
            continue;
        }

        // update total_num_rows
        let prev_total_num_rows = total_num_rows;
        total_num_rows += num_rows;

        // process full text search fields
        for column_name in tantivy_fields.iter() {
            let column_data = match inverted_idx_batch
                .column_by_name(column_name)
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
            {
                Some(column_data) => column_data,
                None => {
                    // generate empty array to ensure the tantivy and parquet have same rows
                    &StringArray::from(vec![""; num_rows])
                }
            };

            // get field
            let field = match tantivy_schema.get_field(column_name) {
                Ok(f) => f,
                Err(_) => fts_field.unwrap(),
            };
            for i in 0..num_rows {
                let doc = docs
                    .entry(i + prev_total_num_rows)
                    .or_insert(tantivy::doc!());
                doc.add_text(field, column_data.value(i));
            }
        }
    }

    let mut index_writer = tantivy::IndexBuilder::new()
        .schema(tantivy_schema)
        .single_segment_index_writer(tantivy_dir, 50_000_000)
        .context("failed to create index builder")?;

    for (_, doc) in docs.into_iter() {
        if let Err(e) = index_writer.add_document(doc) {
            log::error!(
                "generate_tantivy_index: Failed to add document to index: {}",
                e
            );
            return Err(anyhow::anyhow!("Failed to add document to index: {}", e));
        }
    }

    let index = index_writer.finalize().map_err(|e| {
        log::error!(
            "generate_tantivy_index: Failed to finalize the index writer: {}",
            e
        );
        anyhow::anyhow!("Failed to finalize the index writer: {}", e)
    })?;

    Ok(Some(index))
}
