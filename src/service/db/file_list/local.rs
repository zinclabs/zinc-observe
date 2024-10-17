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

use std::io::{BufRead, BufReader};

use config::{
    meta::stream::{FileKey, FileMeta, StreamType},
    utils::{
        asynchronism::file::get_file_contents, file::scan_files, json,
        parquet::parse_file_key_columns,
    },
};
use infra::file_list as infra_file_list;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::common::{infra::wal, meta::stream::StreamParams};

/// use queue to batch send broadcast to other nodes
pub static BROADCAST_QUEUE: Lazy<RwLock<Vec<FileKey>>> =
    Lazy::new(|| RwLock::new(Vec::with_capacity(2048)));

pub async fn set(key: &str, meta: Option<FileMeta>, deleted: bool) -> Result<(), anyhow::Error> {
    let (_stream_key, date_key, _file_name) = parse_file_key_columns(key)?;
    let file_data = FileKey::new(key, meta.clone().unwrap_or_default(), deleted);

    // write into file_list storage
    // retry 5 times
    for _ in 0..5 {
        if let Err(e) = super::progress(key, meta.as_ref(), deleted).await {
            log::error!("[FILE_LIST] Error saving file to storage, retrying: {}", e);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        } else {
            break;
        }
    }

    let cfg = config::get_config();
    // write into local cache for s3
    if !cfg.common.meta_store_external {
        let mut write_buf = json::to_vec(&file_data)?;
        write_buf.push(b'\n');
        let file = wal::get_or_create(
            0,
            StreamParams::new("", "", StreamType::Filelist),
            None,
            &date_key,
        )
        .await;
        file.write(write_buf.as_ref()).await;
    }

    // notify other nodes
    if !cfg.common.meta_store_external || cfg.memory_cache.cache_latest_files {
        let mut q = BROADCAST_QUEUE.write().await;
        q.push(file_data);
    }

    Ok(())
}

/// need to load wal file_list when the ingester
async fn get_in_wal() -> Result<Vec<FileKey>, anyhow::Error> {
    let mut result = Vec::with_capacity(1024);
    let pattern = format!("{}file_list/", &config::get_config().common.data_wal_dir);
    let files = scan_files(&pattern, "json", None).unwrap_or_default();
    let mut line_no = 0;
    for file in files {
        line_no += 1;
        let data = get_file_contents(&file)
            .await
            .expect("open wal file list failed");
        let reader = BufReader::new(data.as_slice());
        // parse file list
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            let item: FileKey = match json::from_slice(line.as_bytes()) {
                Ok(item) => item,
                Err(err) => {
                    log::error!(
                        "parse wal file list failed:\nfile: {}\nline_no: {}\nline: {}\nerr: {}",
                        file,
                        line_no,
                        line,
                        err
                    );
                    continue;
                }
            };
            result.push(item);
        }
    }
    Ok(result)
}

pub async fn load_wal_in_cache() -> Result<(), anyhow::Error> {
    let files = get_in_wal().await?;
    infra_file_list::batch_add(&files).await?;
    Ok(())
}

pub async fn broadcast_cache(node_uuid: Option<String>) -> Result<(), anyhow::Error> {
    let files = infra_file_list::list()
        .await?
        .into_iter()
        .map(|(k, v)| FileKey::new(&k, v, false))
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Ok(());
    }
    for chunk in files.chunks(100) {
        if let Err(e) = super::broadcast::send(chunk, node_uuid.clone()).await {
            log::error!("[FILE_LIST] broadcast cached file list failed: {}", e);
        }
    }
    Ok(())
}
