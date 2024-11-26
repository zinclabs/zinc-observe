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

mod entry;
pub mod errors;
mod immutable;
mod memtable;
mod partition;
mod rwmap;
mod stream;
mod wal;
mod writer;

use std::{path::PathBuf, sync::Arc};

use arrow_schema::Schema;
use config::RwAHashMap;
pub use entry::Entry;
pub use immutable::read_from_immutable;
use once_cell::sync::Lazy;
use tokio::sync::{mpsc, Mutex};
pub use writer::{check_memtable_size, flush_all, get_writer, read_from_memtable, Writer};

pub(crate) type ReadRecordBatchEntry = (Arc<Schema>, Vec<Arc<entry::RecordBatchEntry>>);

pub static WAL_PARQUET_METADATA: Lazy<RwAHashMap<String, config::meta::stream::FileMeta>> =
    Lazy::new(Default::default);

pub async fn init() -> errors::Result<()> {
    // check uncompleted parquet files, need delete those files
    wal::check_uncompleted_parquet_files().await?;

    // replay wal files to create immutable
    tokio::task::spawn(async move {
        if let Err(e) = wal::replay_wal_files().await {
            log::error!("replay wal files error: {}", e);
        }
    });

    // start a job to flush memtable to immutable
    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(
                config::get_config().limit.max_file_retention_time,
            ))
            .await;
            // check memtable ttl
            if let Err(e) = writer::check_ttl().await {
                log::error!("memtable check ttl error: {}", e);
            }
        }
    });

    // start a job to flush memtable to immutable
    tokio::task::spawn(async move {
        if let Err(e) = run().await {
            log::error!("immutable persist error: {}", e);
        }
    });
    Ok(())
}

async fn run() -> errors::Result<()> {
    // start persidt worker
    let cfg = config::get_config();
    let (tx, rx) = mpsc::channel::<PathBuf>(cfg.limit.mem_dump_thread_num);
    let rx = Arc::new(Mutex::new(rx));
    for thread_id in 0..cfg.limit.mem_dump_thread_num {
        let rx = rx.clone();
        tokio::spawn(async move {
            loop {
                let ret = rx.lock().await.recv().await;
                match ret {
                    None => {
                        log::debug!("[INGESTER:MEM] Receiving memtable channel is closed");
                        break;
                    }
                    Some(path) => {
                        if let Err(e) = immutable::persist_table(thread_id, path).await {
                            log::error!("[INGESTER:MEM:{thread_id}] Error persist memtable: {e}");
                        }
                    }
                }
            }
        });
    }

    // start a job to dump immutable data to disk
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        config::get_config().limit.mem_persist_interval,
    ));
    interval.tick().await; // the first tick is immediate
    loop {
        if config::cluster::is_offline() {
            break;
        }
        interval.tick().await;
        // persist immutable data to disk
        if let Err(e) = immutable::persist(tx.clone()).await {
            log::error!("immutable persist error: {}", e);
        }
        // shrink metadata cache
        WAL_PARQUET_METADATA.write().await.shrink_to_fit();
    }

    log::info!("[INGESTER:MEM] immutable persist is stopped");
    Ok(())
}
