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

use std::sync::Arc;

use config::{
    utils::download_utils::{download_file, is_digest_different},
    MMDB_ASN_FILE_NAME, MMDB_CITY_FILE_NAME,
};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
use once_cell::sync::Lazy;
use tokio::{sync::Notify, time};

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

    #[cfg(feature = "enterprise")]
    if get_o2_config().common.enable_enterprise_mmdb {
        o2_enterprise::enterprise::mmdb::mmdb_downloader::run_download_files().await;
        let client = Lazy::get(&CLIENT_INITIALIZED);

        let fname = format!(
            "{}{}",
            &cfg.common.mmdb_data_dir,
            get_o2_config().common.mmdb_enterprise_file_name
        );

        if client.is_none() {
            update_global_maxmind_client(&fname).await;
            log::info!("Maxmind client initialized");
            Lazy::force(&MMDB_INIT_NOTIFIER).notify_one();
        } else {
            log::info!("New enterprise file found, updating client");
            update_global_maxmind_client(&fname).await;
        }

        Lazy::force(&CLIENT_INITIALIZED);
        return;
    }

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

            #[cfg(feature = "enterprise")]
            if get_o2_config().common.enable_enterprise_mmdb {
                let mut geoip = crate::common::infra::config::GEOIP_ENT_TABLE.write();
                *geoip = Some(
                    Geoip::new(GeoipConfig::new(
                        &get_o2_config().common.mmdb_enterprise_file_name,
                    ))
                    .unwrap(),
                );
                return;
            }

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
