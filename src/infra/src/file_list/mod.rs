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

use std::collections::HashMap as stdHashMap;

use async_trait::async_trait;
use config::{
    meta::{
        meta_store::MetaStore,
        stream::{FileKey, FileListDeleted, FileMeta, PartitionTimeLevel, StreamStats, StreamType},
    },
    utils::time::second_micros,
};
use once_cell::sync::Lazy;

use crate::errors::{Error, Result};

pub mod mysql;
pub mod postgres;
pub mod sqlite;

static CLIENT: Lazy<Box<dyn FileList>> = Lazy::new(connect_default);
pub static LOCAL_CACHE: Lazy<Box<dyn FileList>> = Lazy::new(connect_local_cache);

pub fn connect_default() -> Box<dyn FileList> {
    match config::get_config().common.meta_store.as_str().into() {
        MetaStore::Sqlite => Box::<sqlite::SqliteFileList>::default(),
        MetaStore::Etcd => Box::<sqlite::SqliteFileList>::default(),
        MetaStore::Nats => Box::<sqlite::SqliteFileList>::default(),
        MetaStore::MySQL => Box::<mysql::MysqlFileList>::default(),
        MetaStore::PostgreSQL => Box::<postgres::PostgresFileList>::default(),
    }
}

pub fn connect_local_cache() -> Box<dyn FileList> {
    Box::<sqlite::SqliteFileList>::default()
}

#[async_trait]
pub trait FileList: Sync + Send + 'static {
    async fn create_table(&self) -> Result<()>;
    async fn create_table_index(&self) -> Result<()>;
    async fn add(&self, file: &str, meta: &FileMeta) -> Result<()>;
    async fn add_history(&self, file: &str, meta: &FileMeta) -> Result<()>;
    async fn remove(&self, file: &str) -> Result<()>;
    async fn batch_add(&self, files: &[FileKey]) -> Result<()>;
    async fn batch_add_with_id(&self, files: &[(i64, &FileKey)]) -> Result<()>;
    async fn batch_add_history(&self, files: &[FileKey]) -> Result<()>;
    async fn batch_remove(&self, files: &[String]) -> Result<()>;
    async fn batch_add_deleted(
        &self,
        org_id: &str,
        created_at: i64,
        files: &[FileListDeleted],
    ) -> Result<()>;
    async fn batch_remove_deleted(&self, files: &[String]) -> Result<()>;
    async fn get(&self, file: &str) -> Result<FileMeta>;
    async fn contains(&self, file: &str) -> Result<bool>;
    async fn update_flattened(&self, file: &str, flattened: bool) -> Result<()>;
    async fn list(&self) -> Result<Vec<(String, FileMeta)>>;
    async fn query(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_level: PartitionTimeLevel,
        time_range: Option<(i64, i64)>,
        flattened: Option<bool>,
    ) -> Result<Vec<(String, FileMeta)>>;
    async fn query_by_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String, FileMeta)>>;
    async fn query_ids(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_range: Option<(i64, i64)>,
    ) -> Result<Vec<FileId>>;
    async fn query_old_data_hours(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_range: Option<(i64, i64)>,
    ) -> Result<Vec<String>>;
    async fn query_deleted(
        &self,
        org_id: &str,
        time_max: i64,
        limit: i64,
    ) -> Result<Vec<FileListDeleted>>;
    // stream stats
    async fn get_min_ts(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
    ) -> Result<i64>;
    async fn get_max_pk_value(&self) -> Result<i64>;
    async fn get_min_pk_value(&self) -> Result<i64>;
    async fn clean_by_min_pk_value(&self, val: i64) -> Result<()>;
    async fn stats(
        &self,
        org_id: &str,
        stream_type: Option<StreamType>,
        stream_name: Option<&str>,
        pk_value: Option<(i64, i64)>,
    ) -> Result<Vec<(String, StreamStats)>>;
    async fn get_stream_stats(
        &self,
        org_id: &str,
        stream_type: Option<StreamType>,
        stream_name: Option<&str>,
    ) -> Result<Vec<(String, StreamStats)>>;
    async fn del_stream_stats(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
    ) -> Result<()>;
    async fn set_stream_stats(&self, org_id: &str, streams: &[(String, StreamStats)])
        -> Result<()>;
    async fn reset_stream_stats(&self) -> Result<()>;
    async fn reset_stream_stats_min_ts(
        &self,
        org_id: &str,
        stream: &str,
        min_ts: i64,
    ) -> Result<()>;
    async fn len(&self) -> usize;
    async fn is_empty(&self) -> bool;
    async fn clear(&self) -> Result<()>;
    // merge job
    async fn add_job(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream: &str,
        offset: i64,
    ) -> Result<i64>;
    async fn get_pending_jobs(&self, node: &str, limit: i64) -> Result<Vec<MergeJobRecord>>;
    async fn get_pending_jobs_count(&self) -> Result<stdHashMap<String, stdHashMap<String, i64>>>;
    async fn set_job_pending(&self, ids: &[i64]) -> Result<()>;
    async fn set_job_done(&self, id: i64) -> Result<()>;
    async fn update_running_jobs(&self, id: i64) -> Result<()>;
    async fn check_running_jobs(&self, before_date: i64) -> Result<()>;
    async fn clean_done_jobs(&self, before_date: i64) -> Result<()>;
}

pub async fn create_table() -> Result<()> {
    CLIENT.create_table().await
}

pub async fn create_table_index() -> Result<()> {
    CLIENT.create_table_index().await
}

#[inline]
pub async fn add(file: &str, meta: &FileMeta) -> Result<()> {
    CLIENT.add(file, meta).await
}

#[inline]
pub async fn add_history(file: &str, meta: &FileMeta) -> Result<()> {
    CLIENT.add_history(file, meta).await
}

#[inline]
pub async fn remove(file: &str) -> Result<()> {
    CLIENT.remove(file).await
}

#[inline]
pub async fn batch_add(files: &[FileKey]) -> Result<()> {
    CLIENT.batch_add(files).await
}

#[inline]
pub async fn batch_add_history(files: &[FileKey]) -> Result<()> {
    CLIENT.batch_add_history(files).await
}

#[inline]
pub async fn batch_remove(files: &[String]) -> Result<()> {
    CLIENT.batch_remove(files).await
}

#[inline]
pub async fn batch_add_deleted(
    org_id: &str,
    created_at: i64,
    files: &[FileListDeleted],
) -> Result<()> {
    CLIENT.batch_add_deleted(org_id, created_at, files).await
}

#[inline]
pub async fn batch_remove_deleted(files: &[String]) -> Result<()> {
    CLIENT.batch_remove_deleted(files).await
}

#[inline]
pub async fn get(file: &str) -> Result<FileMeta> {
    CLIENT.get(file).await
}

#[inline]
pub async fn contains(file: &str) -> Result<bool> {
    CLIENT.contains(file).await
}

#[inline]
pub async fn update_flattened(file: &str, flattened: bool) -> Result<()> {
    CLIENT.update_flattened(file, flattened).await
}

#[inline]
pub async fn list() -> Result<Vec<(String, FileMeta)>> {
    CLIENT.list().await
}

#[inline]
#[tracing::instrument(name = "infra:file_list:db:query")]
pub async fn query(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    time_level: PartitionTimeLevel,
    time_range: Option<(i64, i64)>,
    flattened: Option<bool>,
) -> Result<Vec<(String, FileMeta)>> {
    validate_time_range(time_range)?;
    CLIENT
        .query(
            org_id,
            stream_type,
            stream_name,
            time_level,
            time_range,
            flattened,
        )
        .await
}

#[inline]
#[tracing::instrument(name = "infra:file_list:query_db_by_ids", skip_all)]
pub async fn query_by_ids(ids: &[i64]) -> Result<Vec<(i64, String, FileMeta)>> {
    CLIENT.query_by_ids(ids).await
}

#[inline]
#[tracing::instrument(name = "infra:file_list:db:query_ids")]
pub async fn query_ids(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    time_range: Option<(i64, i64)>,
) -> Result<Vec<FileId>> {
    validate_time_range(time_range)?;
    CLIENT
        .query_ids(org_id, stream_type, stream_name, time_range)
        .await
}

#[inline]
#[tracing::instrument(name = "infra:file_list:db:query_old_data_hours")]
pub async fn query_old_data_hours(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    time_range: Option<(i64, i64)>,
) -> Result<Vec<String>> {
    validate_time_range(time_range)?;
    CLIENT
        .query_old_data_hours(org_id, stream_type, stream_name, time_range)
        .await
}

#[inline]
pub async fn query_deleted(
    org_id: &str,
    time_max: i64,
    limit: i64,
) -> Result<Vec<FileListDeleted>> {
    CLIENT.query_deleted(org_id, time_max, limit).await
}

#[inline]
pub async fn get_min_ts(org_id: &str, stream_type: StreamType, stream_name: &str) -> Result<i64> {
    CLIENT.get_min_ts(org_id, stream_type, stream_name).await
}

#[inline]
pub async fn get_max_pk_value() -> Result<i64> {
    CLIENT.get_max_pk_value().await
}

#[inline]
pub async fn get_min_pk_value() -> Result<i64> {
    CLIENT.get_min_pk_value().await
}

#[inline]
pub async fn stats(
    org_id: &str,
    stream_type: Option<StreamType>,
    stream_name: Option<&str>,
    pk_value: Option<(i64, i64)>,
) -> Result<Vec<(String, StreamStats)>> {
    CLIENT
        .stats(org_id, stream_type, stream_name, pk_value)
        .await
}

#[inline]
pub async fn get_stream_stats(
    org_id: &str,
    stream_type: Option<StreamType>,
    stream_name: Option<&str>,
) -> Result<Vec<(String, StreamStats)>> {
    CLIENT
        .get_stream_stats(org_id, stream_type, stream_name)
        .await
}

#[inline]
pub async fn del_stream_stats(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<()> {
    CLIENT
        .del_stream_stats(org_id, stream_type, stream_name)
        .await
}

#[inline]
pub async fn set_stream_stats(org_id: &str, streams: &[(String, StreamStats)]) -> Result<()> {
    CLIENT.set_stream_stats(org_id, streams).await
}

#[inline]
pub async fn reset_stream_stats() -> Result<()> {
    CLIENT.reset_stream_stats().await
}

#[inline]
pub async fn reset_stream_stats_min_ts(org_id: &str, stream: &str, min_ts: i64) -> Result<()> {
    CLIENT
        .reset_stream_stats_min_ts(org_id, stream, min_ts)
        .await
}

#[inline]
pub async fn len() -> usize {
    CLIENT.len().await
}

#[inline]
pub async fn is_empty() -> bool {
    CLIENT.is_empty().await
}

#[inline]
pub async fn clear() -> Result<()> {
    CLIENT.clear().await
}

#[inline]
pub async fn add_job(
    org_id: &str,
    stream_type: StreamType,
    stream: &str,
    offset: i64,
) -> Result<i64> {
    CLIENT.add_job(org_id, stream_type, stream, offset).await
}

#[inline]
pub async fn get_pending_jobs(node: &str, limit: i64) -> Result<Vec<MergeJobRecord>> {
    CLIENT.get_pending_jobs(node, limit).await
}

#[inline]
pub async fn get_pending_jobs_count() -> Result<stdHashMap<String, stdHashMap<String, i64>>> {
    CLIENT.get_pending_jobs_count().await
}

#[inline]
pub async fn set_job_pending(ids: &[i64]) -> Result<()> {
    CLIENT.set_job_pending(ids).await
}

#[inline]
pub async fn set_job_done(id: i64) -> Result<()> {
    CLIENT.set_job_done(id).await
}

#[inline]
pub async fn update_running_jobs(id: i64) -> Result<()> {
    CLIENT.update_running_jobs(id).await
}

#[inline]
pub async fn check_running_jobs(before_date: i64) -> Result<()> {
    CLIENT.check_running_jobs(before_date).await
}

#[inline]
pub async fn clean_done_jobs(before_date: i64) -> Result<()> {
    CLIENT.clean_done_jobs(before_date).await
}

pub async fn local_cache_gc() -> Result<()> {
    tokio::task::spawn(async move {
        let cfg = config::get_config();
        if cfg.common.local_mode || !cfg.common.meta_store_external {
            return;
        }

        // gc every hour
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        interval.tick().await; // the first tick is immediate
        loop {
            if let Ok(min_id) = get_min_pk_value().await {
                if min_id > 0 {
                    match LOCAL_CACHE.clean_by_min_pk_value(min_id).await {
                        Ok(_) => log::info!("[file_list] local cache gc done"),
                        Err(e) => log::error!("[file_list] local cache gc failed: {}", e),
                    }
                }
            }
            interval.tick().await;
        }
    });

    Ok(())
}

fn validate_time_range(time_range: Option<(i64, i64)>) -> Result<()> {
    if let Some((start, end)) = time_range {
        if start > end || start == 0 || end == 0 {
            return Err(Error::Message("[file_list] invalid time range".to_string()));
        }
    }
    Ok(())
}

fn calculate_max_ts_upper_bound(time_end: i64, stream_type: StreamType) -> i64 {
    let ts = super::schema::unwrap_partition_time_level(None, stream_type).duration();
    if ts > 0 {
        time_end + second_micros(ts)
    } else {
        time_end + second_micros(PartitionTimeLevel::Hourly.duration())
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct FileRecord {
    #[sqlx(default)]
    pub id: i64,
    #[sqlx(default)]
    pub stream: String,
    pub date: String,
    pub file: String,
    #[sqlx(default)]
    pub deleted: bool,
    pub min_ts: i64,
    pub max_ts: i64,
    pub records: i64,
    pub original_size: i64,
    pub compressed_size: i64,
    #[sqlx(default)]
    pub index_size: i64,
    #[sqlx(default)]
    pub flattened: bool,
}

impl From<&FileRecord> for FileMeta {
    fn from(record: &FileRecord) -> Self {
        Self {
            min_ts: record.min_ts,
            max_ts: record.max_ts,
            records: record.records,
            original_size: record.original_size,
            compressed_size: record.compressed_size,
            index_size: record.index_size,
            flattened: record.flattened,
        }
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct StatsRecord {
    pub stream: String,
    pub file_num: i64,
    pub min_ts: i64,
    pub max_ts: i64,
    pub records: i64,
    pub original_size: i64,
    pub compressed_size: i64,
    pub index_size: i64,
}

impl From<&StatsRecord> for StreamStats {
    fn from(record: &StatsRecord) -> Self {
        Self {
            created_at: 0,
            doc_time_min: record.min_ts,
            doc_time_max: record.max_ts,
            doc_num: record.records,
            file_num: record.file_num,
            storage_size: record.original_size,
            compressed_size: record.compressed_size,
            index_size: record.index_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct FileDeletedRecord {
    pub stream: String,
    pub date: String,
    pub file: String,
    pub index_file: bool,
    pub flattened: bool,
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct MergeJobRecord {
    pub id: i64,
    pub stream: String, // default/logs/default
    pub offsets: i64,   // 1718603746000000
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct MergeJobPendingRecord {
    pub id: i64,
    pub stream: String,
    pub num: i64,
}

#[derive(Debug, Clone, sqlx::Type, PartialEq, Default)]
#[repr(i32)]
pub enum FileListJobStatus {
    #[default]
    Pending,
    Running,
    Done,
}

#[derive(Clone, Debug, Default, sqlx::FromRow)]
pub struct FileId {
    pub id: i64,
    pub records: i64,
    pub original_size: i64,
}
