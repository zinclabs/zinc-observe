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

use std::{fs, path::Path};

use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::stream::{StreamParams, StreamType},
    utils::file::scan_files,
};
use infra::storage;
use tokio::time;

use crate::{common::infra::wal, service::db};

pub async fn run() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    if cfg.common.local_mode || cfg.common.meta_store_external {
        return Ok(());
    }

    tokio::task::spawn(async move { run_move_file_to_s3().await });
    // tokio::task::spawn(async move { run_sync_s3_to_cache().await });

    Ok(())
}

pub async fn run_move_file_to_s3() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_ingester() {
        return Ok(()); // not an ingester, no need to init job
    }

    let mut interval = time::interval(time::Duration::from_secs(
        get_config().limit.file_push_interval,
    ));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = move_file_list_to_storage(true).await {
            log::error!("Error moving file_list to remote: {}", e);
        }
    }
}

// upload compressed file_list to storage & delete moved files from local
pub async fn move_file_list_to_storage(check_in_use: bool) -> Result<(), anyhow::Error> {
    let cfg = get_config();
    let data_dir = Path::new(&cfg.common.data_wal_dir).canonicalize().unwrap();

    let pattern = format!("{}file_list/", &cfg.common.data_wal_dir);
    let files = scan_files(&pattern, "json", None).unwrap_or_default();

    for file in files {
        let local_file = file.to_owned();
        let local_path = Path::new(&file).canonicalize().unwrap();
        let file_path = local_path
            .strip_prefix(&data_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let columns = file_path.splitn(2, '/').collect::<Vec<&str>>();

        // eg: file_list/0/2023/08/21/08/7099303408192061440f3XQ2p.json
        let mut file_name = columns[1].to_string();

        // Hack: compatible for <= 0.5.1
        if !file_name.contains('/') && file_name.contains('_') {
            file_name = file_name.replace('_', "/");
        }

        // check the file is using for write
        if check_in_use
            && wal::check_in_use(StreamParams::new("", "", StreamType::Filelist), &file_name).await
        {
            continue;
        }
        log::info!("[JOB] convert file_list: {}", file);
        match upload_file(&local_file, &file_name).await {
            Ok(_) => match fs::remove_file(&local_file) {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "[JOB] Failed to remove file_list {}: {}",
                        local_file,
                        e.to_string()
                    )
                }
            },
            Err(e) => log::error!("[JOB] Error while uploading file_list to storage {}", e),
        }
    }
    Ok(())
}

async fn upload_file(path_str: &str, file_key: &str) -> Result<(), anyhow::Error> {
    let mut file = fs::File::open(path_str).unwrap();
    let file_meta = file.metadata().unwrap();
    let file_size = file_meta.len();
    log::info!("[JOB] File_list upload begin: local: {}", path_str);
    if file_size == 0 {
        if let Err(e) = fs::remove_file(path_str) {
            log::error!("[JOB] File_list failed to remove: {}, {}", path_str, e);
        }
        return Err(anyhow::anyhow!("File_list is empty: {}", path_str));
    }

    let mut encoder = zstd::Encoder::new(Vec::new(), 3)?;
    std::io::copy(&mut file, &mut encoder)?;
    let compressed_bytes = encoder.finish().unwrap();

    let file_columns = file_key.split('/').collect::<Vec<&str>>();
    let new_file_key = format!(
        "file_list/{}/{}/{}/{}/{}.zst",
        file_columns[1], file_columns[2], file_columns[3], file_columns[4], file_columns[5]
    );

    let result = storage::put(&new_file_key, bytes::Bytes::from(compressed_bytes)).await;
    match result {
        Ok(_output) => {
            log::info!("[JOB] File_list upload succeeded: {}", new_file_key);
            Ok(())
        }
        Err(err) => {
            log::error!("[JOB] File_list upload error: {:?}", err);
            Err(anyhow::anyhow!(err))
        }
    }
}

async fn _run_sync_s3_to_cache() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_querier() && !LOCAL_NODE.is_compactor() {
        return Ok(()); // only querier or compactor need to sync
    }

    let mut interval = time::interval(time::Duration::from_secs(
        get_config().s3.sync_to_cache_interval,
    ));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = db::file_list::remote::cache_latest_hour().await {
            log::error!("[JOB] File_list sync s3 to cache error: {}", e);
        }
    }
}
