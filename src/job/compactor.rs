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
    cluster::LOCAL_NODE,
    get_config,
    meta::{cluster::CompactionJobType, stream::FileKey},
    metrics,
};
use tokio::sync::{mpsc, Mutex};

use crate::service::compact::{
    self,
    merge::{MergeBatch, MergeSender},
};

pub async fn run() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_compactor() {
        return Ok(());
    }

    let cfg = get_config();
    if !cfg.compact.enabled {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<(MergeSender, MergeBatch)>(cfg.limit.file_merge_thread_num);
    let rx = Arc::new(Mutex::new(rx));
    // start merge workers
    for thread_id in 0..cfg.limit.file_merge_thread_num {
        let rx = rx.clone();
        tokio::spawn(async move {
            loop {
                let ret = rx.lock().await.recv().await;
                match ret {
                    None => {
                        log::debug!("[COMPACTOR:JOB] Receiving files channel is closed");
                        break;
                    }
                    Some((tx, msg)) => {
                        match compact::merge::merge_files(
                            thread_id,
                            &msg.org_id,
                            msg.stream_type,
                            &msg.stream_name,
                            &msg.prefix,
                            &msg.files,
                        )
                        .await
                        {
                            Ok((file, meta, _)) => {
                                if let Err(e) = tx
                                    .send(Ok((msg.batch_id, FileKey::new(&file, meta, false))))
                                    .await
                                {
                                    log::error!(
                                        "[COMPACTOR:JOB] Error sending file to merge_job: {}",
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "[COMPACTOR:JOB] Error merging files: stream: {}/{}/{}, err: {}",
                                    msg.org_id,
                                    msg.stream_type,
                                    msg.stream_name,
                                    e
                                );
                                if let Err(e) = tx.send(Err(e)).await {
                                    log::error!(
                                        "[COMPACTOR:JOB] Error sending error to merge_job: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    tokio::task::spawn(async move { run_generate_job().await });
    tokio::task::spawn(async move { run_generate_old_data_job().await });
    tokio::task::spawn(async move { run_merge(tx).await });
    tokio::task::spawn(async move { run_retention().await });
    tokio::task::spawn(async move { run_delay_deletion().await });
    tokio::task::spawn(async move { run_sync_to_db().await });
    tokio::task::spawn(async move { run_check_running_jobs().await });
    tokio::task::spawn(async move { run_clean_done_jobs().await });
    tokio::task::spawn(async move { run_compactor_pending_jobs_metric().await });

    Ok(())
}

/// Report compactor pending jobs as prometheus metric
async fn run_compactor_pending_jobs_metric() -> Result<(), anyhow::Error> {
    let interval = get_config().compact.pending_jobs_metric_interval;

    log::info!("[COMPACTOR] start run_compactor_pending_jobs_metric job");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

        log::debug!("[COMPACTOR] Running compactor pending jobs to report metric");
        let job_status = match infra::file_list::get_pending_jobs_count().await {
            Ok(status) => status,
            Err(e) => {
                log::error!("[COMPACTOR] run compactor pending jobs metric error: {e}");
                continue;
            }
        };

        for (org, inner_map) in job_status {
            for (stream_type, counter) in inner_map {
                metrics::COMPACT_PENDING_JOBS
                    .with_label_values(&[org.as_str(), stream_type.as_str()])
                    .set(counter);
            }
        }
    }
}

/// Generate merging jobs
async fn run_generate_job() -> Result<(), anyhow::Error> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            get_config().compact.interval,
        ))
        .await;
        log::debug!("[COMPACTOR] Running generate merge job");
        if let Err(e) = compact::run_generate_job(CompactionJobType::Current).await {
            log::error!("[COMPACTOR] run generate merge job error: {e}");
        }
    }
}

/// Generate merging jobs for old data
async fn run_generate_old_data_job() -> Result<(), anyhow::Error> {
    loop {
        // run every 1 hour at least
        tokio::time::sleep(tokio::time::Duration::from_secs(
            get_config().compact.old_data_interval,
        ))
        .await;
        log::debug!("[COMPACTOR] Running generate merge job for old data");
        if let Err(e) = compact::run_generate_job(CompactionJobType::Historical).await {
            log::error!("[COMPACTOR] run generate merge job for old data error: {e}");
        }
    }
}

/// Merge small files
async fn run_merge(tx: mpsc::Sender<(MergeSender, MergeBatch)>) -> Result<(), anyhow::Error> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            get_config().compact.interval,
        ))
        .await;
        log::debug!("[COMPACTOR] Running data merge");
        if let Err(e) = compact::run_merge(tx.clone()).await {
            log::error!("[COMPACTOR] run data merge error: {e}");
        }
    }
}

/// Deletion for data retention
async fn run_retention() -> Result<(), anyhow::Error> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            get_config().compact.interval + 1,
        ))
        .await;
        log::debug!("[COMPACTOR] Running data retention");
        if let Err(e) = compact::run_retention().await {
            log::error!("[COMPACTOR] run data retention error: {e}");
        }
    }
}

/// Delete files based on the file_file_deleted in the database
async fn run_delay_deletion() -> Result<(), anyhow::Error> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            get_config().compact.interval + 2,
        ))
        .await;
        log::debug!("[COMPACTOR] Running data delay deletion");
        if let Err(e) = compact::run_delay_deletion().await {
            log::error!("[COMPACTOR] run data delay deletion error: {e}");
        }
    }
}

async fn run_sync_to_db() -> Result<(), anyhow::Error> {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            get_config().compact.sync_to_db_interval,
        ))
        .await;
        log::debug!("[COMPACTOR] Running sync cached compact offset to db");
        if let Err(e) = crate::service::db::compact::files::sync_cache_to_db().await {
            log::error!("[COMPACTOR] run sync cached compact offset to db error: {e}");
        }
    }
}

async fn run_check_running_jobs() -> Result<(), anyhow::Error> {
    loop {
        let time = get_config().compact.job_run_timeout;
        log::debug!("[COMPACTOR] Running check running jobs");
        let updated_at = config::utils::time::now_micros() - (time * 1000 * 1000);
        if let Err(e) = infra::file_list::check_running_jobs(updated_at).await {
            log::error!("[COMPACTOR] run check running jobs error: {e}");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(time as u64)).await;
    }
}

async fn run_clean_done_jobs() -> Result<(), anyhow::Error> {
    loop {
        let time = get_config().compact.job_clean_wait_time;
        log::debug!("[COMPACTOR] Running clean done jobs");
        let updated_at = config::utils::time::now_micros() - (time * 1000 * 1000);
        if let Err(e) = infra::file_list::clean_done_jobs(updated_at).await {
            log::error!("[COMPACTOR] run clean done jobs error: {e}");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(time as u64)).await;
    }
}
