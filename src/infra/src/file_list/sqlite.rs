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

use async_trait::async_trait;
use config::{
    meta::stream::{FileKey, FileMeta, PartitionTimeLevel, StreamStats, StreamType},
    utils::parquet::parse_file_key_columns,
};
use hashbrown::HashMap;
use sqlx::{Executor, Pool, QueryBuilder, Row, Sqlite};

use crate::{
    db::sqlite::{CLIENT_RO, CLIENT_RW},
    errors::{Error, Result},
};

pub struct SqliteFileList {}

impl SqliteFileList {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SqliteFileList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FileList for SqliteFileList {
    async fn create_table(&self) -> Result<()> {
        create_table().await
    }

    async fn create_table_index(&self) -> Result<()> {
        create_table_index().await
    }

    async fn add(&self, file: &str, meta: &FileMeta) -> Result<()> {
        self.inner_add("file_list", file, meta).await
    }

    async fn add_history(&self, file: &str, meta: &FileMeta) -> Result<()> {
        self.inner_add("file_list_history", file, meta).await
    }

    async fn remove(&self, file: &str) -> Result<()> {
        self.batch_remove(&[file.to_string()]).await
    }

    async fn batch_add(&self, files: &[FileKey]) -> Result<()> {
        self.inner_batch_add("file_list", files).await
    }

    async fn batch_add_history(&self, files: &[FileKey]) -> Result<()> {
        self.inner_batch_add("file_list_history", files).await
    }

    async fn batch_remove(&self, files: &[String]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let chunks = files.chunks(100);
        for files in chunks {
            // get ids of the files
            let client = CLIENT_RW.clone();
            let client = client.lock().await;
            let pool = client.clone();
            let mut ids = Vec::with_capacity(files.len());
            for file in files {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
                let ret: Option<i64> = match sqlx::query_scalar(
                    r#"SELECT id FROM file_list WHERE stream = $1 AND date = $2 AND file = $3;"#,
                )
                .bind(stream_key)
                .bind(date_key)
                .bind(file_name)
                .fetch_one(&pool)
                .await
                {
                    Ok(v) => v,
                    Err(sqlx::Error::RowNotFound) => continue,
                    Err(e) => return Err(e.into()),
                };
                match ret {
                    Some(v) => ids.push(v.to_string()),
                    None => {
                        return Err(Error::Message(
                            "[SQLITE] query error: id should not empty from file_list".to_string(),
                        ));
                    }
                }
            }
            // delete files by ids
            if !ids.is_empty() {
                let sql = format!("DELETE FROM file_list WHERE id IN({});", ids.join(","));
                _ = pool.execute(sql.as_str()).await?;
            }
        }
        Ok(())
    }

    async fn batch_add_deleted(
        &self,
        org_id: &str,
        flattened: bool,
        created_at: i64,
        files: &[String],
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let chunks = files.chunks(100);
        for files in chunks {
            let client = CLIENT_RW.clone();
            let client = client.lock().await;
            let mut tx = client.begin().await?;
            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT INTO file_list_deleted (org, stream, date, file, flattened, created_at)",
            );
            query_builder.push_values(files, |mut b, item: &String| {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(item).expect("parse file key failed");
                b.push_bind(org_id)
                    .push_bind(stream_key)
                    .push_bind(date_key)
                    .push_bind(file_name)
                    .push_bind(flattened)
                    .push_bind(created_at);
            });
            if let Err(e) = query_builder.build().execute(&mut *tx).await {
                if let Err(e) = tx.rollback().await {
                    log::error!("[SQLITE] rollback file_list_deleted batch add error: {}", e);
                }
                return Err(e.into());
            };
            if let Err(e) = tx.commit().await {
                log::error!("[SQLITE] commit file_list_deleted batch add error: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }

    async fn batch_remove_deleted(&self, files: &[String]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let chunks = files.chunks(100);
        for files in chunks {
            // get ids of the files
            let client = CLIENT_RW.clone();
            let client = client.lock().await;
            let pool = client.clone();
            let mut ids = Vec::with_capacity(files.len());
            for file in files {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
                let ret: Option<i64> = match sqlx::query_scalar(
                    r#"SELECT id FROM file_list_deleted WHERE stream = $1 AND date = $2 AND file = $3;"#,
                )
                .bind(stream_key)
                .bind(date_key)
                .bind(file_name)
                .fetch_one(&pool)
                .await
                {
                    Ok(v) => v,
                    Err(sqlx::Error::RowNotFound) => continue,
                    Err(e) => return Err(e.into()),
                };
                match ret {
                    Some(v) => ids.push(v.to_string()),
                    None => {
                        return Err(Error::Message(
                            "[SQLITE] query error: id should not empty from file_list_deleted"
                                .to_string(),
                        ));
                    }
                }
            }
            // delete files by ids
            if !ids.is_empty() {
                let sql = format!(
                    "DELETE FROM file_list_deleted WHERE id IN({});",
                    ids.join(",")
                );
                _ = pool.execute(sql.as_str()).await?;
            }
        }
        Ok(())
    }

    async fn get(&self, file: &str) -> Result<FileMeta> {
        let pool = CLIENT_RO.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        let ret = sqlx::query_as::<_, super::FileRecord>(
            r#"
SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, flattened
    FROM file_list WHERE stream = $1 AND date = $2 AND file = $3;
            "#,
        )
        .bind(stream_key)
        .bind(date_key)
        .bind(file_name)
        .fetch_one(&pool)
        .await?;
        Ok(FileMeta::from(&ret))
    }

    async fn contains(&self, file: &str) -> Result<bool> {
        let pool = CLIENT_RO.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        let ret = sqlx::query(
            r#"SELECT * FROM file_list WHERE stream = $1 AND date = $2 AND file = $3;"#,
        )
        .bind(stream_key)
        .bind(date_key)
        .bind(file_name)
        .fetch_one(&pool)
        .await;
        if let Err(sqlx::Error::RowNotFound) = ret {
            return Ok(false);
        }
        Ok(!ret.unwrap().is_empty())
    }

    async fn update_flattened(&self, file: &str, flattened: bool) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        sqlx::query(
            r#"UPDATE file_list SET flattened = $1 WHERE stream = $2 AND date = $3 AND file = $4;"#,
        )
        .bind(flattened)
        .bind(stream_key)
        .bind(date_key)
        .bind(file_name)
        .execute(&*client)
        .await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<(String, FileMeta)>> {
        let pool = CLIENT_RO.clone();
        let ret = sqlx::query_as::<_, super::FileRecord>(
            r#"SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, flattened FROM file_list;"#,
        )
        .fetch_all(&pool)
        .await?;
        Ok(ret
            .into_iter()
            .map(|r| {
                (
                    format!("files/{}/{}/{}", r.stream, r.date, r.file),
                    FileMeta::from(&r),
                )
            })
            .collect())
    }

    async fn query(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        _time_level: PartitionTimeLevel,
        time_range: Option<(i64, i64)>,
        flattened: Option<bool>,
    ) -> Result<Vec<(String, FileMeta)>> {
        if let Some((start, end)) = time_range {
            if start == 0 && end == 0 {
                return Ok(Vec::new());
            }
        }

        let stream_key = format!("{org_id}/{stream_type}/{stream_name}");

        let pool = CLIENT_RO.clone();
        let ret = if flattened.is_some() {
            sqlx::query_as::<_, super::FileRecord>(
                r#"
SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, flattened
    FROM file_list 
    WHERE stream = $1 AND flattened = $2 LIMIT 1000;
                "#,
            )
            .bind(stream_key)
            .bind(flattened.unwrap())
            .fetch_all(&pool)
            .await
        } else {
            let (time_start, time_end) = time_range.unwrap_or((0, 0));
            sqlx::query_as::<_, super::FileRecord>(
                r#"
SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, flattened
    FROM file_list 
    WHERE stream = $1 AND max_ts >= $2 AND min_ts <= $3;
                "#,
            )
            .bind(stream_key)
            .bind(time_start)
            .bind(time_end)
            .fetch_all(&pool)
            .await
        };
        Ok(ret?
            .iter()
            .map(|r| {
                (
                    format!("files/{}/{}/{}", r.stream, r.date, r.file),
                    r.into(),
                )
            })
            .collect())
    }

    async fn query_deleted(
        &self,
        org_id: &str,
        time_max: i64,
        limit: i64,
    ) -> Result<Vec<(String, bool)>> {
        if time_max == 0 {
            return Ok(Vec::new());
        }
        let pool = CLIENT_RO.clone();
        let ret = sqlx::query_as::<_, super::FileDeletedRecord>(
            r#"SELECT stream, date, file, flattened FROM file_list_deleted WHERE org = $1 AND created_at < $2 LIMIT $3;"#,
        )
        .bind(org_id)
        .bind(time_max)
        .bind(limit)
        .fetch_all(&pool)
        .await?;
        Ok(ret
            .iter()
            .map(|r| {
                (
                    format!("files/{}/{}/{}", r.stream, r.date, r.file),
                    r.flattened,
                )
            })
            .collect())
    }

    async fn get_min_ts(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
    ) -> Result<i64> {
        let stream_key = format!("{org_id}/{stream_type}/{stream_name}");
        let min_ts = config::utils::time::BASE_TIME.timestamp_micros();
        let pool = CLIENT_RO.clone();
        let ret: Option<i64> = sqlx::query_scalar(
            r#"SELECT MIN(min_ts) AS id FROM file_list WHERE stream = $1 AND min_ts > $2;"#,
        )
        .bind(stream_key)
        .bind(min_ts)
        .fetch_one(&pool)
        .await?;
        Ok(ret.unwrap_or_default())
    }

    async fn get_max_pk_value(&self) -> Result<i64> {
        let pool = CLIENT_RO.clone();
        let ret: Option<i64> = sqlx::query_scalar(r#"SELECT MAX(id) AS id FROM file_list;"#)
            .fetch_one(&pool)
            .await?;
        Ok(ret.unwrap_or_default())
    }

    async fn stats(
        &self,
        org_id: &str,
        stream_type: Option<StreamType>,
        stream_name: Option<&str>,
        pk_value: Option<(i64, i64)>,
    ) -> Result<Vec<(String, StreamStats)>> {
        let (field, value) = if stream_type.is_some() && stream_name.is_some() {
            (
                "stream",
                format!(
                    "{}/{}/{}",
                    org_id,
                    stream_type.unwrap(),
                    stream_name.unwrap()
                ),
            )
        } else {
            ("org", org_id.to_string())
        };
        let sql = format!(
            r#"
SELECT stream, MIN(min_ts) as min_ts, MAX(max_ts) as max_ts, COUNT(*) as file_num, SUM(records) as records, SUM(original_size) as original_size, SUM(compressed_size) as compressed_size
    FROM file_list 
    WHERE {field} = '{value}'
            "#,
        );
        let sql = match pk_value {
            None => format!("{} GROUP BY stream", sql),
            Some((0, 0)) => format!("{} GROUP BY stream", sql),
            Some((min, max)) => {
                format!("{} AND id > {} AND id <= {} GROUP BY stream", sql, min, max)
            }
        };
        let pool = CLIENT_RO.clone();
        let ret = sqlx::query_as::<_, super::StatsRecord>(&sql)
            .fetch_all(&pool)
            .await?;
        Ok(ret
            .iter()
            .map(|r| (r.stream.to_owned(), r.into()))
            .collect())
    }

    async fn get_stream_stats(
        &self,
        org_id: &str,
        stream_type: Option<StreamType>,
        stream_name: Option<&str>,
    ) -> Result<Vec<(String, StreamStats)>> {
        let sql = if stream_type.is_some() && stream_name.is_some() {
            format!(
                "SELECT * FROM stream_stats WHERE stream = '{}/{}/{}';",
                org_id,
                stream_type.unwrap(),
                stream_name.unwrap()
            )
        } else {
            format!("SELECT * FROM stream_stats WHERE org = '{}';", org_id)
        };
        let pool = CLIENT_RO.clone();
        let ret = sqlx::query_as::<_, super::StatsRecord>(&sql)
            .fetch_all(&pool)
            .await?;
        Ok(ret
            .iter()
            .map(|r| (r.stream.to_owned(), r.into()))
            .collect())
    }

    async fn del_stream_stats(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
    ) -> Result<()> {
        let sql = format!(
            "DELETE FROM stream_stats WHERE stream = '{}/{}/{}';",
            org_id, stream_type, stream_name
        );
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(&sql).execute(&*client).await?;
        Ok(())
    }

    async fn set_stream_stats(
        &self,
        org_id: &str,
        streams: &[(String, StreamStats)],
    ) -> Result<()> {
        let old_stats = super::get_stream_stats(org_id, None, None).await?;
        let old_stats = old_stats.into_iter().collect::<HashMap<_, _>>();
        let mut new_streams = Vec::new();
        let mut update_streams = Vec::with_capacity(streams.len());
        for (stream_key, item) in streams {
            let mut stats = match old_stats.get(stream_key) {
                Some(s) => s.to_owned(),
                None => {
                    new_streams.push(stream_key);
                    StreamStats::default()
                }
            };
            stats.add_stream_stats(item);
            update_streams.push((stream_key, stats));
        }

        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let mut tx = client.begin().await?;
        for stream_key in new_streams {
            let org_id = stream_key[..stream_key.find('/').unwrap()].to_string();
            if let Err(e) = sqlx::query(
                r#"
    INSERT INTO stream_stats 
    (org, stream, file_num, min_ts, max_ts, records, original_size, compressed_size)
    VALUES ($1, $2, 0, 0, 0, 0, 0, 0);
                "#,
            )
            .bind(org_id)
            .bind(stream_key)
            .execute(&mut *tx)
            .await
            {
                if let Err(e) = tx.rollback().await {
                    log::error!("[SQLITE] rollback insert stream stats error: {}", e);
                }
                return Err(e.into());
            }
        }
        if let Err(e) = tx.commit().await {
            log::error!("[SQLITE] commit set stream stats error: {}", e);
            return Err(e.into());
        }

        let mut tx = client.begin().await?;
        for (stream_key, stats) in update_streams {
            if let Err(e) = sqlx::query(
                r#"
    UPDATE stream_stats 
    SET file_num = $1, min_ts = $2, max_ts = $3, records = $4, original_size = $5, compressed_size = $6
    WHERE stream = $7;
                "#,
            )
            .bind(stats.file_num)
            .bind(stats.doc_time_min)
            .bind(stats.doc_time_max)
            .bind(stats.doc_num)
            .bind(stats.storage_size as i64)
            .bind(stats.compressed_size as i64)
            .bind(stream_key)
            .execute(&mut *tx)
            .await
            {
                if let Err(e) = tx.rollback().await {
                    log::error!("[SQLITE] rollback set stream stats error: {}", e);
                }
                return Err(e.into());
            }
        }
        if let Err(e) = tx.commit().await {
            log::error!("[SQLITE] commit set stream stats error: {}", e);
            return Err(e.into());
        }

        Ok(())
    }

    async fn reset_stream_stats_min_ts(
        &self,
        _org_id: &str,
        stream: &str,
        min_ts: i64,
    ) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(r#"UPDATE stream_stats SET min_ts = $1 WHERE stream = $2;"#)
            .bind(min_ts)
            .bind(stream)
            .execute(&*client)
            .await?;
        Ok(())
    }

    async fn reset_stream_stats(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(r#"UPDATE stream_stats SET file_num = 0, min_ts = 0, max_ts = 0, records = 0, original_size = 0, compressed_size = 0;"#)
        .execute(&*client)
       .await?;
        Ok(())
    }

    async fn len(&self) -> usize {
        let pool = CLIENT_RO.clone();
        let ret = match sqlx::query(r#"SELECT COUNT(*) as num FROM file_list;"#)
            .fetch_one(&pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[SQLITE] get file list len error: {}", e);
                return 0;
            }
        };
        match ret.try_get::<i64, &str>("num") {
            Ok(v) => v as usize,
            _ => 0,
        }
    }

    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    async fn clear(&self) -> Result<()> {
        Ok(())
    }

    async fn add_job(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream: &str,
        offset: i64,
    ) -> Result<()> {
        let stream_key = format!("{org_id}/{stream_type}/{stream}");
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        match sqlx::query(
            "INSERT INTO file_list_jobs (org, stream, offsets, status, node, started_at, updated_at) VALUES ($1, $2, $3, $4, '', 0, 0);",
        )
        .bind(org_id)
        .bind(stream_key)
        .bind(offset)
        .bind(super::FileListJobStatus::Pending)
        .execute(&*client)
        .await
        {
            Err(sqlx::Error::Database(e)) => if e.is_unique_violation() {
                Ok(())
            } else {
                Err(Error::Message(e.to_string()))
            },
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }

    async fn get_pending_jobs(&self, node: &str, limit: i64) -> Result<Vec<super::MergeJobRecord>> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let mut tx = client.begin().await?;
        // get pending jobs group by stream and order by num desc
        let ret = match sqlx::query_as::<_, super::MergeJobPendingRecord>(
            r#"
SELECT stream, max(id) as id, COUNT(*) AS num
    FROM file_list_jobs 
    WHERE status = $1 
    GROUP BY stream 
    ORDER BY num DESC 
    LIMIT $2;"#,
        )
        .bind(super::FileListJobStatus::Pending)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        {
            Ok(v) => v,
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    log::error!(
                        "[SQLITE] rollback select file_list_jobs pending jobs for update error: {e}"
                    );
                }
                return Err(e.into());
            }
        };
        // update jobs status to running
        let ids = ret.iter().map(|r| r.id.to_string()).collect::<Vec<_>>();
        if ids.is_empty() {
            if let Err(e) = tx.rollback().await {
                log::error!("[SQLITE] rollback select file_list_jobs pending jobs error: {e}");
            }
            return Ok(Vec::new());
        }
        let sql = format!(
            "UPDATE file_list_jobs SET status = $1, node = $2, started_at = $3, updated_at = $4 WHERE id IN ({});",
            ids.join(",")
        );
        let now = config::utils::time::now_micros();
        if let Err(e) = sqlx::query(&sql)
            .bind(super::FileListJobStatus::Running)
            .bind(node)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
        {
            if let Err(e) = tx.rollback().await {
                log::error!("[SQLITE] rollback update file_list_jobs status error: {e}");
            }
            return Err(e.into());
        }
        // get jobs by ids
        let sql = format!(
            "SELECT * FROM file_list_jobs WHERE id IN ({});",
            ids.join(",")
        );
        let ret = match sqlx::query_as::<_, super::MergeJobRecord>(&sql)
            .fetch_all(&mut *tx)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    log::error!("[SQLITE] rollback select file_list_jobs by ids error: {e}");
                }
                return Err(e.into());
            }
        };
        if let Err(e) = tx.commit().await {
            log::error!("[SQLITE] commit select file_list_jobs pending jobs error: {e}");
            return Err(e.into());
        }
        Ok(ret)
    }

    async fn set_job_pending(&self, ids: &[i64]) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let sql = format!(
            "UPDATE file_list_jobs SET status = $1 WHERE id IN ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        sqlx::query(&sql)
            .bind(super::FileListJobStatus::Pending)
            .execute(&*client)
            .await?;
        Ok(())
    }

    async fn set_job_done(&self, id: i64) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(r#"UPDATE file_list_jobs SET status = $1, updated_at = $2 WHERE id = $3;"#)
            .bind(super::FileListJobStatus::Done)
            .bind(config::utils::time::now_micros())
            .bind(id)
            .execute(&*client)
            .await?;
        Ok(())
    }

    async fn update_running_jobs(&self, id: i64) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        sqlx::query(r#"UPDATE file_list_jobs SET updated_at = $1 WHERE id = $2;"#)
            .bind(config::utils::time::now_micros())
            .bind(id)
            .execute(&*client)
            .await?;
        Ok(())
    }

    async fn check_running_jobs(&self, before_date: i64) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let ret = sqlx::query(
            r#"UPDATE file_list_jobs SET status = $1 WHERE status = $2 AND updated_at < $3;"#,
        )
        .bind(super::FileListJobStatus::Pending)
        .bind(super::FileListJobStatus::Running)
        .bind(before_date)
        .execute(&*client)
        .await?;
        if ret.rows_affected() > 0 {
            log::warn!("[SQLITE] reset running jobs status to pending");
        }
        Ok(())
    }

    async fn clean_done_jobs(&self, before_date: i64) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let ret =
            sqlx::query(r#"DELETE FROM file_list_jobs WHERE status = $1 AND updated_at < $2;"#)
                .bind(super::FileListJobStatus::Done)
                .bind(before_date)
                .execute(&*client)
                .await?;
        if ret.rows_affected() > 0 {
            log::warn!("[SQLITE] clean done jobs");
        }
        Ok(())
    }
}

impl SqliteFileList {
    async fn inner_add(&self, table: &str, file: &str, meta: &FileMeta) -> Result<()> {
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        let org_id = stream_key[..stream_key.find('/').unwrap()].to_string();
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        match  sqlx::query(
            format!(r#"
INSERT INTO {table} (org, stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, flattened)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11);
        "#).as_str(),
    )
        .bind(org_id)
        .bind(stream_key)
        .bind(date_key)
        .bind(file_name)
        .bind(false)
        .bind(meta.min_ts)
        .bind(meta.max_ts)
        .bind(meta.records)
        .bind(meta.original_size)
        .bind(meta.compressed_size)
        .bind(meta.flattened)
        .execute(&*client)
        .await {
            Err(sqlx::Error::Database(e)) => if e.is_unique_violation() {
                  Ok(())
            } else {
                  Err(Error::Message(e.to_string()))
            },
            Err(e) =>  Err(e.into()),
            Ok(_) => Ok(()),
        }
    }

    async fn inner_batch_add(&self, table: &str, files: &[FileKey]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let chunks = files.chunks(100);
        for files in chunks {
            let client = CLIENT_RW.clone();
            let client = client.lock().await;
            let mut tx = client.begin().await?;
            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                format!("INSERT INTO {table} (org, stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, flattened)").as_str(),
            );
            query_builder.push_values(files, |mut b, item| {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(&item.key).expect("parse file key failed");
                let org_id = stream_key[..stream_key.find('/').unwrap()].to_string();
                b.push_bind(org_id)
                    .push_bind(stream_key)
                    .push_bind(date_key)
                    .push_bind(file_name)
                    .push_bind(false)
                    .push_bind(item.meta.min_ts)
                    .push_bind(item.meta.max_ts)
                    .push_bind(item.meta.records)
                    .push_bind(item.meta.original_size)
                    .push_bind(item.meta.compressed_size)
                    .push_bind(item.meta.flattened);
            });
            let need_single_insert = match query_builder.build().execute(&mut *tx).await {
                Ok(_) => false,
                Err(sqlx::Error::Database(e)) => {
                    if e.is_unique_violation() {
                        true
                    } else {
                        if let Err(e) = tx.rollback().await {
                            log::error!("[SQLITE] rollback {table} batch add error: {}", e);
                        }
                        return Err(Error::Message(e.to_string()));
                    }
                }
                Err(e) => {
                    if let Err(e) = tx.rollback().await {
                        log::error!("[SQLITE] rollback {table} batch add error: {}", e);
                    }
                    return Err(e.into());
                }
            };
            if need_single_insert {
                if let Err(e) = tx.rollback().await {
                    log::error!("[SQLITE] rollback {table} batch add error: {}", e);
                    return Err(e.into());
                }
                // release lock
                drop(client);
                // add file one by one
                for item in files {
                    if let Err(e) = self.inner_add(table, &item.key, &item.meta).await {
                        log::error!("[SQLITE] single insert {table} add error: {}", e);
                        return Err(e);
                    }
                }
            } else if let Err(e) = tx.commit().await {
                log::error!("[SQLITE] commit {table} batch add error: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }
}

pub async fn create_table() -> Result<()> {
    let client = CLIENT_RW.clone();
    let client = client.lock().await;
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list
(
    id        INTEGER not null primary key autoincrement,
    org       VARCHAR not null,
    stream    VARCHAR not null,
    date      VARCHAR not null,
    file      VARCHAR not null,
    deleted   BOOLEAN default false not null,
    flattened BOOLEAN default false not null,
    min_ts    BIGINT not null,
    max_ts    BIGINT not null,
    records   BIGINT not null,
    original_size   BIGINT not null,
    compressed_size BIGINT not null
);
        "#,
    )
    .execute(&*client)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list_history
(
    id        INTEGER not null primary key autoincrement,
    org       VARCHAR not null,
    stream    VARCHAR not null,
    date      VARCHAR not null,
    file      VARCHAR not null,
    deleted   BOOLEAN default false not null,
    flattened BOOLEAN default false not null,
    min_ts    BIGINT not null,
    max_ts    BIGINT not null,
    records   BIGINT not null,
    original_size   BIGINT not null,
    compressed_size BIGINT not null
);
        "#,
    )
    .execute(&*client)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list_deleted
(
    id         INTEGER not null primary key autoincrement,
    org        VARCHAR not null,
    stream     VARCHAR not null,
    date       VARCHAR not null,
    file       VARCHAR not null,
    flattened  BOOLEAN default false not null,
    created_at BIGINT not null
);
        "#,
    )
    .execute(&*client)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list_jobs
(
    id         INTEGER not null primary key autoincrement,
    org        VARCHAR not null,
    stream     VARCHAR not null,
    offsets    BIGINT not null,
    status     INT not null,
    node       VARCHAR not null,
    started_at BIGINT not null,
    updated_at BIGINT not null
);
        "#,
    )
    .execute(&*client)
    .await?;

    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS stream_stats
(
    id      INTEGER not null primary key autoincrement,
    org     VARCHAR not null,
    stream  VARCHAR not null,
    file_num BIGINT not null,
    min_ts   BIGINT not null,
    max_ts   BIGINT not null,
    records  BIGINT not null,
    original_size   BIGINT not null,
    compressed_size BIGINT not null
);
        "#,
    )
    .execute(&*client)
    .await?;

    // create column flattened for old version <= 0.10.5
    let column = "flattened";
    let data_type = "BOOLEAN default false not null";
    add_column(&client, "file_list", column, data_type).await?;
    add_column(&client, "file_list_history", column, data_type).await?;
    add_column(&client, "file_list_deleted", column, data_type).await?;

    // create column started_at for old version <= 0.10.8
    let column = "started_at";
    let data_type = "BIGINT default 0 not null";
    add_column(&client, "file_list_jobs", column, data_type).await?;

    Ok(())
}

pub async fn create_table_index() -> Result<()> {
    let sqls = vec![
        (
            "file_list",
            "CREATE INDEX IF NOT EXISTS file_list_org_idx on file_list (org);",
        ),
        (
            "file_list",
            "CREATE INDEX IF NOT EXISTS file_list_stream_ts_idx on file_list (stream, max_ts, min_ts);",
        ),
        (
            "file_list_history",
            "CREATE INDEX IF NOT EXISTS file_list_history_org_idx on file_list_history (org);",
        ),
        (
            "file_list_history",
            "CREATE INDEX IF NOT EXISTS file_list_history_stream_ts_idx on file_list_history (stream, max_ts, min_ts);",
        ),
        (
            "file_list_history",
            "CREATE UNIQUE INDEX IF NOT EXISTS file_list_history_stream_file_idx on file_list_history (stream, date, file);",
        ),
        (
            "file_list_deleted",
            "CREATE INDEX IF NOT EXISTS file_list_deleted_created_at_idx on file_list_deleted (org, created_at);",
        ),
        (
            "file_list_deleted",
            "CREATE INDEX IF NOT EXISTS file_list_deleted_stream_date_file_idx on file_list_deleted (stream, date, file);",
        ),
        (
            "file_list_jobs",
            "CREATE UNIQUE INDEX IF NOT EXISTS file_list_jobs_stream_offsets_idx on file_list_jobs (stream, offsets);",
        ),
        (
            "file_list_jobs",
            "CREATE INDEX IF NOT EXISTS file_list_jobs_stream_status_idx on file_list_jobs (status, stream);",
        ),
        (
            "stream_stats",
            "CREATE INDEX IF NOT EXISTS stream_stats_org_idx on stream_stats (org);",
        ),
        (
            "stream_stats",
            "CREATE UNIQUE INDEX IF NOT EXISTS stream_stats_stream_idx on stream_stats (stream);",
        ),
    ];

    let client = CLIENT_RW.clone();
    let client = client.lock().await;
    for (table, sql) in sqls {
        if let Err(e) = sqlx::query(sql).execute(&*client).await {
            log::error!("[SQLITE] create table {} index error: {}", table, e);
            return Err(e.into());
        }
    }

    // create UNIQUE index for file_list
    let unique_index_sql = r#"CREATE UNIQUE INDEX IF NOT EXISTS file_list_stream_file_idx on file_list (stream, date, file);"#;
    if let Err(e) = sqlx::query(unique_index_sql).execute(&*client).await {
        if !e.to_string().contains("UNIQUE constraint failed") {
            return Err(e.into());
        }
        // delete duplicate records
        log::warn!("[SQLITE] starting delete duplicate records");
        let ret = sqlx::query(
                r#"SELECT stream, date, file, min(id) as id FROM file_list GROUP BY stream, date, file HAVING COUNT(*) > 1;"#,
            ).fetch_all(&*client).await?;
        log::warn!("[SQLITE] total: {} duplicate records", ret.len());
        for (i, r) in ret.iter().enumerate() {
            let stream = r.get::<String, &str>("stream");
            let date = r.get::<String, &str>("date");
            let file = r.get::<String, &str>("file");
            let id = r.get::<i64, &str>("id");
            sqlx::query(
                    r#"DELETE FROM file_list WHERE id != $1 AND stream = $2 AND date = $3 AND file = $4;"#,
                ).bind(id).bind(stream).bind(date).bind(file).execute(&*client).await?;
            if i / 1000 == 0 {
                log::warn!("[SQLITE] delete duplicate records: {}/{}", i, ret.len());
            }
        }
        log::warn!(
            "[SQLITE] delete duplicate records: {}/{}",
            ret.len(),
            ret.len()
        );
        // create index again
        sqlx::query(unique_index_sql).execute(&*client).await?;
        log::warn!("[SQLITE] create table index(file_list_stream_file_idx) succeed");
    }

    // delete trigger for old version
    // compatible for old version <= 0.6.4
    sqlx::query(r#"DROP TRIGGER IF EXISTS update_stream_stats_delete;"#)
        .execute(&*client)
        .await?;

    Ok(())
}

async fn add_column(
    client: &Pool<Sqlite>,
    table: &str,
    column: &str,
    data_type: &str,
) -> Result<()> {
    // Attempt to add the column, ignoring the error if the column already exists
    let check_sql = format!("ALTER TABLE {table} ADD COLUMN {column} {data_type};");
    if let Err(e) = sqlx::query(&check_sql).execute(client).await {
        // Check if the error is about the duplicate column
        if !e.to_string().contains("duplicate column name") {
            // If the error is not about the duplicate column, return it
            return Err(e.into());
        }
    }
    Ok(())
}
