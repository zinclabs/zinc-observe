use std::{future::Future, sync::Arc};

use bytes::Bytes;
use config::meta::{alerts::alert::Alert, stream::StreamType};
use itertools::Itertools;

use crate::{db::Event, errors::Error};

/// Sends event to the cluster coordinator indicating that an alert has been put
/// into the database.
pub async fn emit_put_event(org: &str, alert: &Alert) -> Result<(), Error> {
    let key = alert_key(org, alert.stream_type, &alert.stream_name, &alert.name);
    let cluster_coordinator = super::get_coordinator().await;
    cluster_coordinator
        .put(&key, bytes::Bytes::from(""), true, None)
        .await?;
    Ok(())
}

/// Sends event to the cluster coordinator indicating that an alert has been
/// deleted from the database.
pub async fn emit_delete_event(
    org: &str,
    stream_type: StreamType,
    stream_name: &str,
    alert_name: &str,
) -> Result<(), Error> {
    let key = alert_key(org, stream_type, stream_name, alert_name);
    let cluster_coordinator = super::get_coordinator().await;
    cluster_coordinator.delete(&key, false, true, None).await
}

/// Watch for alert events published to the cluster coordinator.
pub async fn watch_events<OnPut, OnPutFut, OnDelete, OnDeleteFut>(
    on_put: OnPut,
    on_delete: OnDelete,
) -> Result<(), anyhow::Error>
where
    OnPut: Fn(String, StreamType, String, String, Option<Bytes>) -> OnPutFut,
    OnPutFut: Future<Output = Result<(), anyhow::Error>>,
    OnDelete: Fn(String, StreamType, String, String) -> OnDeleteFut,
    OnDeleteFut: Future<Output = Result<(), anyhow::Error>>,
{
    let cluster_coordinator = super::get_coordinator().await;
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
            Event::Put(ev) => {
                let Some((org, stream_type, stream_name, alert_name)) = parse_alert_key(&ev.key)
                else {
                    log::error!("watch_alerts: failed to parse event key {}", &ev.key);
                    continue;
                };
                let _ = (on_put)(org, stream_type, stream_name, alert_name, ev.value).await;
            }
            Event::Delete(ev) => {
                let Some((org, stream_type, stream_name, alert_name)) = parse_alert_key(&ev.key)
                else {
                    log::error!("watch_alerts: failed to parse event key {}", &ev.key);
                    continue;
                };
                let _ = (on_delete)(org, stream_type, stream_name, alert_name).await;
            }
            Event::Empty => {}
        }
    }
    Ok(())
}

/// Returns the key used to identify an individual alert in events sent to
/// cluster cache watchers.
fn alert_key(org: &str, stream_type: StreamType, stream_name: &str, alert_name: &str) -> String {
    format!("/alerts/{org}/{stream_type}/{stream_name}/{alert_name}")
}

/// Tries to parse the key used to identify an individual alert in avents
/// sent to cluster cache watchers. Returns the organization, stream type,
/// stream name, and alert name from the key.
fn parse_alert_key(key: &str) -> Option<(String, StreamType, String, String)> {
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
