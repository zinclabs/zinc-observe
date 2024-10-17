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

use config::utils::json;
use itertools::Itertools;

use crate::{
    common::{infra::config::ALERTS_DESTINATIONS, meta::alerts::destinations::Destination},
    service::db,
};

pub async fn get(org_id: &str, name: &str) -> Result<Destination, anyhow::Error> {
    let map_key = format!("{org_id}/{name}");
    if let Some(val) = ALERTS_DESTINATIONS.get(&map_key) {
        return Ok(val.value().clone());
    }

    let key = format!("/destinations/{org_id}/{name}");
    let val = db::get(&key).await?;
    let dest: Destination = json::from_slice(&val)?;
    Ok(dest)
}

pub async fn set(org_id: &str, destination: &Destination) -> Result<(), anyhow::Error> {
    let key = format!("/destinations/{org_id}/{}", destination.name);
    Ok(db::put(
        &key,
        json::to_vec(destination).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await?)
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), anyhow::Error> {
    let key = format!("/destinations/{org_id}/{name}");
    Ok(db::delete(&key, false, db::NEED_WATCH, None).await?)
}

pub async fn list(org_id: &str) -> Result<Vec<Destination>, anyhow::Error> {
    let cache = ALERTS_DESTINATIONS.clone();
    if !cache.is_empty() {
        return Ok(cache
            .iter()
            .filter_map(|dest| {
                let k = dest.key();
                (k.starts_with(&format!("{org_id}/"))).then(|| dest.value().clone())
            })
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .collect());
    }

    let key = format!("/destinations/{org_id}/");
    let mut items: Vec<Destination> = Vec::new();
    for item_value in db::list_values(&key).await? {
        let dest: Destination = json::from_slice(&item_value)?;
        items.push(dest)
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/destinations/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching alert destinations");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alert_destinations: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value: Destination = if config::get_config().common.meta_store_external {
                    match db::get(&ev.key).await {
                        Ok(val) => match json::from_slice(&val) {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        },
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    }
                } else {
                    json::from_slice(&ev.value.unwrap()).unwrap()
                };
                ALERTS_DESTINATIONS.insert(item_key.to_owned(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                ALERTS_DESTINATIONS.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/destinations/";
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: Destination = json::from_slice(&item_value).unwrap();
        ALERTS_DESTINATIONS.insert(item_key.to_owned(), json_val);
    }
    log::info!("Alert destinations Cached");
    Ok(())
}
