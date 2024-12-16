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

use std::collections::HashMap;

use config::{
    cluster::LOCAL_NODE, get_config, get_instance_id, utils::json, SIZE_IN_MB, TELEMETRY_CLIENT,
};
use hashbrown::HashSet;
use infra::{cache::stats, db as infra_db, schema::STREAM_SCHEMAS_LATEST};
use segment::{message::Track, Client, Message};
use sysinfo::SystemExt;

use crate::common::infra::{cluster::get_cached_online_nodes, config::*};

#[derive(Clone, Debug, Default)]
pub struct Telemetry {
    pub instance_id: String,
    pub event: Track,
    pub base_info: HashMap<String, json::Value>,
}

impl Telemetry {
    pub fn new() -> Self {
        Telemetry {
            instance_id: "".to_string(),
            event: Track::default(),
            base_info: get_base_info(&mut HashMap::new()),
        }
    }

    pub fn add_event(&mut self, track: Track) {
        self.event = track;
    }

    pub async fn event(
        &mut self,
        event: &str,
        data: Option<HashMap<String, json::Value>>,
        send_zo_data: bool,
    ) {
        if !get_config().common.telemetry_enabled {
            return;
        }
        log::info!("sending event {}", event);
        let mut props = self.base_info.clone();
        if data.is_some() {
            for item in data.unwrap() {
                props.insert(item.0, item.1);
            }
        }
        if send_zo_data {
            props = add_zo_info(props).await;
        }
        self.add_event(Track {
            user: segment::message::User::UserId {
                user_id: segment::message::User::AnonymousId {
                    anonymous_id: get_instance_id(),
                }
                .to_string(),
            },
            event: event.to_string(),
            properties: json::to_value(props).unwrap(),
            timestamp: Some(time::OffsetDateTime::now_utc()),
            ..Default::default()
        });

        let res = TELEMETRY_CLIENT
            .send(get_instance_id(), Message::from(self.event.clone()))
            .await;

        if res.is_err() {
            log::error!("Error sending event {}, {:?}", event, res);
        }
    }

    pub async fn heart_beat(&mut self, event: &str, data: Option<HashMap<String, json::Value>>) {
        self.event(event, data, true).await;
    }
}

pub fn get_base_info(data: &mut HashMap<String, json::Value>) -> HashMap<String, json::Value> {
    let system = sysinfo::System::new_all();
    data.insert("cpu_count".to_string(), system.cpus().len().into());
    data.insert("total_memory".to_string(), system.total_memory().into());
    data.insert("free_memory".to_string(), system.free_memory().into());
    data.insert("os".to_string(), system.name().into());
    data.insert("os_release".to_string(), system.os_version().into());
    data.insert(
        "time_zone".to_string(),
        chrono::Local::now().offset().local_minus_utc().into(),
    );
    data.insert(
        "host_name".to_string(),
        get_config().common.instance_name.clone().into(),
    );

    data.insert("zo_version".to_string(), VERSION.to_owned().into());

    data.clone()
}

pub async fn add_zo_info(mut data: HashMap<String, json::Value>) -> HashMap<String, json::Value> {
    let mut num_streams = 0;
    let mut logs_streams = 0;
    let mut metrics_streams = 0;
    let mut orgs = HashSet::new();
    let r = STREAM_SCHEMAS_LATEST.read().await;
    for key in r.keys() {
        num_streams += 1;
        let stream_type = key.split('/').collect::<Vec<&str>>();
        orgs.insert(stream_type[0].to_string());
        if stream_type.len() < 2 {
            continue;
        }
        match stream_type[1] {
            "logs" => logs_streams += 1,
            "metrics" => metrics_streams += 1,
            _ => (),
        }
    }
    drop(r);

    let cfg = get_config();
    data.insert("num_org".to_string(), orgs.len().into());
    data.insert("num_streams".to_string(), num_streams.into());
    data.insert("num_logs_streams".to_string(), logs_streams.into());
    data.insert("num_metrics_streams".to_string(), metrics_streams.into());
    data.insert("num_users".to_string(), USERS.len().into());
    data.insert(
        "is_local_mode".to_string(),
        json::Value::Bool(cfg.common.local_mode),
    );
    if cfg.common.local_mode {
        data.insert(
            "local_mode_storage".to_string(),
            cfg.common.local_mode_storage.clone().into(),
        );
    }
    let db = infra_db::get_db().await;
    match db.list_keys("/dashboard/").await {
        Ok(keys) => {
            data.insert("num_dashboards".to_string(), keys.len().into());
        }
        Err(e) => {
            log::error!("Error getting dashboards {}", e);
        }
    }

    if !LOCAL_NODE.is_single_node() {
        match get_cached_online_nodes().await {
            Some(nodes) => {
                data.insert("is_HA_mode".to_string(), json::Value::Bool(true));
                data.insert("number_of_nodes".to_string(), nodes.len().into());
                data.insert(
                    "querier_nodes".to_string(),
                    crate::common::infra::cluster::get_cached_online_querier_nodes(None)
                        .await
                        .unwrap_or_default()
                        .len()
                        .into(),
                );
                data.insert(
                    "ingester_nodes".to_string(),
                    crate::common::infra::cluster::get_cached_online_ingester_nodes()
                        .await
                        .unwrap_or_default()
                        .len()
                        .into(),
                );
            }
            None => {
                data.insert("is_HA_mode".to_string(), json::Value::Bool(false));
            }
        }
    }
    let mut streams_orig_size: f64 = 0.0;
    let mut streams_compressed_size: f64 = 0.0;
    let mut logs_orig_size: f64 = 0.0;
    let mut logs_compressed_size: f64 = 0.0;
    let mut metrics_orig_size: f64 = 0.0;
    let mut metrics_compressed_size: f64 = 0.0;
    let mut traces_orig_size: f64 = 0.0;
    let mut traces_compressed_size: f64 = 0.0;
    for stats in stats::get_stats().iter() {
        streams_orig_size += stats.storage_size;
        streams_compressed_size += stats.compressed_size;

        let stream_type = stats.key().split('/').collect::<Vec<&str>>();
        if stream_type.len() < 2 {
            continue;
        }
        match stream_type[1] {
            "logs" => {
                logs_orig_size += stats.storage_size;
                logs_compressed_size += stats.compressed_size;
            }
            "metrics" => {
                metrics_orig_size += stats.storage_size;
                metrics_compressed_size += stats.compressed_size;
            }
            "traces" => {
                traces_orig_size += stats.storage_size;
                traces_compressed_size += stats.compressed_size;
            }
            _ => (),
        }
    }
    data.insert(
        "streams_total_size_mb".to_string(),
        format!("{:.0}", (streams_orig_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "streams_compressed_size_mb".to_string(),
        format!("{:.0}", (streams_compressed_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "metrics_orig_size".to_string(),
        format!("{:.0}", (metrics_orig_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "metrics_compressed_size".to_string(),
        format!("{:.0}", (metrics_compressed_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "logs_orig_size".to_string(),
        format!("{:.0}", (logs_orig_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "logs_compressed_size".to_string(),
        format!("{:.0}", (logs_compressed_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "traces_orig_size".to_string(),
        format!("{:.0}", (traces_orig_size / SIZE_IN_MB)).into(),
    );
    data.insert(
        "traces_compressed_size".to_string(),
        format!("{:.0}", (traces_compressed_size / SIZE_IN_MB)).into(),
    );

    data.insert(
        "num_query_functions".to_string(),
        QUERY_FUNCTIONS.len().into(),
    );

    let mut rt_alerts = 0;
    let mut scheduled_alerts = 0;
    let alert_cacher = STREAM_ALERTS.read().await;
    for (_, alerts) in alert_cacher.iter() {
        for alert in alerts.iter() {
            if alert.is_real_time {
                rt_alerts += 1
            } else {
                scheduled_alerts += 1
            }
        }
    }
    drop(alert_cacher);
    data.insert("real_time_alerts".to_string(), rt_alerts.into());
    data.insert("scheduled_alerts".to_string(), scheduled_alerts.into());
    data
}

#[cfg(test)]
mod test_telemetry {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_new() {
        let tel = Telemetry::new();
        let props = tel.base_info.clone();
        add_zo_info(props).await;
        assert!(!tel.base_info.is_empty())
    }
}
