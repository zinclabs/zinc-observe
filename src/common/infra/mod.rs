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

use ::config::{cache_instance_id, ider};
#[cfg(feature = "enterprise")]
use o2_openfga::config::get_config as get_openfga_config;

use crate::service::db::instance;

pub mod cluster;
pub mod config;
#[cfg(feature = "enterprise")]
pub mod ofga;
pub mod wal;

pub async fn init() -> Result<(), anyhow::Error> {
    // set instance id
    let instance_id = match instance::get().await {
        Ok(Some(instance)) => instance,
        Ok(None) | Err(_) => {
            let id = ider::generate();
            let _ = instance::set(&id).await;
            id
        }
    };
    cache_instance_id(&instance_id);

    wal::init()?;
    // because of asynchronous, we need to wait for a while
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}

#[cfg(feature = "enterprise")]
pub async fn init_openfga() -> Result<(), anyhow::Error> {
    crate::service::db::ofga::cache()
        .await
        .expect("ofga model cache failed");
    o2_openfga::authorizer::authz::init_open_fga().await;
    if get_openfga_config().enabled {
        if let Err(e) = ofga::init().await {
            log::error!("OFGA init failed: {}", e);
        }
    }
    Ok(())
}
