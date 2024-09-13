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

use std::sync::Arc;

use config::utils::json;

use crate::{
    common::{
        infra::config::{SYSLOG_ENABLED, SYSLOG_ROUTES},
        meta::syslog::SyslogRoute,
    },
    service::db,
};

#[tracing::instrument(name = "service:db:syslog:toggle_syslog_setting")]
pub async fn toggle_syslog_setting(enabled: bool) -> Result<(), anyhow::Error> {
    Ok(db::put(
        "/syslog/enabled",
        json::to_vec(&json::Value::Bool(enabled)).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await?)
}

#[tracing::instrument(name = "service:db:syslog:list")]
pub async fn list() -> Result<Vec<SyslogRoute>, anyhow::Error> {
    Ok(db::list("/syslog/route/")
        .await?
        .values()
        .map(|val| json::from_slice(val).unwrap())
        .collect())
}

#[tracing::instrument(name = "service:db:syslog:set", skip_all)]
pub async fn set(route: &SyslogRoute) -> Result<(), anyhow::Error> {
    Ok(db::put(
        &format!("/syslog/route/{}", route.id),
        json::to_vec(route).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await?)
}

#[tracing::instrument(name = "service:db:syslog:get")]
pub async fn get(id: &str) -> Result<SyslogRoute, anyhow::Error> {
    let val = db::get(&format!("/syslog/route/{id}")).await?;
    Ok(json::from_slice(&val).unwrap())
}

#[tracing::instrument(name = "service:db:syslog:delete")]
pub async fn delete(id: &str) -> Result<(), anyhow::Error> {
    Ok(db::delete(&format!("/syslog/route/{id}"), false, db::NEED_WATCH, None).await?)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/syslog/route/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching syslog routes");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_syslog: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_value: SyslogRoute = if config::get_config().common.meta_store_external {
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

                SYSLOG_ROUTES.insert(item_value.id.to_owned(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                SYSLOG_ROUTES.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/syslog/route/";
    let ret = db::list(key).await?;
    for (_, item_value) in ret {
        let json_val: SyslogRoute = json::from_slice(&item_value).unwrap();
        SYSLOG_ROUTES.insert(json_val.id.to_owned(), json_val);
    }
    log::info!("SyslogRoutes Cached");
    Ok(())
}

pub async fn watch_syslog_settings() -> Result<(), anyhow::Error> {
    let key = "/syslog/enabled";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching SyslogServer settings");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_syslog: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_value: bool = if config::get_config().common.meta_store_external {
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
                let mut syslog_enabled = SYSLOG_ENABLED.write();
                *syslog_enabled = item_value;
            }
            db::Event::Delete(_) => {
                let mut syslog_enabled = SYSLOG_ENABLED.write();
                *syslog_enabled = false;
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache_syslog_settings() -> Result<(), anyhow::Error> {
    let key = "/syslog/enabled";
    if let Ok(val) = db::get(key).await {
        let item_value: bool = json::from_slice(&val).unwrap();
        let mut syslog_enabled = SYSLOG_ENABLED.write();
        *syslog_enabled = item_value;
    }
    log::info!("SyslogServer settings Cached");
    Ok(())
}
