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

use infra::db::Event;
use o2_enterprise::enterprise::cipher::CipherData;

use crate::{cipher::registry::REGISTRY, service::db};

// DBKey to set cipher keys
pub const CIPHER_KEY_PREFIX: &str = "/cipher_keys/";

pub async fn watch() -> Result<(), anyhow::Error> {
    let prefix = CIPHER_KEY_PREFIX;
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(prefix).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching cipher keys");

    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_cipher_keys: event channel closed");
                return Ok(());
            }
        };

        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(prefix).unwrap();
                let (org, name) = item_key.split_once("/").unwrap();
                let item = match infra::table::cipher::get_data(
                    org,
                    infra::table::cipher::EntryKind::CipherKey,
                    name,
                )
                .await
                {
                    Ok(Some(val)) => val,
                    Ok(None) => {
                        log::error!("unexpected missing cipher key");
                        continue;
                    }
                    Err(e) => {
                        log::error!("Error getting value: {}", e);
                        continue;
                    }
                };
                let cd: CipherData = serde_json::from_str(&item).unwrap();
                let kname = format!("{org}:{name}");
                let key = cd.get_key().await.unwrap();
                {
                    let mut lock = REGISTRY.write();
                    lock.add_key(kname, Box::new(key));
                    drop(lock);
                }
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(prefix).unwrap();
                let (org, name) = item_key.split_once("/").unwrap();
                let kname = format!("{org}:{name}");
                {
                    let mut lock = REGISTRY.write();
                    lock.remove_key(&kname);
                    drop(lock);
                }
            }
            Event::Empty => {}
        }
    }
}
