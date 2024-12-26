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
    meta::{
        alerts::alert::{Alert, ListAlertsParams},
        stream::StreamType,
    },
    utils::json,
};
use infra::{
    db::{connect_to_orm, ORM_CLIENT},
    table::alerts as table,
};
use itertools::Itertools;
use sea_orm::{ConnectionTrait, TransactionTrait};
use svix_ksuid::Ksuid;

use crate::{common::infra::config::STREAM_ALERTS, service::db};

pub async fn get_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<Option<Alert>, infra::errors::Error> {
    // We cannot check the cache because the cache stores alerts by stream type
    // and stream name which are currently unknown.
    let alert = table::get_by_id(conn, org_id, alert_id)
        .await?
        .map(|(_f, a)| a);
    Ok(alert)
}

pub async fn get_by_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<Option<Alert>, infra::errors::Error> {
    let stream_key = cache_stream_key(org_id, stream_type, stream_name);
    let mut value: Option<Alert> = if let Some(v) = STREAM_ALERTS.read().await.get(&stream_key) {
        v.iter().find(|x| x.name.eq(name)).cloned()
    } else {
        None
    };
    if value.is_none() {
        let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
        let alert =
            table::get_by_name(client, org_id, "default", stream_type, stream_name, name).await?;
        value = alert.map(|(_f, a)| a);
    }
    Ok(value)
}

pub async fn set(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    folder_id: &str,
    alert: Alert,
    create: bool,
) -> Result<(), infra::errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match table::put(client, org_id, folder_id, alert).await {
        Ok(alert) => {
            cluster::emit_put_event(org_id, &alert).await?;
            #[cfg(feature = "enterprise")]
            super_cluster::emit_put_event(org_id, &alert).await?;

            let schedule_key = scheduler_key(stream_type, stream_name, &alert.name);
            let trigger = db::scheduler::Trigger {
                org: org_id.to_string(),
                module_key: schedule_key.clone(),
                next_run_at: chrono::Utc::now().timestamp_micros(),
                is_realtime: alert.is_real_time,
                is_silenced: false,
                ..Default::default()
            };

            if create {
                match db::scheduler::push(trigger).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
                        Ok(())
                    }
                }
            } else if db::scheduler::exists(
                org_id,
                db::scheduler::TriggerModule::Alert,
                &schedule_key,
            )
            .await
            {
                match db::scheduler::update_trigger(trigger).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to update trigger for alert {schedule_key}: {}", e);
                        Ok(())
                    }
                }
            } else {
                match db::scheduler::push(trigger).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
                        Ok(())
                    }
                }
            }
        }
        Err(e) => Err(e),
    }
}

pub async fn create<C: TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    alert: Alert,
) -> Result<Alert, infra::errors::Error> {
    let alert = table::create(conn, org_id, folder_id, alert).await?;

    cluster::emit_put_event(org_id, &alert).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_put_event(org_id, &alert).await?;

    let schedule_key = scheduler_key(alert.stream_type, &alert.stream_name, &alert.name);
    let trigger = db::scheduler::Trigger {
        org: org_id.to_string(),
        module_key: schedule_key.clone(),
        next_run_at: chrono::Utc::now().timestamp_micros(),
        is_realtime: alert.is_real_time,
        is_silenced: false,
        ..Default::default()
    };

    let _ = db::scheduler::push(trigger).await.map_err(|e| {
        log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
        e
    });

    Ok(alert)
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: Option<&str>,
    alert: Alert,
) -> Result<Alert, infra::errors::Error> {
    let alert = table::update(conn, org_id, folder_id, alert).await?;

    cluster::emit_put_event(org_id, &alert).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_put_event(org_id, &alert).await?;

    let schedule_key = scheduler_key(alert.stream_type, &alert.stream_name, &alert.name);
    let trigger = db::scheduler::Trigger {
        org: org_id.to_string(),
        module_key: schedule_key.clone(),
        next_run_at: chrono::Utc::now().timestamp_micros(),
        is_realtime: alert.is_real_time,
        is_silenced: false,
        ..Default::default()
    };

    if db::scheduler::exists(org_id, db::scheduler::TriggerModule::Alert, &schedule_key).await {
        let _ = db::scheduler::update_trigger(trigger).await.map_err(|e| {
            log::error!("Failed to update trigger for alert {schedule_key}: {}", e);
            e
        });
    } else {
        let _ = db::scheduler::push(trigger).await.map_err(|e| {
            log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
            e
        });
    }

    Ok(alert)
}

pub async fn set_without_updating_trigger(org_id: &str, alert: Alert) -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let alert = table::put(client, org_id, "default", alert).await?;
    cluster::emit_put_event(org_id, &alert).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_put_event(org_id, &alert).await?;
    Ok(())
}

pub async fn delete_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<(), infra::errors::Error> {
    let Some((_folder, alert)) = table::get_by_id(conn, org_id, alert_id).await? else {
        return Ok(());
    };

    table::delete_by_id(conn, org_id, alert_id).await?;
    cluster::emit_delete_event(org_id, alert.stream_type, &alert.stream_name, &alert.name).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_delete_event(org_id, alert.stream_type, &alert.stream_name, &alert.name)
        .await?;

    let schedule_key = scheduler_key(alert.stream_type, &alert.stream_name, &alert.name);
    if let Err(e) =
        db::scheduler::delete(org_id, db::scheduler::TriggerModule::Alert, &schedule_key).await
    {
        log::error!("Failed to delete trigger: {}", e);
    };
    Ok(())
}

pub async fn delete_by_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<(), infra::errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    table::delete_by_name(client, org_id, "default", stream_type, stream_name, name).await?;
    cluster::emit_delete_event(org_id, stream_type, stream_name, name).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_delete_event(org_id, stream_type, stream_name, name).await?;

    let schedule_key = scheduler_key(stream_type, stream_name, name);
    if let Err(e) =
        db::scheduler::delete(org_id, db::scheduler::TriggerModule::Alert, &schedule_key).await
    {
        log::error!("Failed to delete trigger: {}", e);
    };
    Ok(())
}

pub async fn list(
    org_id: &str,
    stream_type: Option<StreamType>,
    stream_name: Option<&str>,
) -> Result<Vec<Alert>, infra::errors::Error> {
    let params = ListAlertsParams::new(org_id).in_folder("default");
    let params = if let Some(stream_name) = stream_name {
        params.for_stream(stream_type.unwrap_or_default(), stream_name)
    } else {
        params
    };

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let items = table::list(client, params)
        .await?
        .into_iter()
        .map(|(_f, a)| a)
        .collect();
    Ok(items)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch("/alerts/").await?;
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
                let Some((org, stream_type, stream_name, alert_name)) =
                    cluster::parse_alert_key(&ev.key)
                else {
                    log::error!("watch_alerts: failed to parse event key {}", &ev.key);
                    continue;
                };

                let item_value: Alert = if config::get_config().common.meta_store_external {
                    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
                    match table::get_by_name(
                        client,
                        &org,
                        "default",
                        stream_type,
                        &stream_name,
                        &alert_name,
                    )
                    .await
                    {
                        Ok(Some(val)) => val.1,
                        Ok(None) => {
                            log::error!("Tried to get alert that does not exist in DB");
                            continue;
                        }
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    }
                } else {
                    json::from_slice(&ev.value.unwrap()).unwrap()
                };

                let mut cacher = STREAM_ALERTS.write().await;
                let stream_key = cache_stream_key(&org, stream_type, &stream_name);
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
                let Some((org, stream_type, stream_name, alert_name)) =
                    cluster::parse_alert_key(&ev.key)
                else {
                    log::error!("watch_alerts: failed to parse event key {}", &ev.key);
                    continue;
                };

                let mut cacher = STREAM_ALERTS.write().await;
                let stream_key = cache_stream_key(&org, stream_type, &stream_name);
                let group = match cacher.get_mut(&stream_key) {
                    Some(v) => v,
                    None => continue,
                };
                group.retain(|v| !v.name.eq(&alert_name));

                drop(cacher);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let alerts = table::list(client, ListAlertsParams::all())
        .await?
        .into_iter()
        .map(|(_f, a)| a)
        .collect_vec();

    for alert in alerts {
        let mut cacher = STREAM_ALERTS.write().await;
        let stream_key = cache_stream_key(&alert.org_id, alert.stream_type, &alert.stream_name);
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

/// Returns the key used to store alerts in the in-memory cache, grouped by
/// stream.
fn cache_stream_key(org: &str, stream_type: StreamType, stream_name: &str) -> String {
    format!("{org}/{stream_type}/{stream_name}")
}

/// Returns the key used to schedule a trigger for the alert.
fn scheduler_key(stream_type: StreamType, stream_name: &str, alert_name: &str) -> String {
    format!("{stream_type}/{stream_name}/{alert_name}")
}

/// Helper functions for sending events to cache watchers in the cluster.
mod cluster {
    use config::meta::{alerts::alert::Alert, stream::StreamType};
    use itertools::Itertools;

    /// Sends event to the cluster cache watchers indicating that an alert has been
    /// put into the database.
    pub async fn emit_put_event(org: &str, alert: &Alert) -> Result<(), infra::errors::Error> {
        let key = alert_key(org, alert.stream_type, &alert.stream_name, &alert.name);
        let cluster_coordinator = infra::db::get_coordinator().await;
        cluster_coordinator
            .put(&key, bytes::Bytes::from(""), true, None)
            .await?;
        Ok(())
    }

    /// Sends event to the cluster cache watchers indicating that an alert has been
    /// deleted from the database.
    pub async fn emit_delete_event(
        org: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert_name: &str,
    ) -> Result<(), infra::errors::Error> {
        let key = alert_key(org, stream_type, stream_name, alert_name);
        let cluster_coordinator = infra::db::get_coordinator().await;
        cluster_coordinator.delete(&key, false, true, None).await
    }

    /// Returns the key used to identify an individual alert in events sent to
    /// cluster cache watchers.
    fn alert_key(
        org: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert_name: &str,
    ) -> String {
        format!("/alerts/{org}/{stream_type}/{stream_name}/{alert_name}")
    }

    /// Tries to parse the key used to identify an individual alert in avents
    /// sent to cluster cache watchers. Returns the organization, stream type,
    /// stream name, and alert name from the key.
    pub fn parse_alert_key(key: &str) -> Option<(String, StreamType, String, String)> {
        let parts = key.trim_start_matches("/").split('/').collect_vec();
        if parts.len() < 5 || parts[0] != "alerts" {
            return None;
        }
        let org = parts[1].to_owned();
        let stream_type: StreamType = parts[2].into();
        let stream_name = parts[3].to_owned();
        let alert_name = parts[4].to_owned();
        Some((org, stream_type, stream_name, alert_name))
    }
}

/// Helper functions for sending events to the super cluster queue.
#[cfg(feature = "enterprise")]
mod super_cluster {
    use config::{
        meta::{alerts::alert::Alert, stream::StreamType},
        utils::json,
    };
    use infra::errors::Error;
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

    /// Sends event to the super cluster queue indicating that an alert has been
    /// put into the database.
    pub async fn emit_put_event(org: &str, alert: &Alert) -> Result<(), infra::errors::Error> {
        let key = alert_key(org, alert.stream_type, &alert.stream_name, &alert.name);
        let value = json::to_vec(alert)?.into();
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::put(&key, value, true, None)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Sends event to the super cluster queue indicating that an alert has been
    /// deleted from the database.
    pub async fn emit_delete_event(
        org: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert_name: &str,
    ) -> Result<(), infra::errors::Error> {
        let key = alert_key(org, stream_type, stream_name, alert_name);
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::delete(&key, false, true, None)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Sends event to the super cluster queue indicating that all alert have
    /// been deleted from the database.
    pub async fn _emit_delete_all_event() -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::delete("/alerts/", true, false, None)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Returns the key used to identify an individual alert in events sent to
    /// the super cluster queue.
    fn alert_key(
        org: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert_name: &str,
    ) -> String {
        format!("/alerts/{org}/{stream_type}/{stream_name}/{alert_name}")
    }
}
