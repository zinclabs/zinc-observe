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

use crate::{
    common::infra::config::USER_SESSIONS,
    service::db::{self},
};

// DBKey to set settings for an org
pub const USER_SESSION_KEY: &str = "/user_sessions/";

pub async fn get(session_id: &str) -> Result<String, anyhow::Error> {
    match USER_SESSIONS.get(session_id) {
        Some(val) => Ok(val.to_string()),
        None => {
            let val = db::get(&format!("{USER_SESSION_KEY}{session_id}")).await?;

            Ok(json::to_string(&val).unwrap())
        }
    }
}

pub async fn set(session_id: &str, val: &str) -> Result<(), anyhow::Error> {
    db::put(
        &format!("{USER_SESSION_KEY}{session_id}"),
        json::to_vec(&val).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await?;
    Ok(())
}

pub async fn delete(session_id: &str) -> Result<(), anyhow::Error> {
    Ok(db::delete(
        &format!("{USER_SESSION_KEY}{session_id}"),
        false,
        db::NEED_WATCH,
        None,
    )
    .await?)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = USER_SESSION_KEY;
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching user sessions");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_user: event channel closed");
                return Ok(());
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value = if config::get_config().common.meta_store_external {
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
                USER_SESSIONS.insert(item_key.to_string(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                USER_SESSIONS.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = USER_SESSION_KEY;
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let session_id = item_key.strip_prefix(key).unwrap();
        let json_val: String = json::from_slice(&item_value).unwrap();
        USER_SESSIONS.insert(session_id.to_owned(), json_val);
    }
    log::info!("User Sessions Cached");
    Ok(())
}
