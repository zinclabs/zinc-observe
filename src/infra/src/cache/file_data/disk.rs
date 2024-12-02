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
    cmp::{max, min},
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use async_recursion::async_recursion;
use bytes::Bytes;
use config::{
    get_config, is_local_disk_storage,
    meta::inverted_index::InvertedIndexTantivyMode,
    metrics,
    utils::{
        file::*,
        hash::{gxhash, Sum64},
    },
    RwAHashMap, FILE_EXT_TANTIVY, FILE_EXT_TANTIVY_FOLDER,
};
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use super::CacheStrategy;
use crate::{cache::meta::ResultCacheMeta, storage};

static FILES: Lazy<Vec<RwLock<FileData>>> = Lazy::new(|| {
    let cfg = get_config();
    let mut files = Vec::with_capacity(cfg.disk_cache.bucket_num);
    for _ in 0..cfg.disk_cache.bucket_num {
        files.push(RwLock::new(FileData::new(FileType::DATA)));
    }
    files
});

// read only
static FILES_READER: Lazy<Vec<FileData>> = Lazy::new(|| {
    let cfg = get_config();
    let mut files = Vec::with_capacity(cfg.disk_cache.bucket_num);
    for _ in 0..cfg.disk_cache.bucket_num {
        files.push(FileData::new(FileType::DATA));
    }
    files
});

static RESULT_FILES: Lazy<Vec<RwLock<FileData>>> = Lazy::new(|| {
    let cfg = get_config();
    let mut files = Vec::with_capacity(cfg.disk_cache.bucket_num);
    for _ in 0..cfg.disk_cache.bucket_num {
        files.push(RwLock::new(FileData::new(FileType::RESULT)));
    }
    files
});

// read only
static RESULT_FILES_READER: Lazy<Vec<FileData>> = Lazy::new(|| {
    let cfg = get_config();
    let mut files = Vec::with_capacity(cfg.disk_cache.bucket_num);
    for _ in 0..cfg.disk_cache.bucket_num {
        files.push(FileData::new(FileType::RESULT));
    }
    files
});

pub static QUERY_RESULT_CACHE: Lazy<RwAHashMap<String, Vec<ResultCacheMeta>>> =
    Lazy::new(Default::default);

pub struct FileData {
    max_size: usize,
    cur_size: usize,
    root_dir: String,
    multi_dir: Vec<String>,
    data: CacheStrategy,
}

#[derive(Debug)]
pub enum FileType {
    DATA,
    RESULT,
}

impl Default for FileData {
    fn default() -> Self {
        Self::new(FileType::DATA)
    }
}

impl FileData {
    fn new(file_type: FileType) -> FileData {
        let cfg = get_config();
        let size = match file_type {
            FileType::DATA => cfg.disk_cache.max_size,
            FileType::RESULT => cfg.disk_cache.result_max_size,
        };
        FileData::with_capacity_and_cache_strategy(size, &cfg.disk_cache.cache_strategy)
    }

    fn with_capacity_and_cache_strategy(max_size: usize, strategy: &str) -> FileData {
        let cfg = get_config();

        FileData {
            max_size,
            cur_size: 0,
            root_dir: format!(
                "{}{}",
                cfg.common.data_cache_dir,
                storage::format_key("", true),
            ),
            multi_dir: cfg
                .disk_cache
                .multi_dir
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string())
                .collect(),
            data: CacheStrategy::new(strategy),
        }
    }

    async fn exist(&self, file: &str) -> bool {
        self.data.contains_key(file)
    }

    async fn get(&self, file: &str, range: Option<Range<usize>>) -> Option<Bytes> {
        let file_path = format!("{}{}{}", self.root_dir, self.choose_multi_dir(file), file);
        tokio::task::spawn_blocking(move || match get_file_contents(&file_path, range) {
            Ok(data) => Some(Bytes::from(data)),
            Err(_) => None,
        })
        .await
        .ok()
        .flatten()
    }

    async fn get_size(&self, file: &str) -> Option<usize> {
        let file_path = format!("{}{}{}", self.root_dir, self.choose_multi_dir(file), file);
        match get_file_len(&file_path) {
            Ok(v) => Some(v as usize),
            Err(_) => None,
        }
    }

    async fn set(&mut self, trace_id: &str, file: &str, data: Bytes) -> Result<(), anyhow::Error> {
        let data_size = data.len();
        if self.cur_size + data_size >= self.max_size {
            log::info!(
                "[trace_id {trace_id}] File disk cache is full, can't cache extra {} bytes",
                data_size
            );
            // cache is full, need release some space
            let need_release_size = min(
                self.max_size,
                max(get_config().disk_cache.release_size, data_size * 100),
            );
            self.gc(trace_id, need_release_size).await?;
        }

        self.cur_size += data_size;
        self.data.insert(file.to_string(), data_size);
        // write file into local disk
        let file_path = format!("{}{}{}", self.root_dir, self.choose_multi_dir(file), file);
        fs::create_dir_all(Path::new(&file_path).parent().unwrap())?;
        put_file_contents(&file_path, &data)?;
        // metrics
        let columns = file.split('/').collect::<Vec<&str>>();
        if columns[0] == "files" {
            metrics::QUERY_DISK_CACHE_FILES
                .with_label_values(&[columns[1], columns[2]])
                .inc();
            metrics::QUERY_DISK_CACHE_USED_BYTES
                .with_label_values(&[columns[1], columns[2]])
                .add(data_size as i64);
        } else if columns[0] == "results" {
            metrics::QUERY_DISK_RESULT_CACHE_FILES
                .with_label_values(&[columns[1], columns[2]])
                .inc();
            metrics::QUERY_DISK_RESULT_CACHE_USED_BYTES
                .with_label_values(&[columns[1], columns[2]])
                .add(data_size as i64);
        };
        Ok(())
    }

    async fn gc(&mut self, trace_id: &str, need_release_size: usize) -> Result<(), anyhow::Error> {
        let cfg = get_config();
        log::info!(
            "[trace_id {trace_id}] File disk cache start gc {}/{}, need to release {} bytes",
            self.cur_size,
            self.max_size,
            need_release_size
        );
        let mut release_size = 0;
        let mut remove_result_files = vec![];
        loop {
            let item = self.data.remove();
            if item.is_none() {
                log::error!(
                    "[trace_id {trace_id}] File disk cache is corrupt, it shouldn't be none"
                );
                break;
            }
            let (key, data_size) = item.unwrap();
            // delete file from local disk
            let file_path = format!(
                "{}{}{}",
                self.root_dir,
                self.choose_multi_dir(key.as_str()),
                key
            );
            if let Err(e) = fs::remove_file(&file_path) {
                log::error!(
                    "[trace_id {trace_id}] File disk cache gc remove file: {}, error: {}",
                    file_path,
                    e
                );
            }

            // Handle for tantivy index
            if cfg.common.inverted_index_tantivy_mode == InvertedIndexTantivyMode::Mmap.to_string()
                && file_path.ends_with(FILE_EXT_TANTIVY)
            {
                let file_path = file_path.replace(FILE_EXT_TANTIVY, FILE_EXT_TANTIVY_FOLDER);
                if let Err(e) = fs::remove_dir_all(&file_path) {
                    log::error!(
                        "[trace_id {trace_id}] File disk cache gc remove file: {}, error: {}",
                        file_path,
                        e
                    );
                }
            }

            if key.starts_with("results/") {
                let columns = key.split('/').collect::<Vec<&str>>();
                let query_key = format!(
                    "{}_{}_{}_{}",
                    columns[1], columns[2], columns[3], columns[4]
                );
                remove_result_files.push(query_key);
            }
            // metrics
            let columns = key.split('/').collect::<Vec<&str>>();
            if columns[0] == "files" {
                metrics::QUERY_DISK_CACHE_FILES
                    .with_label_values(&[columns[1], columns[2]])
                    .dec();
                metrics::QUERY_DISK_CACHE_USED_BYTES
                    .with_label_values(&[columns[1], columns[2]])
                    .sub(data_size as i64);
            } else if columns[0] == "results" {
                metrics::QUERY_DISK_RESULT_CACHE_FILES
                    .with_label_values(&[columns[1], columns[2]])
                    .dec();
                metrics::QUERY_DISK_RESULT_CACHE_USED_BYTES
                    .with_label_values(&[columns[1], columns[2]])
                    .sub(data_size as i64);
            }
            release_size += data_size;
            if release_size >= need_release_size {
                break;
            }
        }
        self.cur_size -= release_size;

        if !remove_result_files.is_empty() {
            let mut r = QUERY_RESULT_CACHE.write().await;
            for query_key in remove_result_files {
                r.remove(&query_key);
            }
            drop(r);
        }
        log::info!(
            "[trace_id {trace_id}] File disk cache gc done, released {} bytes",
            release_size
        );

        Ok(())
    }

    async fn remove(&mut self, trace_id: &str, file: &str) -> Result<(), anyhow::Error> {
        log::debug!("[trace_id {trace_id}] File disk cache remove file {}", file);

        let Some((key, data_size)) = self.data.remove_key(file) else {
            return Ok(());
        };
        self.cur_size -= data_size;
        log::debug!(
            "[trace_id {trace_id}] File disk cache remove file done, released {} bytes",
            data_size
        );

        // delete file from local disk
        let file_path = format!(
            "{}{}{}",
            self.root_dir,
            self.choose_multi_dir(key.as_str()),
            key
        );
        if let Err(e) = fs::remove_file(&file_path) {
            log::error!(
                "[trace_id {trace_id}] File disk cache gc remove file: {}, error: {}",
                file_path,
                e
            );
        }

        // metrics
        let columns = key.split('/').collect::<Vec<&str>>();
        if columns[0] == "files" {
            metrics::QUERY_DISK_CACHE_FILES
                .with_label_values(&[columns[1], columns[2]])
                .dec();
            metrics::QUERY_DISK_CACHE_USED_BYTES
                .with_label_values(&[columns[1], columns[2]])
                .sub(data_size as i64);
        } else if columns[0] == "results" {
            metrics::QUERY_DISK_RESULT_CACHE_FILES
                .with_label_values(&[columns[1], columns[2]])
                .dec();
            metrics::QUERY_DISK_RESULT_CACHE_USED_BYTES
                .with_label_values(&[columns[1], columns[2]])
                .sub(data_size as i64);
        }

        Ok(())
    }

    fn choose_multi_dir(&self, file: &str) -> String {
        if self.multi_dir.is_empty() {
            return "".to_string();
        }

        let h = gxhash::new().sum64(file);
        let index = h % (self.multi_dir.len() as u64);
        format!("{}/", self.multi_dir.get(index as usize).unwrap())
    }

    fn size(&self) -> (usize, usize) {
        (self.max_size, self.cur_size)
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub async fn init() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    for file in FILES.iter() {
        let root_dir = file.read().await.root_dir.clone();
        std::fs::create_dir_all(&root_dir).expect("create cache dir success");
    }
    // trigger read only files
    for file in FILES_READER.iter() {
        let root_dir = file.root_dir.clone();
        std::fs::create_dir_all(&root_dir).expect("create cache dir success");
    }

    tokio::task::spawn(async move {
        log::info!("Loading disk cache start");
        let root_dir = FILES[0].read().await.root_dir.clone();
        let root_dir = Path::new(&root_dir).canonicalize().unwrap();
        if let Err(e) = load(&root_dir, &root_dir).await {
            log::error!("load disk cache error: {}", e);
        }
        log::info!(
            "Loading disk cache done, total files: {} ",
            len(FileType::DATA).await
        );
    });

    tokio::task::spawn(async move {
        if cfg.disk_cache.gc_interval == 0 {
            return;
        }
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(cfg.disk_cache.gc_interval));
        interval.tick().await; // the first tick is immediate
        loop {
            if let Err(e) = gc().await {
                log::error!("disk cache gc error: {}", e);
            }
            interval.tick().await;
        }
    });
    Ok(())
}

#[inline]
pub async fn get(file: &str, range: Option<Range<usize>>) -> Option<Bytes> {
    if !get_config().disk_cache.enabled {
        return None;
    }
    let idx = get_bucket_idx(file);
    let files = if file.starts_with("files") {
        FILES_READER.get(idx).unwrap()
    } else {
        RESULT_FILES_READER.get(idx).unwrap()
    };
    files.get(file, range).await
}

#[inline]
pub async fn get_size(file: &str) -> Option<usize> {
    if !get_config().disk_cache.enabled {
        return None;
    }
    let idx = get_bucket_idx(file);
    let files = if file.starts_with("files") {
        FILES_READER.get(idx).unwrap()
    } else {
        RESULT_FILES_READER.get(idx).unwrap()
    };
    files.get_size(file).await
}

#[inline]
pub async fn exist(file: &str) -> bool {
    if !get_config().disk_cache.enabled {
        return false;
    }
    let idx = get_bucket_idx(file);
    let files = if file.starts_with("files") {
        FILES[idx].read().await
    } else {
        RESULT_FILES[idx].read().await
    };
    files.exist(file).await
}

#[inline]
pub async fn set(trace_id: &str, file: &str, data: Bytes) -> Result<(), anyhow::Error> {
    let cfg = get_config();
    if !cfg.disk_cache.enabled || (!cfg.common.result_cache_enabled && is_local_disk_storage()) {
        return Ok(());
    }
    let idx = get_bucket_idx(file);
    let mut files = if file.starts_with("files") {
        FILES[idx].write().await
    } else {
        RESULT_FILES[idx].write().await
    };
    if files.exist(file).await {
        return Ok(());
    }
    files.set(trace_id, file, data).await
}

#[inline]
pub async fn remove(trace_id: &str, file: &str) -> Result<(), anyhow::Error> {
    if !get_config().disk_cache.enabled {
        return Ok(());
    }
    let idx = get_bucket_idx(file);
    let mut files = if file.starts_with("files") {
        FILES[idx].write().await
    } else {
        RESULT_FILES[idx].write().await
    };
    files.remove(trace_id, file).await
}

#[async_recursion]
async fn load(root_dir: &PathBuf, scan_dir: &PathBuf) -> Result<(), anyhow::Error> {
    let mut entries = tokio::fs::read_dir(&scan_dir).await?;
    let mut result_cache: HashMap<String, Vec<ResultCacheMeta>> = HashMap::new();
    loop {
        match entries.next_entry().await {
            Err(e) => return Err(e.into()),
            Ok(None) => break,
            Ok(Some(f)) => {
                let fp = match f.path().canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("canonicalize file path error: {}", e);
                        continue;
                    }
                };
                let ft = match f.file_type().await {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("get file type error: {}", e);
                        continue;
                    }
                };
                if ft.is_dir() {
                    if let Err(e) = load(root_dir, &fp).await {
                        log::error!("load disk cache error: {}", e);
                    }
                } else {
                    let meta = match get_file_meta(&fp) {
                        Ok(m) => m,
                        Err(e) => {
                            log::error!("get file meta error: {}", e);
                            continue;
                        }
                    };
                    let data_size = meta.len() as usize;
                    let mut file_key = fp
                        .strip_prefix(root_dir)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/");

                    if !get_config().disk_cache.multi_dir.is_empty() {
                        file_key = file_key.split('/').skip(1).collect::<Vec<_>>().join("/");
                    }
                    // check file already exists
                    if exist(&file_key).await {
                        continue;
                    }
                    // write into cache
                    let idx = get_bucket_idx(&file_key);
                    if file_key.starts_with("files") {
                        let mut w = FILES[idx].write().await;
                        w.cur_size += data_size;
                        w.data.insert(file_key.clone(), data_size);
                        let total = w.len();
                        drop(w);
                        // print progress
                        if total % 1000 == 0 {
                            log::info!("Loading disk cache {}", total);
                        }
                        // metrics
                        let columns = file_key.split('/').collect::<Vec<&str>>();

                        metrics::QUERY_DISK_CACHE_FILES
                            .with_label_values(&[columns[1], columns[2]])
                            .inc();
                        metrics::QUERY_DISK_CACHE_USED_BYTES
                            .with_label_values(&[columns[1], columns[2]])
                            .add(data_size as i64);
                    } else {
                        let mut w = RESULT_FILES[idx].write().await;
                        w.cur_size += data_size;
                        w.data.insert(file_key.clone(), data_size);
                        let total = w.len();
                        drop(w);
                        // print progress
                        if total % 1000 == 0 {
                            log::info!("Loading disk cache {}", total);
                        }
                        // metrics
                        let columns = file_key.split('/').collect::<Vec<&str>>();

                        metrics::QUERY_DISK_RESULT_CACHE_FILES
                            .with_label_values(&[columns[1], columns[2]])
                            .inc();
                        metrics::QUERY_DISK_RESULT_CACHE_USED_BYTES
                            .with_label_values(&[columns[1], columns[2]])
                            .add(data_size as i64);

                        let columns = file_key.split('/').collect::<Vec<&str>>();
                        if columns[0] == "results" {
                            let query_key = format!(
                                "{}_{}_{}_{}",
                                columns[1], columns[2], columns[3], columns[4]
                            );
                            let meta = columns[5].split('_').collect::<Vec<&str>>();
                            let is_aggregate = meta[2] == "1";
                            let is_descending = meta[3] == "1";
                            result_cache.entry(query_key).or_insert_with(Vec::new).push(
                                ResultCacheMeta {
                                    start_time: meta[0].parse().unwrap(),
                                    end_time: meta[1].parse().unwrap(),
                                    is_aggregate,
                                    is_descending,
                                },
                            );
                        };
                    }
                }
            }
        }
    }

    // write all data from result_cache to QUERY_RESULT_CACHE
    QUERY_RESULT_CACHE.write().await.extend(result_cache);
    Ok(())
}

async fn gc() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    if !cfg.disk_cache.enabled || is_local_disk_storage() {
        return Ok(());
    }
    for file in FILES.iter() {
        let r = file.read().await;
        if r.cur_size + cfg.disk_cache.release_size < r.max_size {
            drop(r);
            continue;
        }
        drop(r);
        let mut w = file.write().await;
        w.gc("global", cfg.disk_cache.gc_size).await?;
        drop(w);
    }
    for file in RESULT_FILES.iter() {
        let r = file.read().await;
        if r.cur_size + cfg.disk_cache.release_size < r.max_size {
            drop(r);
            continue;
        }
        drop(r);
        let mut w = file.write().await;
        w.gc("global", cfg.disk_cache.gc_size).await?;
        drop(w);
    }
    Ok(())
}

#[inline]
pub async fn stats(file_type: FileType) -> (usize, usize) {
    let mut total_size = 0;
    let mut used_size = 0;
    let files = match file_type {
        FileType::DATA => &FILES,
        FileType::RESULT => &RESULT_FILES,
    };

    for file in files.iter() {
        let r = file.read().await;
        let (max_size, cur_size) = r.size();
        total_size += max_size;
        used_size += cur_size;
    }
    (total_size, used_size)
}

#[inline]
pub async fn len(file_type: FileType) -> usize {
    let mut total = 0;
    let files = match file_type {
        FileType::DATA => &FILES,
        FileType::RESULT => &RESULT_FILES,
    };
    for file in files.iter() {
        let r = file.read().await;
        total += r.len();
    }
    total
}

#[inline]
pub async fn is_empty(file_type: FileType) -> bool {
    let files = match file_type {
        FileType::DATA => &FILES,
        FileType::RESULT => &RESULT_FILES,
    };

    for file in files.iter() {
        let r = file.read().await;
        if !r.is_empty() {
            return false;
        }
    }
    true
}
#[inline]
pub async fn get_dir() -> String {
    FILES[0].read().await.root_dir.clone()
}

pub async fn download(trace_id: &str, file: &str) -> Result<(), anyhow::Error> {
    let data = storage::get(file).await?;
    if data.is_empty() {
        return Err(anyhow::anyhow!("file {} data size is zero", file));
    }
    if let Err(e) = set(trace_id, file, data).await {
        return Err(anyhow::anyhow!(
            "set file {} to disk cache failed: {}",
            file,
            e
        ));
    };
    Ok(())
}

fn get_bucket_idx(file: &str) -> usize {
    let cfg = get_config();
    if cfg.disk_cache.bucket_num <= 1 {
        0
    } else {
        let h = gxhash::new().sum64(file);
        (h as usize) % cfg.disk_cache.bucket_num
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lru_cache_set_file() {
        let trace_id = "session_123";
        let mut file_data = FileData::with_capacity_and_cache_strategy(1024, "lru");
        let content = Bytes::from("Some text Need to store in cache");
        for i in 0..100 {
            let file_key = format!(
                "files/default/logs/olympics/2022/10/03/10/6982652937134804993_1_{}.parquet",
                i
            );
            let resp = file_data.set(trace_id, &file_key, content.clone()).await;
            assert!(resp.is_ok());
        }
    }

    #[tokio::test]
    async fn test_lru_cache_get_file() {
        let trace_id = "session_123";
        let mut file_data =
            FileData::with_capacity_and_cache_strategy(get_config().disk_cache.max_size, "lru");
        let file_key = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_2_1.parquet";
        let content = Bytes::from("Some text");

        file_data
            .set(trace_id, file_key, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key).await);

        file_data
            .set(trace_id, file_key, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key).await);
        assert!(file_data.size().0 > 0);
    }

    #[tokio::test]
    async fn test_lru_cache_miss() {
        let trace_id = "session_456";
        let mut file_data = FileData::with_capacity_and_cache_strategy(10, "lru");
        let file_key1 = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_3_1.parquet";
        let file_key2 = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_3_2.parquet";
        let content = Bytes::from("Some text");
        // set one key
        file_data
            .set(trace_id, file_key1, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key1).await);
        // set another key, will release first key
        file_data
            .set(trace_id, file_key2, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key2).await);
        // get first key, should get error
        assert!(!file_data.exist(file_key1).await);
    }

    #[tokio::test]
    async fn test_fifo_cache_set_file() {
        let trace_id = "session_123";
        let mut file_data = FileData::with_capacity_and_cache_strategy(1024, "fifo");
        let content = Bytes::from("Some text Need to store in cache");
        for i in 0..100 {
            let file_key = format!(
                "files/default/logs/olympics/2022/10/03/10/6982652937134804993_1_{}.parquet",
                i
            );
            let resp = file_data.set(trace_id, &file_key, content.clone()).await;
            assert!(resp.is_ok());
        }
    }

    #[tokio::test]
    async fn test_fifo_cache_get_file() {
        let trace_id = "session_123";
        let mut file_data =
            FileData::with_capacity_and_cache_strategy(get_config().disk_cache.max_size, "fifo");
        let file_key = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_2_1.parquet";
        let content = Bytes::from("Some text");

        file_data
            .set(trace_id, file_key, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key).await);

        file_data
            .set(trace_id, file_key, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key).await);
        assert!(file_data.size().0 > 0);
    }

    #[tokio::test]
    async fn test_fifo_cache_miss() {
        let trace_id = "session_456";
        let mut file_data = FileData::with_capacity_and_cache_strategy(10, "fifo");
        let file_key1 = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_3_1.parquet";
        let file_key2 = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_3_2.parquet";
        let content = Bytes::from("Some text");
        // set one key
        file_data
            .set(trace_id, file_key1, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key1).await);
        // set another key, will release first key
        file_data
            .set(trace_id, file_key2, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key2).await);
        // get first key, should get error
        assert!(!file_data.exist(file_key1).await);
    }

    #[tokio::test]
    async fn test_multi_dir() {
        let multi_dir: Vec<String> = "dir1 , dir2 , dir3"
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .collect();

        let trace_id = "session_123";
        let mut file_data =
            FileData::with_capacity_and_cache_strategy(get_config().disk_cache.max_size, "lru");
        file_data.multi_dir = multi_dir;
        let file_key = "files/default/logs/olympics/2022/10/03/10/6982652937134804993_2_1.parquet";
        let content = Bytes::from("Some text");

        file_data
            .set(trace_id, file_key, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key).await);

        file_data
            .set(trace_id, file_key, content.clone())
            .await
            .unwrap();
        assert!(file_data.exist(file_key).await);
        assert!(file_data.size().0 > 0);

        assert_eq!(file_data.get(&file_key, None).await, Some(content))
    }
}
