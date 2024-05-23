// Copyright 2023 Zinc Labs Inc.
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

use config::{cluster, config::CONFIG};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::O2_CONFIG;
use tokio::time;

use crate::service;

pub async fn run() -> Result<(), anyhow::Error> {
    if !cluster::is_alert_manager(&cluster::LOCAL_NODE_ROLE) {
        return Ok(());
    }

    log::info!(
        "Spawning embedded report server {}",
        CONFIG.report_server.enable_report_server
    );
    if CONFIG.report_server.enable_report_server {
        tokio::task::spawn(async move {
            if let Err(e) = reportserver::spawn_server().await {
                log::error!("report server failed to spawn {}", e);
            }
        });
    }

    // check super cluster
    #[cfg(feature = "enterprise")]
    if O2_CONFIG.super_cluster.enabled {
        let cluster_name =
            o2_enterprise::enterprise::super_cluster::kv::alert_manager::get_job_cluster().await?;
        if !cluster_name.is_empty() {
            let clusters = o2_enterprise::enterprise::super_cluster::kv::cluster::list().await?;
            if clusters.iter().any(|c| c.name == cluster_name) {
                log::info!("[ALERT MANAGER] is running in cluster: {}", cluster_name);
                return Ok(());
            }
        }
        let cluster_name = config::get_cluster_name();
        // regester to super cluster
        o2_enterprise::enterprise::super_cluster::kv::alert_manager::register_job_cluster(
            &cluster_name,
        )
        .await?;
    }

    tokio::task::spawn(async move { run_schedule_jobs().await });
    tokio::task::spawn(async move { clean_complete_jobs().await });
    tokio::task::spawn(async move { watch_timeout_jobs().await });

    Ok(())
}

async fn run_schedule_jobs() -> Result<(), anyhow::Error> {
    let mut interval = time::interval(time::Duration::from_secs(30));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = service::alerts::alert_manager::run().await {
            log::error!("[ALERT MANAGER] run schedule jobs error: {}", e);
        }
    }
}

async fn clean_complete_jobs() -> Result<(), anyhow::Error> {
    let mut interval = time::interval(time::Duration::from_secs(
        CONFIG.limit.scheduler_clean_interval,
    ));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = infra::scheduler::clean_complete().await {
            log::error!("[ALERT MANAGER] clean complete jobs error: {}", e);
        }
    }
}

async fn watch_timeout_jobs() -> Result<(), anyhow::Error> {
    let mut interval = time::interval(time::Duration::from_secs(
        CONFIG.limit.scheduler_watch_interval,
    ));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = infra::scheduler::watch_timeout().await {
            log::error!("[ALERT MANAGER] watch timeout jobs error: {}", e);
        }
    }
}
