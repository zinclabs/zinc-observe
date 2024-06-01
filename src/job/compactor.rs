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

use config::{
    cluster::{is_compactor, LOCAL_NODE_ROLE},
    get_config,
    meta::stream::FileKey,
};
use tokio::{
    sync::{mpsc, Mutex},
    time,
};

use crate::service::compact::{
    self,
    merge::{MergeBatch, MergeSender},
};

pub async fn run() -> Result<(), anyhow::Error> {
    if !is_compactor(&LOCAL_NODE_ROLE) {
        return Ok(());
    }

    let cfg = get_config();
    if !cfg.compact.enabled {
        return Ok(());
    }

    let (tx, rx) = mpsc::channel::<(MergeSender, MergeBatch)>(cfg.limit.file_move_thread_num);
    let rx = Arc::new(Mutex::new(rx));
    // start merge workers
    for thread_id in 0..cfg.limit.file_move_thread_num {
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
                        let ret = compact::merge::merge_files(
                            thread_id,
                            &msg.org_id,
                            msg.stream_type,
                            &msg.stream_name,
                            &msg.prefix,
                            &msg.files,
                        )
                        .await;
                        time::sleep(time::Duration::from_millis(10)).await;
                        match ret {
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
                                log::error!("[COMPACTOR:JOB] Error merging files: {}", e);
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

    tokio::task::spawn(async move { run_merge(tx).await });
    tokio::task::spawn(async move { run_retention().await });
    tokio::task::spawn(async move { run_delay_deletion().await });
    tokio::task::spawn(async move { run_sync_to_db().await });

    Ok(())
}

/// Merge small files
async fn run_merge(tx: mpsc::Sender<(MergeSender, MergeBatch)>) -> Result<(), anyhow::Error> {
    loop {
        time::sleep(time::Duration::from_secs(get_config().compact.interval)).await;
        let locker = compact::QUEUE_LOCKER.clone();
        let locker = locker.lock().await;
        log::debug!("[COMPACTOR] Running data merge");
        let ret = compact::run_merge(tx.clone()).await;
        if ret.is_err() {
            log::error!("[COMPACTOR] run data merge error: {}", ret.err().unwrap());
        }
        drop(locker);
    }
}

/// Deletion for data retention
async fn run_retention() -> Result<(), anyhow::Error> {
    loop {
        time::sleep(time::Duration::from_secs(get_config().compact.interval + 1)).await;
        let locker = compact::QUEUE_LOCKER.clone();
        let locker = locker.lock().await;
        log::debug!("[COMPACTOR] Running data retention");
        let ret = compact::run_retention().await;
        if ret.is_err() {
            log::error!("[COMPACTOR] run data delete error: {}", ret.err().unwrap());
        }
        drop(locker);
    }
}

/// Delete files based on the file_file_deleted in the database
async fn run_delay_deletion() -> Result<(), anyhow::Error> {
    loop {
        time::sleep(time::Duration::from_secs(get_config().compact.interval + 2)).await;
        let locker = compact::QUEUE_LOCKER.clone();
        let locker = locker.lock().await;
        log::debug!("[COMPACTOR] Running data delay deletion");
        let ret = compact::run_delay_deletion().await;
        if ret.is_err() {
            log::error!("[COMPACTOR] run files delete error: {}", ret.err().unwrap());
        }
        drop(locker);
    }
}

async fn run_sync_to_db() -> Result<(), anyhow::Error> {
    loop {
        time::sleep(time::Duration::from_secs(
            get_config().compact.sync_to_db_interval,
        ))
        .await;
        if let Err(e) = crate::service::db::compact::files::sync_cache_to_db().await {
            log::error!("[COMPACTOR] run offset sync cache to db error: {}", e);
        }
    }
}
