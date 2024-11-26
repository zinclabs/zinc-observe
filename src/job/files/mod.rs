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

use config::{
    cluster::{is_offline, LOCAL_NODE},
    ider,
    meta::stream::StreamType,
    FILE_EXT_PARQUET,
};

pub mod broadcast;
pub mod idx;
pub mod parquet;

pub async fn run() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_ingester() {
        return Ok(()); // not an ingester, no need to init job
    }

    tokio::task::spawn(async move { parquet::run().await });
    tokio::task::spawn(async move { broadcast::run().await });
    tokio::task::spawn(async move { clean_empty_dirs().await });

    Ok(())
}

async fn clean_empty_dirs() -> Result<(), anyhow::Error> {
    loop {
        if is_offline() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        let last_updated = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let root = format!("{}files/", config::get_config().common.data_wal_dir);
        if let Err(e) =
            config::utils::asynchronism::file::clean_empty_dirs(&root, Some(last_updated)).await
        {
            log::error!("clean_empty_dirs, err: {}", e);
        }
    }
    log::info!("job::files::clean_empty_dirs is stopped");
    Ok(())
}

pub fn generate_storage_file_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    wal_file_name: &str,
) -> String {
    // eg: 0/2023/08/21/08/8b8a5451bbe1c44b/ip=1234/7099303408192061440f3XQ2p.json
    let file_columns = wal_file_name.splitn(7, '/').collect::<Vec<&str>>();
    let stream_key = format!("{}/{}/{}", org_id, stream_type, stream_name);
    let file_date = format!(
        "{}/{}/{}/{}",
        file_columns[1], file_columns[2], file_columns[3], file_columns[4]
    );
    // let hash_id = file_columns[5].to_string();
    let file_name = file_columns.last().unwrap().to_string();
    let file_name_pos = file_name.rfind('/').unwrap_or_default();
    let id = ider::generate();
    let file_name = if file_name_pos == 0 {
        id
    } else {
        format!("{}/{}", &file_name[..file_name_pos], id)
    };
    format!("files/{stream_key}/{file_date}/{file_name}{FILE_EXT_PARQUET}")
}
