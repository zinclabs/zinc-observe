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

use config::{cluster::LOCAL_NODE, meta::promql::ClusterLeader};
use hashbrown::HashMap;
use tokio::time::{self, Duration};

use crate::{common::infra::config::METRIC_CLUSTER_LEADER, service::db};

pub async fn run() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_ingester() {
        return Ok(()); // not an ingester, no need to init job
    }

    let cfg = config::get_config();
    if !cfg.common.metrics_dedup_enabled {
        return Ok(());
    }

    let mut interval = time::interval(Duration::from_secs(cfg.limit.metrics_leader_push_interval));
    interval.tick().await; // trigger the first run

    let mut last_leaders: HashMap<String, ClusterLeader> = HashMap::new(); // maintain the last state

    loop {
        interval.tick().await;
        // only update if there's a change
        let map = METRIC_CLUSTER_LEADER.read().await.clone();
        for (key, value) in map.iter() {
            if last_leaders.contains_key(key) {
                let last_leader = last_leaders.get(key).unwrap();
                if value.eq(last_leader) {
                    continue;
                }
            }

            let result = db::metrics::set_prom_cluster_leader(key, value).await;
            match result {
                Ok(_) => {
                    let _ = last_leaders.insert(key.to_string(), value.clone());
                }
                Err(err) => log::error!("error updating leader to db {}", err),
            }
        }
    }
}
