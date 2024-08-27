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

use std::io::{BufRead, BufReader};

use bytes::Buf;
use chrono::{DateTime, Duration, TimeZone, Utc};
use config::utils::inverted_index::convert_parquet_idx_file_name;
use futures::future::try_join_all;
use hashbrown::HashMap;
use infra::{file_list as infra_file_list, storage};

pub async fn delete(
    org_id: &str,
    time_min: i64,
    time_max: i64,
    batch_size: i64,
) -> Result<i64, anyhow::Error> {
    let files = query_deleted(org_id, time_min, time_max, batch_size).await?;
    if files.is_empty() {
        return Ok(0);
    }
    let files_num = files.values().flatten().count() as i64;

    // delete files from storage
    if let Err(e) = storage::del(
        &files
            .values()
            .flatten()
            .map(|file| file.0.as_str())
            .collect::<Vec<_>>(),
    )
    .await
    {
        // maybe the file already deleted, so we just skip the `not found` error
        if !e.to_string().to_lowercase().contains("not found") {
            log::error!("[COMPACT] delete files from storage failed: {}", e);
            return Err(e);
        }
    }

    // delete related inverted index puffin files
    let inverted_index_files = files
        .values()
        .flatten()
        .filter_map(|file| convert_parquet_idx_file_name(&file.0))
        .collect::<Vec<_>>();
    if !inverted_index_files.is_empty() {
        if let Err(e) = storage::del(
            &inverted_index_files
                .iter()
                .map(|file| file.as_str())
                .collect::<Vec<_>>(),
        )
        .await
        {
            // maybe the file already deleted or there's not related index files,
            // so we just skip the `not found` error
            if !e.to_string().to_lowercase().contains("not found") {
                log::error!("[COMPACT] delete files from storage failed: {}", e);
                return Err(e);
            }
        }
    }

    // delete flattened files from storage
    let flattened_files = files
        .values()
        .flatten()
        .filter_map(|file| {
            if file.1 {
                Some(format!(
                    "files{}/{}",
                    config::get_config().common.column_all,
                    file.0.strip_prefix("files/").unwrap()
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if !flattened_files.is_empty() {
        if let Err(e) = storage::del(
            &flattened_files
                .iter()
                .map(|file| file.as_str())
                .collect::<Vec<_>>(),
        )
        .await
        {
            // maybe the file already deleted, so we just skip the `not found` error
            if !e.to_string().to_lowercase().contains("not found") {
                log::error!("[COMPACT] delete files from storage failed: {}", e);
                return Err(e);
            }
        }
    }

    // delete files from file_list_deleted s3
    if files.keys().len() > 1 || !files.contains_key("") {
        if let Err(e) =
            storage::del(&files.keys().map(|file| file.as_str()).collect::<Vec<_>>()).await
        {
            log::error!("[COMPACT] delete files from storage failed: {}", e);
            return Err(e);
        }
    }

    // delete files from file_list_deleted table
    if let Err(e) = infra_file_list::batch_remove_deleted(
        &files
            .values()
            .flatten()
            .map(|file| file.0.to_owned())
            .collect::<Vec<_>>(),
    )
    .await
    {
        log::error!("[COMPACT] delete files from table failed: {}", e);
        return Err(e.into());
    }

    Ok(files_num)
}

async fn query_deleted(
    org_id: &str,
    time_min: i64,
    time_max: i64,
    limit: i64,
) -> Result<HashMap<String, Vec<(String, bool)>>, anyhow::Error> {
    if config::get_config().common.meta_store_external {
        query_deleted_from_table(org_id, time_min, time_max, limit).await
    } else {
        query_deleted_from_s3(org_id, time_min, time_max, limit).await
    }
}

async fn query_deleted_from_table(
    org_id: &str,
    _time_min: i64,
    time_max: i64,
    limit: i64,
) -> Result<HashMap<String, Vec<(String, bool)>>, anyhow::Error> {
    let files = infra_file_list::query_deleted(org_id, time_max, limit).await?;
    let mut hash_files = HashMap::default();
    if !files.is_empty() {
        hash_files.insert("".to_string(), files);
    }
    Ok(hash_files)
}

async fn query_deleted_from_s3(
    org_id: &str,
    time_min: i64,
    time_max: i64,
    _limit: i64,
) -> Result<HashMap<String, Vec<(String, bool)>>, anyhow::Error> {
    let mut cur_time = if time_min != 0 {
        time_min
    } else {
        let prefix = format!("file_list_deleted/{org_id}/");
        let mut files = storage::list(&prefix).await?;
        if files.is_empty() {
            return Ok(HashMap::default());
        }
        files.sort();
        // file_list_deleted/{org_id}/{year}/{month}/{day}/{hour}/{file}.json.zst
        let file = files.first().unwrap();
        let columns = file.split('/').collect::<Vec<_>>();
        let time = Utc
            .with_ymd_and_hms(
                columns[2].parse::<i32>().unwrap(),
                columns[3].parse::<u32>().unwrap(),
                columns[4].parse::<u32>().unwrap(),
                columns[5].parse::<u32>().unwrap(),
                0,
                0,
            )
            .unwrap();
        time.timestamp_micros()
    };
    let mut files = HashMap::with_capacity(16);
    while cur_time <= time_max {
        let offset_time: DateTime<Utc> = Utc.timestamp_nanos(cur_time * 1000);
        let file_list_prefix = format!("{org_id}/{}", offset_time.format("%Y/%m/%d/%H/"));
        let prefix_files = load_prefix_from_s3(&file_list_prefix).await?;
        for (file, records) in prefix_files {
            let entry = files.entry(file).or_insert_with(Vec::new);
            entry.extend(records);
        }
        cur_time += Duration::try_hours(1).unwrap().num_microseconds().unwrap();
    }
    Ok(files)
}

pub async fn load_prefix_from_s3(
    prefix: &str,
) -> Result<HashMap<String, Vec<(String, bool)>>, anyhow::Error> {
    let prefix = format!("file_list_deleted/{prefix}/");
    let files = storage::list(&prefix).await?;
    let files_num = files.len();
    if files.is_empty() {
        return Ok(HashMap::default());
    }
    log::info!("Load file_list_deleted gets {} files", files_num);

    let cfg = config::get_config();
    let mut tasks = Vec::with_capacity(cfg.limit.query_thread_num + 1);
    let chunk_size = std::cmp::max(1, files_num / cfg.limit.query_thread_num);
    for chunk in files.chunks(chunk_size) {
        let chunk = chunk.to_vec();
        let task: tokio::task::JoinHandle<Result<HashMap<String, Vec<_>>, anyhow::Error>> =
            tokio::task::spawn(async move {
                let mut files = HashMap::with_capacity(chunk.len());
                for file in chunk {
                    match process_file(&file).await {
                        Err(err) => {
                            log::error!("Error processing file: {:?} {:?}", file, err);
                            continue;
                        }
                        Ok(records) => {
                            if records.is_empty() {
                                continue;
                            }
                            let entry = files.entry(file).or_insert_with(Vec::new);
                            entry.extend(records);
                        }
                    }
                    tokio::task::yield_now().await;
                }
                Ok(files)
            });
        tasks.push(task);
    }

    let mut files = HashMap::with_capacity(files_num);
    let task_results = try_join_all(tasks).await?;
    for task_result in task_results {
        for (file, records) in task_result? {
            let entry = files.entry(file).or_insert_with(Vec::new);
            entry.extend(records);
        }
    }
    Ok(files)
}

async fn process_file(file: &str) -> Result<Vec<(String, bool)>, anyhow::Error> {
    // download file list from storage
    let data = match storage::get(file).await {
        Ok(data) => data,
        Err(_) => {
            return Ok(Vec::new());
        }
    };

    // uncompress file
    let uncompress = zstd::decode_all(data.reader())?;
    let uncompress_reader = BufReader::new(uncompress.reader());
    // parse file list
    let mut records = Vec::with_capacity(16);
    for line in uncompress_reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        records.push((line, false));
    }

    Ok(records)
}
