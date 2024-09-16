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

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Arc,
    },
};

use arrow_schema::Schema;
use chrono::{Duration, Utc};
use config::{
    get_config, metrics,
    utils::hash::{gxhash, Sum64},
    MEM_TABLE_INDIVIDUAL_STREAMS,
};
use once_cell::sync::Lazy;
use snafu::ResultExt;
use tokio::sync::{Mutex, RwLock};
use wal::Writer as WalWriter;

use crate::{
    entry::Entry,
    errors::*,
    immutable::{Immutable, IMMUTABLES},
    memtable::MemTable,
    rwmap::RwMap,
    ReadRecordBatchEntry,
};

static WRITERS: Lazy<Vec<RwMap<WriterKey, Arc<Writer>>>> = Lazy::new(|| {
    let cfg = get_config();
    let writer_num = cfg.limit.mem_table_bucket_num + MEM_TABLE_INDIVIDUAL_STREAMS.len();
    let mut writers = Vec::with_capacity(writer_num);
    for _ in 0..writer_num {
        writers.push(RwMap::default());
    }
    writers
});

pub struct Writer {
    idx: usize,
    key: WriterKey,
    wal: Arc<Mutex<WalWriter>>,
    memtable: Arc<RwLock<MemTable>>,
    next_seq: AtomicU64,
    created_at: AtomicI64,
}

// check total memory size
pub fn check_memtable_size() -> Result<()> {
    let total_mem_size = metrics::INGEST_MEMTABLE_ARROW_BYTES
        .with_label_values(&[])
        .get();
    if total_mem_size >= get_config().limit.mem_table_max_size as i64 {
        Err(Error::MemoryTableOverflowError {})
    } else {
        Ok(())
    }
}

/// Get a writer for a given org_id and stream_type
pub async fn get_writer(org_id: &str, stream_type: &str, stream_name: &str) -> Arc<Writer> {
    let idx = if let Some(idx) = MEM_TABLE_INDIVIDUAL_STREAMS.get(stream_name) {
        *idx
    } else {
        let hash_id = gxhash::new().sum64(stream_name);
        hash_id as usize % (WRITERS.len() - MEM_TABLE_INDIVIDUAL_STREAMS.len())
    };
    let key = WriterKey::new(org_id, stream_type);
    let mut rw = WRITERS[idx].write().await;
    let w = rw
        .entry(key.clone())
        .or_insert_with(|| Arc::new(Writer::new(idx, key)));
    w.clone()
}

pub async fn read_from_memtable(
    org_id: &str,
    stream_type: &str,
    stream_name: &str,
    time_range: Option<(i64, i64)>,
) -> Result<Vec<ReadRecordBatchEntry>> {
    let key = WriterKey::new(org_id, stream_type);
    let hash_id = gxhash::new().sum64(stream_name);
    let idx = hash_id as usize % WRITERS.len();
    let w = WRITERS[idx].read().await;
    let Some(r) = w.get(&key) else {
        return Ok(Vec::new());
    };
    r.read(stream_name, time_range).await
}

pub async fn check_ttl() -> Result<()> {
    for w in WRITERS.iter() {
        let w = w.read().await;
        for r in w.values() {
            // check writer
            r.write(Arc::new(Schema::empty()), Entry::default(), true)
                .await?;
        }
    }
    Ok(())
}

pub async fn flush_all() -> Result<()> {
    for w in WRITERS.iter() {
        let mut w = w.write().await;
        let keys = w.keys().cloned().collect::<Vec<_>>();
        for r in w.values() {
            r.close().await?; // close writer
            metrics::INGEST_MEMTABLE_FILES.with_label_values(&[]).dec();
        }
        for key in keys {
            w.remove(&key);
        }
    }
    Ok(())
}

impl Writer {
    pub(crate) fn new(idx: usize, key: WriterKey) -> Self {
        let now = Utc::now().timestamp_micros();
        let cfg = get_config();
        let next_seq = AtomicU64::new(now as u64);
        let wal_id = next_seq.fetch_add(1, Ordering::SeqCst);
        let wal_dir = PathBuf::from(&cfg.common.data_wal_dir)
            .join("logs")
            .join(idx.to_string());
        log::info!(
            "[INGESTER:MEM] create file: {}/{}/{}/{}.wal",
            wal_dir.display().to_string(),
            &key.org_id,
            &key.stream_type,
            wal_id
        );
        Self {
            idx,
            key: key.clone(),
            wal: Arc::new(Mutex::new(
                WalWriter::new(
                    wal_dir,
                    &key.org_id,
                    &key.stream_type,
                    wal_id,
                    cfg.limit.max_file_size_on_disk as u64,
                    cfg.limit.wal_write_buffer_size,
                )
                .expect("wal file create error"),
            )),
            memtable: Arc::new(RwLock::new(MemTable::new())),
            next_seq,
            created_at: AtomicI64::new(now),
        }
    }

    // check_ttl is used to check if the memtable has expired
    pub async fn write(
        &self,
        schema: Arc<Schema>,
        mut entry: Entry,
        check_ttl: bool,
    ) -> Result<()> {
        if entry.data.is_empty() && !check_ttl {
            return Ok(());
        }
        let (entry_bytes, entry_batch) = if !check_ttl {
            let bytes = entry.into_bytes()?;
            let batch = entry.into_batch(self.key.stream_type.clone(), schema.clone())?;
            (bytes, Some(batch))
        } else {
            (Vec::new(), None)
        };
        let start = std::time::Instant::now();
        let mut wal = self.wal.lock().await;
        let wal_lock_time = start.elapsed().as_millis() as f64;
        metrics::INGEST_WAL_LOCK_TIME
            .with_label_values(&[&self.key.org_id])
            .observe(wal_lock_time);
        let mut mem = self.memtable.write().await;
        let mem_lock_time = start.elapsed().as_millis() as f64 - wal_lock_time;
        metrics::INGEST_MEMTABLE_LOCK_TIME
            .with_label_values(&[&self.key.org_id])
            .observe(mem_lock_time);
        if self.check_wal_threshold(wal.size(), entry_bytes.len())
            || self.check_mem_threshold(mem.size(), entry.data_size)
        {
            let cfg = get_config();
            // sync wal before rotation
            wal.sync().context(WalSnafu)?;
            // rotation wal
            let wal_id = self.next_seq.fetch_add(1, Ordering::SeqCst);
            let wal_dir = PathBuf::from(&cfg.common.data_wal_dir)
                .join("logs")
                .join(self.idx.to_string());
            log::info!(
                "[INGESTER:MEM] create file: {}/{}/{}/{}.wal",
                wal_dir.display().to_string(),
                &self.key.org_id,
                &self.key.stream_type,
                wal_id
            );
            let new_wal = WalWriter::new(
                wal_dir,
                &self.key.org_id,
                &self.key.stream_type,
                wal_id,
                cfg.limit.max_file_size_on_disk as u64,
                cfg.limit.wal_write_buffer_size,
            )
            .context(WalSnafu)?;
            let old_wal = std::mem::replace(&mut *wal, new_wal);

            // rotation memtable
            let new_mem = MemTable::new();
            let old_mem = std::mem::replace(&mut *mem, new_mem);
            // update created_at
            self.created_at
                .store(Utc::now().timestamp_micros(), Ordering::Release);

            let path = old_wal.path().clone();
            let path_str = path.display().to_string();
            let table = Arc::new(Immutable::new(self.idx, self.key.clone(), old_mem));
            log::info!("[INGESTER:MEM] start add to IMMUTABLES, file: {}", path_str,);
            IMMUTABLES.write().await.insert(path, table);
            log::info!("[INGESTER:MEM] dones add to IMMUTABLES, file: {}", path_str);
        }

        if !check_ttl {
            // write into wal
            wal.write(&entry_bytes, false).context(WalSnafu)?;
            // write into memtable
            let Some(entry_batch) = entry_batch else {
                return Ok(());
            };
            mem.write(schema, entry, entry_batch)?;
        }

        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        // rotation wal
        let mut wal = self.wal.lock().await;
        wal.sync().context(WalSnafu)?;
        let path = wal.path().clone();
        drop(wal);

        // rotation memtable
        let mut mem = self.memtable.write().await;
        let new_mem = MemTable::new();
        let old_mem = std::mem::replace(&mut *mem, new_mem);
        drop(mem);

        let table = Arc::new(Immutable::new(self.idx, self.key.clone(), old_mem));
        IMMUTABLES.write().await.insert(path, table);
        Ok(())
    }

    pub async fn sync(&self) -> Result<()> {
        let mut wal = self.wal.lock().await;
        wal.sync().context(WalSnafu)
    }

    pub async fn read(
        &self,
        stream_name: &str,
        time_range: Option<(i64, i64)>,
    ) -> Result<Vec<ReadRecordBatchEntry>> {
        let memtable = self.memtable.read().await;
        memtable.read(stream_name, time_range)
    }

    /// Check if the wal file size is over the threshold or the file is too old
    fn check_wal_threshold(&self, written_size: (usize, usize), data_size: usize) -> bool {
        let cfg = get_config();
        let (compressed_size, _uncompressed_size) = written_size;
        compressed_size > wal::FILE_TYPE_IDENTIFIER_LEN
            && (compressed_size + data_size > cfg.limit.max_file_size_on_disk
                || self.created_at.load(Ordering::Relaxed)
                    + Duration::try_seconds(cfg.limit.max_file_retention_time as i64)
                        .unwrap()
                        .num_microseconds()
                        .unwrap()
                    <= Utc::now().timestamp_micros())
    }

    /// Check if the memtable size is over the threshold
    fn check_mem_threshold(&self, written_size: (usize, usize), data_size: usize) -> bool {
        let cfg = get_config();
        let (json_size, arrow_size) = written_size;
        json_size > 0
            && (json_size + data_size > cfg.limit.max_file_size_in_memory
                || arrow_size + data_size > cfg.limit.max_file_size_in_memory)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct WriterKey {
    pub(crate) org_id: Arc<str>,
    pub(crate) stream_type: Arc<str>,
}

impl WriterKey {
    pub(crate) fn new<T>(org_id: T, stream_type: T) -> Self
    where
        T: AsRef<str>,
    {
        Self {
            org_id: Arc::from(org_id.as_ref()),
            stream_type: Arc::from(stream_type.as_ref()),
        }
    }
}
