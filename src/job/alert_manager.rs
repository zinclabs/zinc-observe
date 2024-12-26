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

use config::{cluster::LOCAL_NODE, get_config};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
use tokio::time;

use crate::service;

pub async fn run() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_alert_manager() {
        return Ok(());
    }

    let cfg = get_config();
    log::info!(
        "Spawning embedded report server {}",
        cfg.report_server.enable_report_server
    );
    if cfg.report_server.enable_report_server {
        tokio::task::spawn(async move {
            if let Err(e) = report_server::spawn_server().await {
                log::error!("report server failed to spawn {}", e);
            }
        });
    }

    // check super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
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
        // register to super cluster
        o2_enterprise::enterprise::super_cluster::kv::alert_manager::register_job_cluster(
            &cluster_name,
        )
        .await?;
    }

    tokio::task::spawn(async move { run_schedule_jobs().await });
    tokio::task::spawn(async move { clean_complete_jobs().await });
    tokio::task::spawn(async move { watch_timeout_jobs().await });
    for i in 0..cfg.limit.search_job_workers {
        tokio::task::spawn(async move { run_search_jobs(i).await });
    }
    tokio::task::spawn(async move { run_check_running_search_jobs().await });
    tokio::task::spawn(async move { run_delete_jobs().await });

    Ok(())
}

async fn run_schedule_jobs() -> Result<(), anyhow::Error> {
    let interval = get_config().limit.alert_schedule_interval;
    let mut interval = time::interval(time::Duration::from_secs(interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = service::alerts::scheduler::run().await {
            log::error!("[ALERT MANAGER] run schedule jobs error: {}", e);
        }
    }
}

async fn clean_complete_jobs() -> Result<(), anyhow::Error> {
    let scheduler_clean_interval = get_config().limit.scheduler_clean_interval;
    if scheduler_clean_interval < 0 {
        return Ok(());
    }
    let mut interval = time::interval(time::Duration::from_secs(scheduler_clean_interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = infra::scheduler::clean_complete().await {
            log::error!("[ALERT MANAGER] clean complete jobs error: {}", e);
        }
    }
}

async fn watch_timeout_jobs() -> Result<(), anyhow::Error> {
    let scheduler_watch_interval = get_config().limit.scheduler_watch_interval;
    if scheduler_watch_interval < 0 {
        return Ok(());
    }
    let mut interval = time::interval(time::Duration::from_secs(scheduler_watch_interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = infra::scheduler::watch_timeout().await {
            log::error!("[ALERT MANAGER] watch timeout jobs error: {}", e);
        }
    }
}

#[cfg(feature = "enterprise")]
async fn run_search_jobs(id: i64) -> Result<(), anyhow::Error> {
    let interval = get_config().limit.search_job_scheduler_interval;
    let mut interval = time::interval(time::Duration::from_secs(interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = service::search_jobs::run(id).await {
            log::error!("[SEARCH JOB {id}] run search jobs error: {}", e);
        }
    }
}

#[cfg(feature = "enterprise")]
async fn run_check_running_search_jobs() -> Result<(), anyhow::Error> {
    let time = get_config().limit.search_job_run_timeout;
    let mut interval = time::interval(time::Duration::from_secs(time as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        log::debug!("[SEARCH JOB] Running check on running jobs");
        let now = config::utils::time::now_micros();
        let updated_at = now - (time * 1_000_000);
        if let Err(e) = service::db::search_job::search_jobs::check_running_jobs(updated_at).await {
            log::error!("[SEARCH JOB] Error checking running jobs: {e}");
        }
    }
}

#[cfg(feature = "enterprise")]
async fn run_delete_jobs() -> Result<(), anyhow::Error> {
    let interval = get_config().limit.search_job_delete_interval;
    let mut interval = time::interval(time::Duration::from_secs(interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        log::debug!("[SEARCH JOB] Running delete jobs");
        if let Err(e) = service::search_jobs::delete_jobs().await {
            log::error!("[SEARCH JOB] run delete jobs error: {}", e);
        }
    }
}

#[cfg(not(feature = "enterprise"))]
async fn run_search_jobs(_id: i64) -> Result<(), anyhow::Error> {
    Ok(())
}

#[cfg(not(feature = "enterprise"))]
async fn run_check_running_search_jobs() -> Result<(), anyhow::Error> {
    Ok(())
}

#[cfg(not(feature = "enterprise"))]
async fn run_delete_jobs() -> Result<(), anyhow::Error> {
    Ok(())
}
