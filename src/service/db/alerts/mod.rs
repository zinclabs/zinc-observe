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

use config::{meta::stream::StreamType, utils::json};

use crate::{
    common::{infra::config::STREAM_ALERTS, meta::alerts::Alert},
    service::db,
};

pub mod destinations;
pub mod realtime_triggers;
pub mod templates;

pub async fn get(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<Option<Alert>, anyhow::Error> {
    let stream_key = format!("{org_id}/{stream_type}/{stream_name}");
    let value: Option<Alert> = if let Some(v) = STREAM_ALERTS.read().await.get(&stream_key) {
        v.iter().find(|x| x.name.eq(name)).cloned()
    } else {
        let key = format!("/alerts/{org_id}/{stream_type}/{stream_name}/{name}");
        match db::get(&key).await {
            Ok(val) => json::from_slice(&val)?,
            Err(_) => None,
        }
    };
    if value.is_none() { Ok(None) } else { Ok(value) }
}

pub async fn set(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    alert: &Alert,
    create: bool,
) -> Result<(), anyhow::Error> {
    let schedule_key = format!("{stream_type}/{stream_name}/{}", alert.name);
    let key = format!("/alerts/{org_id}/{}", &schedule_key);
    match db::put(
        &key,
        json::to_vec(alert).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await
    {
        Ok(_) => {
            let trigger = db::scheduler::Trigger {
                org: org_id.to_string(),
                module_key: schedule_key,
                next_run_at: chrono::Utc::now().timestamp_micros(),
                is_realtime: alert.is_real_time,
                is_silenced: false,
                ..Default::default()
            };
            if create {
                match db::scheduler::push(trigger).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to save trigger: {}", e);
                        Ok(())
                    }
                }
            } else {
                match db::scheduler::update_trigger(trigger).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to update trigger: {}", e);
                        Ok(())
                    }
                }
            }
        }
        Err(e) => Err(anyhow::anyhow!("Error save alert: {}", e)),
    }
}

pub async fn set_without_updating_trigger(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    alert: &Alert,
) -> Result<String, anyhow::Error> {
    let schedule_key = format!("{stream_type}/{stream_name}/{}", alert.name);
    let key = format!("/alerts/{org_id}/{}", &schedule_key);
    match db::put(
        &key,
        json::to_vec(alert).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await
    {
        Ok(_) => Ok(schedule_key),
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

pub async fn delete(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<(), anyhow::Error> {
    let schedule_key = format!("{stream_type}/{stream_name}/{name}");
    let key = format!("/alerts/{org_id}/{}", &schedule_key);
    match db::delete(&key, false, db::NEED_WATCH, None).await {
        Ok(_) => {
            match db::scheduler::delete(org_id, db::scheduler::TriggerModule::Alert, &schedule_key)
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => {
                    log::error!("Failed to delete trigger: {}", e);
                    Ok(())
                }
            }
        }
        Err(e) => Err(anyhow::anyhow!("Error deleting alert: {e}")),
    }
}

pub async fn list(
    org_id: &str,
    stream_type: Option<StreamType>,
    stream_name: Option<&str>,
) -> Result<Vec<Alert>, anyhow::Error> {
    let loc_stream_type = stream_type.unwrap_or_default();
    let key = match stream_name {
        Some(stream_name) => format!("/alerts/{org_id}/{loc_stream_type}/{stream_name}"),
        None => format!("/alerts/{org_id}/"),
    };
    let ret = db::list_values(&key).await?;
    let mut items: Vec<Alert> = Vec::with_capacity(ret.len());
    for item_value in ret {
        let json_val = json::from_slice(&item_value)?;
        items.push(json_val)
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/alerts/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching alerts");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alerts: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let stream_key = item_key[0..item_key.rfind('/').unwrap()].to_string();
                let item_value: Alert = if config::CONFIG.common.meta_store_external {
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
                let mut cacher = STREAM_ALERTS.write().await;
                let group = cacher.entry(stream_key.to_string()).or_default();
                if group.contains(&item_value) {
                    let idx = group.iter().position(|x| x.eq(&item_value)).unwrap();
                    let _ = std::mem::replace(&mut group[idx], item_value);
                } else {
                    group.push(item_value);
                }
                drop(cacher);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let stream_key = item_key[0..item_key.rfind('/').unwrap()].to_string();
                let alert_name = item_key[item_key.rfind('/').unwrap() + 1..].to_string();
                let mut cacher = STREAM_ALERTS.write().await;
                if stream_key.contains('/') {
                    let group = match cacher.get_mut(&stream_key) {
                        Some(v) => v,
                        None => continue,
                    };
                    group.retain(|v| !v.name.eq(&alert_name));
                } else {
                    cacher.remove(&stream_key);
                }
                drop(cacher);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/alerts/";
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let new_key = item_key.strip_prefix(key).unwrap();
        let alert: Alert = match json::from_slice(&item_value) {
            Ok(v) => v,
            Err(_) => {
                // HACK: for old version, write it back to up
                let data: json::Value = json::from_slice(&item_value).unwrap();
                let data = data.as_object().unwrap();
                let destinations = if let Some(dest) = data.get("destination") {
                    vec![dest.as_str().unwrap().to_string()]
                } else {
                    vec![]
                };
                let alert = Alert {
                    name: data.get("name").unwrap().as_str().unwrap().to_string(),
                    stream_type: data.get("stream_type").unwrap().as_str().unwrap().into(),
                    stream_name: match data.get("stream") {
                        Some(v) => v.as_str().unwrap().to_string(),
                        None => data
                            .get("stream_name")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string(),
                    },
                    destinations,
                    ..Default::default()
                };
                _ = db::put(
                    &item_key,
                    json::to_vec(&alert).unwrap().into(),
                    db::NO_NEED_WATCH,
                    None,
                )
                .await;
                alert
            }
        };
        let stream_key = &new_key[0..new_key.rfind('/').unwrap()];

        let mut cacher = STREAM_ALERTS.write().await;
        let group = cacher.entry(stream_key.to_string()).or_default();
        group.push(alert);
    }
    log::info!("Alerts Cached");
    Ok(())
}

pub async fn reset() -> Result<(), anyhow::Error> {
    let key = "/alerts/";
    Ok(db::delete(key, true, db::NO_NEED_WATCH, None).await?)
}
