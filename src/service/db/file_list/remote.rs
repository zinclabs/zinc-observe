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
    collections::HashSet,
    io::{BufRead, BufReader},
    sync::atomic::{AtomicBool, Ordering},
};

use bytes::Buf;
use chrono::{DateTime, Duration, TimeZone, Utc};
use config::{meta::stream::FileKey, utils::json};
use futures::future::try_join_all;
use infra::{cache::stats, file_list as infra_file_list, storage};
use once_cell::sync::Lazy;
use tokio::{sync::RwLock, time};

use crate::service::db;

pub static LOADED_FILES: Lazy<RwLock<HashSet<String>>> =
    Lazy::new(|| RwLock::new(HashSet::with_capacity(24)));
pub static LOADED_ALL_FILES: AtomicBool = AtomicBool::new(false);

pub async fn cache(prefix: &str, force: bool) -> Result<(), anyhow::Error> {
    let prefix = format!("file_list/{prefix}");
    let mut rw = LOADED_FILES.write().await;
    if !force && rw.contains(&prefix) {
        return Ok(());
    }

    let start = std::time::Instant::now();
    let mut stats = ProcessStats::default();

    let mut files = storage::list(&prefix).await?;
    files.reverse(); // reverse order let deleted files be the first
    let files_num = files.len();
    log::info!("Load file_list [{prefix}] gets {} files", files_num);
    if files.is_empty() {
        // cache result
        rw.insert(prefix);
        return Ok(());
    }

    let cfg = config::get_config();
    let mut tasks = Vec::with_capacity(cfg.limit.query_thread_num + 1);
    let chunk_size = std::cmp::max(1, files_num / cfg.limit.query_thread_num);
    for chunk in files.chunks(chunk_size) {
        let chunk = chunk.to_vec();
        let task: tokio::task::JoinHandle<Result<ProcessStats, anyhow::Error>> =
            tokio::task::spawn(async move {
                let start = std::time::Instant::now();
                let mut stats = ProcessStats::default();
                for file in chunk {
                    match process_file(&file).await {
                        Err(err) => {
                            log::error!("Error processing file: {:?} {:?}", file, err);
                            continue;
                        }
                        Ok(files) => {
                            if files.is_empty() {
                                continue;
                            }
                            stats.file_count += files.len();
                            if let Err(e) = infra_file_list::batch_add(&files).await {
                                log::error!("Error sending file: {:?} {:?}", file, e);
                                continue;
                            }
                        }
                    }
                    tokio::task::yield_now().await;
                }
                stats.download_time = start.elapsed().as_millis() as usize;
                Ok(stats)
            });
        tasks.push(task);
    }

    let task_results = try_join_all(tasks).await?;
    for task_result in task_results {
        stats = stats + task_result?;
    }
    stats.caching_time = start.elapsed().as_millis() as usize;

    log::info!(
        "Load file_list [{prefix}] load {}:{} done, download: {}ms, caching: {}ms",
        files_num,
        stats.file_count,
        stats.download_time,
        stats.caching_time
    );

    // create table index
    infra_file_list::create_table_index().await?;
    infra_file_list::LOCAL_CACHE.create_table_index().await?;
    log::info!("Load file_list create table index done");

    // delete files
    let deleted_files = super::DELETED_FILES
        .iter()
        .map(|v| v.key().to_string())
        .collect::<Vec<String>>();
    infra_file_list::batch_remove(&deleted_files).await?;

    // cache result
    rw.insert(prefix.clone());
    log::info!("Load file_list [{prefix}] done deleting done");

    // clean depulicate files
    super::DEPULICATE_FILES.clear();
    super::DEPULICATE_FILES.shrink_to_fit();

    // clean deleted files
    super::DELETED_FILES.clear();
    super::DELETED_FILES.shrink_to_fit();
    Ok(())
}

pub async fn cache_stats() -> Result<(), anyhow::Error> {
    let orgs = db::schema::list_organizations_from_cache().await;
    for org_id in orgs {
        let ret = infra_file_list::get_stream_stats(&org_id, None, None).await;
        if ret.is_err() {
            log::error!("Load stream stats error: {}", ret.err().unwrap());
            continue;
        }
        for (stream, stats) in ret.unwrap() {
            let columns = stream.split('/').collect::<Vec<&str>>();
            let org_id = columns[0];
            let stream_type = columns[1];
            let stream_name = columns[2];
            stats::set_stream_stats(org_id, stream_name, stream_type.into(), stats);
        }
        time::sleep(time::Duration::from_millis(100)).await;
    }
    Ok(())
}

async fn process_file(file: &str) -> Result<Vec<FileKey>, anyhow::Error> {
    // download file list from storage
    let data = match storage::get(file).await {
        Ok(data) => data,
        Err(_) => {
            return Ok(vec![]);
        }
    };

    // uncompress file
    let uncompress = zstd::decode_all(data.reader())?;
    let uncompress_reader = BufReader::new(uncompress.reader());
    // parse file list
    let mut records = Vec::with_capacity(1024);
    for (line_no, line) in uncompress_reader.lines().enumerate() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let item: FileKey = match json::from_slice(line.as_bytes()) {
            Ok(item) => item,
            Err(err) => {
                log::error!(
                    "parse remote file list failed:\nfile: {}\nline_no: {}\nline: {}\nerr: {}",
                    file,
                    line_no,
                    line,
                    err
                );
                continue;
            }
        };
        // check backlist
        if !super::BLOCKED_ORGS.is_empty() {
            let columns = item.key.split('/').collect::<Vec<&str>>();
            let org_id = columns.get(1).unwrap_or(&"");
            if super::BLOCKED_ORGS.contains(&org_id.to_string()) {
                // log::error!("Load file_list skip blacklist org: {}", org_id);
                continue;
            }
        }
        // check deleted files
        if super::DELETED_FILES.contains_key(&item.key) {
            continue;
        }
        if item.deleted {
            super::DELETED_FILES.insert(item.key, item.meta.to_owned());
            continue;
        }
        // check duplicate files
        if config::get_config().common.feature_filelist_dedup_enabled {
            if super::DEPULICATE_FILES.contains(&item.key) {
                continue;
            }
            super::DEPULICATE_FILES.insert(item.key.to_string());
        }
        records.push(item);
    }

    Ok(records)
}

pub async fn cache_time_range(time_min: i64, time_max: i64) -> Result<(), anyhow::Error> {
    if time_min == 0 || time_max == 0 {
        return Ok(());
    }
    let mut cur_time = time_min;
    while cur_time <= time_max {
        let offset_time: DateTime<Utc> = Utc.timestamp_nanos(cur_time * 1000);
        let file_list_prefix = offset_time.format("%Y/%m/%d/%H/").to_string();
        cache(&file_list_prefix, false).await?;
        cur_time += Duration::try_hours(1).unwrap().num_microseconds().unwrap();
    }
    Ok(())
}

// cache by day: 2023-01-02
pub async fn cache_day(day: &str) -> Result<(), anyhow::Error> {
    let day_start = DateTime::parse_from_rfc3339(&format!("{day}T00:00:00Z"))?.with_timezone(&Utc);
    let day_end = DateTime::parse_from_rfc3339(&format!("{day}T23:59:59Z"))?.with_timezone(&Utc);
    let time_min = day_start.timestamp_micros();
    let time_max = day_end.timestamp_micros();
    cache_time_range(time_min, time_max).await
}

pub async fn cache_all() -> Result<(), anyhow::Error> {
    if LOADED_ALL_FILES.load(Ordering::Relaxed) {
        return Ok(());
    }
    let prefix = "file_list/".to_string();
    let files = storage::list(&prefix).await?;
    let mut prefixes = HashSet::new();
    for file in files {
        // file_list/2023/06/26/07/7078998136898850816tVckGD.json.zst
        let columns = file.split('/').collect::<Vec<_>>();
        let key = format!(
            "{}/{}/{}/{}/",
            columns[1], columns[2], columns[3], columns[4]
        );
        prefixes.insert(key);
    }
    for prefix in prefixes {
        cache(&prefix, false).await?;
    }
    LOADED_ALL_FILES.store(true, Ordering::Release);
    Ok(())
}

pub async fn cache_latest_hour() -> Result<(), anyhow::Error> {
    // for hourly
    let prefix = Utc::now().format("%Y/%m/%d/%H/").to_string();
    cache(&prefix, true).await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // for daily
    let prefix = Utc::now().format("%Y/%m/%d/00/").to_string();
    cache(&prefix, true).await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(())
}

#[derive(Debug, Clone, Default)]
struct ProcessStats {
    pub file_count: usize,
    pub download_time: usize,
    pub caching_time: usize,
}

impl std::ops::Add<ProcessStats> for ProcessStats {
    type Output = ProcessStats;

    fn add(self, rhs: ProcessStats) -> Self::Output {
        ProcessStats {
            file_count: self.file_count + rhs.file_count,
            download_time: self.download_time + rhs.download_time,
            caching_time: self.caching_time + rhs.caching_time,
        }
    }
}
