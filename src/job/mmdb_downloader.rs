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

use std::{cmp::min, path::Path, sync::Arc};

use config::{MMDB_ASN_FILE_NAME, MMDB_CITY_FILE_NAME};
use futures::stream::StreamExt;
use once_cell::sync::Lazy;
use reqwest::Client;
use sha256::try_digest;
use tokio::{fs::File, io::AsyncWriteExt, sync::Notify, time};

use crate::{
    common::{
        infra::config::{GEOIP_ASN_TABLE, GEOIP_CITY_TABLE, MAXMIND_DB_CLIENT},
        meta::maxmind::MaxmindClient,
    },
    service::enrichment_table::geoip::{Geoip, GeoipConfig},
};

static CLIENT_INITIALIZED: Lazy<bool> = Lazy::new(|| true);
pub static MMDB_INIT_NOTIFIER: Lazy<Arc<Notify>> = Lazy::new(|| Arc::new(Notify::new()));

pub async fn run() -> Result<(), anyhow::Error> {
    let cfg = config::get_config();
    std::fs::create_dir_all(&cfg.common.mmdb_data_dir)?;
    // should run it every 24 hours
    let mut interval = time::interval(time::Duration::from_secs(cfg.common.mmdb_update_duration));

    loop {
        interval.tick().await;
        run_download_files().await;
    }
}

async fn run_download_files() {
    let cfg = config::get_config();

    // send request and await response
    let client = reqwest::Client::new();
    let city_fname = format!("{}{}", &cfg.common.mmdb_data_dir, MMDB_CITY_FILE_NAME);
    let asn_fname = format!("{}{}", &cfg.common.mmdb_data_dir, MMDB_ASN_FILE_NAME);

    let download_city_files =
        is_digest_different(&city_fname, &cfg.common.mmdb_geolite_citydb_sha256_url)
            .await
            .unwrap_or_else(|e| {
                log::error!("Error checking digest difference: {e}");
                false
            });

    let download_asn_files =
        is_digest_different(&asn_fname, &cfg.common.mmdb_geolite_asndb_sha256_url)
            .await
            .unwrap_or_else(|e| {
                log::error!("Error checking digest difference: {e}");
                false
            });

    if download_city_files {
        match download_file(&client, &cfg.common.mmdb_geolite_citydb_url, &city_fname).await {
            Ok(()) => {}
            Err(e) => log::error!("failed to download the files {}", e),
        }
    }

    if download_asn_files {
        match download_file(&client, &cfg.common.mmdb_geolite_asndb_url, &asn_fname).await {
            Ok(()) => {}
            Err(e) => log::error!("failed to download the files {}", e),
        }
    }

    let client = Lazy::get(&CLIENT_INITIALIZED);

    if client.is_none() {
        update_global_maxmind_client(&asn_fname).await;
        update_global_maxmind_client(&city_fname).await;
        log::info!("Maxmind client initialized");
        Lazy::force(&MMDB_INIT_NOTIFIER).notify_one();
    } else {
        if download_asn_files {
            log::info!("New asn file found, updating client");
            update_global_maxmind_client(&asn_fname).await;
        }

        if download_city_files {
            log::info!("New city file found, updating client");
            update_global_maxmind_client(&city_fname).await;
        }
    }

    Lazy::force(&CLIENT_INITIALIZED);
}

/// Update the global maxdb client object
pub async fn update_global_maxmind_client(fname: &str) {
    match MaxmindClient::new_with_path(fname) {
        Ok(maxminddb_client) => {
            let mut client = MAXMIND_DB_CLIENT.write().await;
            *client = Some(maxminddb_client);

            if fname.ends_with(MMDB_CITY_FILE_NAME) {
                let mut geoip_city = GEOIP_CITY_TABLE.write();
                *geoip_city = Some(Geoip::new(GeoipConfig::new(MMDB_CITY_FILE_NAME)).unwrap());
            } else {
                let mut geoip_asn = GEOIP_ASN_TABLE.write();
                *geoip_asn = Some(Geoip::new(GeoipConfig::new(MMDB_ASN_FILE_NAME)).unwrap());
            }
        }
        Err(e) => log::warn!(
            "Failed to create MaxmindClient with path: {}, {}",
            fname,
            e.to_string()
        ),
    }
}

pub async fn is_digest_different(
    local_file_path: &str,
    remote_sha256sum_path: &str,
) -> Result<bool, anyhow::Error> {
    let response = reqwest::get(remote_sha256sum_path).await?;
    let remote_file_sha = response.text().await?;
    let local_file_sha = try_digest(Path::new(local_file_path)).unwrap_or_default();
    Ok(remote_file_sha.trim() != local_file_sha.trim())
}

pub async fn download_file(client: &Client, url: &str, path: &str) -> Result<(), String> {
    // Reqwest setup
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    // download chunks
    let mut file = File::create(path)
        .await
        .or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err("Error while downloading file".to_string()))?;
        file.write_all(&chunk)
            .await
            .or(Err("Error while writing to file".to_string()))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
    }

    Ok(())
}
