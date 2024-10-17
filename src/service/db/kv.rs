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

use crate::{common::infra::config::KVS, service::db};

fn mk_keys(org_id: &str, key: &str) -> (String, String) {
    let cache_key = format!("{org_id}/{key}");
    let db_key = format!("/kv/{cache_key}");
    (cache_key, db_key)
}

pub async fn get(org_id: &str, key: &str) -> Result<bytes::Bytes, anyhow::Error> {
    let (cache_key, db_key) = mk_keys(org_id, key);
    if let Some(it) = KVS.get(&cache_key) {
        return Ok(it.value().clone());
    }
    let val = db::get(&db_key).await?;
    KVS.insert(cache_key, val.clone());
    Ok(val)
}

pub async fn set(org_id: &str, key: &str, val: bytes::Bytes) -> Result<(), anyhow::Error> {
    let (cache_key, db_key) = mk_keys(org_id, key);
    db::put(&db_key, val.clone(), db::NEED_WATCH, None).await?;
    KVS.insert(cache_key, val);
    Ok(())
}

pub async fn delete(org_id: &str, key: &str) -> Result<(), anyhow::Error> {
    let (cache_key, db_key) = mk_keys(org_id, key);
    KVS.remove(&cache_key);
    Ok(db::delete(&db_key, false, db::NEED_WATCH, None).await?)
}

pub async fn list(org_id: &str, prefix: &str) -> Result<Vec<String>, anyhow::Error> {
    let cache_key = if prefix.ends_with('*') {
        format!("{org_id}/{}", prefix.strip_suffix('*').unwrap())
    } else {
        format!("{org_id}/{prefix}")
    };
    let db_key = format!("/kv/{cache_key}");
    Ok(db::list_keys(&db_key)
        .await?
        .into_iter()
        .map(|it| it.strip_prefix(&format!("/kv/{org_id}/")).unwrap().into())
        .collect())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/kv/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching kv");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_kv: event channel closed");
                return Ok(());
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value = if config::get_config().common.meta_store_external {
                    match db::get(&ev.key).await {
                        Ok(val) => val,
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    }
                } else {
                    ev.value.unwrap()
                };
                KVS.insert(item_key.to_string(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                KVS.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
}
