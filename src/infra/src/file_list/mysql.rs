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
    get_config,
    meta::stream::{
        FileKey, FileListDeleted, FileMeta, PartitionTimeLevel, StreamStats, StreamType,
    },
    metrics::{DB_QUERY_NUMS, DB_QUERY_TIME},
    utils::{
        hash::Sum64,
        parquet::parse_file_key_columns,
        time::{end_of_the_day, DAY_MICRO_SECS},
    },
};
use hashbrown::HashMap;
use sqlx::{Executor, MySql, QueryBuilder, Row};

use crate::{
    db::{
        mysql::{create_index, CLIENT},
        IndexStatement,
    },
    errors::{DbError, Error, Result},
};

pub struct MysqlFileList {}

impl MysqlFileList {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MysqlFileList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FileList for MysqlFileList {
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
        let pool = CLIENT.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        DB_QUERY_NUMS
            .with_label_values(&["delete", "file_list"])
            .inc();
        sqlx::query(r#"DELETE FROM file_list WHERE stream = ? AND date = ? AND file = ?;"#)
            .bind(stream_key)
            .bind(date_key)
            .bind(file_name)
            .execute(&pool)
            .await?;
        Ok(())
    }

    async fn batch_add(&self, files: &[FileKey]) -> Result<()> {
        self.inner_batch_add("file_list", files).await
    }

    async fn batch_add_with_id(&self, _files: &[(i64, &FileKey)]) -> Result<()> {
        unimplemented!("Unsupported")
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
            let pool = CLIENT.clone();
            let mut ids = Vec::with_capacity(files.len());
            for file in files {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
                DB_QUERY_NUMS
                    .with_label_values(&["select", "file_list"])
                    .inc();
                let start = std::time::Instant::now();
                let query_res = sqlx::query_scalar(
                    r#"SELECT id FROM file_list WHERE stream = ? AND date = ? AND file = ?"#,
                )
                .bind(stream_key)
                .bind(date_key)
                .bind(file_name)
                .fetch_one(&pool)
                .await;
                let time = start.elapsed().as_secs_f64();
                DB_QUERY_TIME
                    .with_label_values(&["select_id", "file_list"])
                    .observe(time);
                let ret: Option<i64> = match query_res {
                    Ok(v) => v,
                    Err(sqlx::Error::RowNotFound) => continue,
                    Err(e) => return Err(e.into()),
                };
                match ret {
                    Some(v) => ids.push(v.to_string()),
                    None => {
                        return Err(Error::Message(
                            "[MYSQL] query error: id should not empty from file_list".to_string(),
                        ));
                    }
                }
            }
            // delete files by ids
            if !ids.is_empty() {
                let sql = format!("DELETE FROM file_list WHERE id IN({});", ids.join(","));
                DB_QUERY_NUMS
                    .with_label_values(&["delete", "file_list"])
                    .inc();
                let start = std::time::Instant::now();
                _ = pool.execute(sql.as_str()).await?;
                let time = start.elapsed().as_secs_f64();
                DB_QUERY_TIME
                    .with_label_values(&["delete_id", "file_list"])
                    .observe(time);
            }
        }
        Ok(())
    }

    async fn batch_add_deleted(
        &self,
        org_id: &str,
        created_at: i64,
        files: &[FileListDeleted],
    ) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        let pool = CLIENT.clone();
        let chunks = files.chunks(100);
        for files in chunks {
            let mut tx = pool.begin().await?;
            let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
                "INSERT INTO file_list_deleted (org, stream, date, file, index_file, flattened, created_at)",
            );
            query_builder.push_values(files, |mut b, item| {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(&item.file).expect("parse file key failed");
                b.push_bind(org_id)
                    .push_bind(stream_key)
                    .push_bind(date_key)
                    .push_bind(file_name)
                    .push_bind(item.index_file)
                    .push_bind(item.flattened)
                    .push_bind(created_at);
            });
            DB_QUERY_NUMS
                .with_label_values(&["insert", "file_list_deleted"])
                .inc();
            if let Err(e) = query_builder.build().execute(&mut *tx).await {
                if let Err(e) = tx.rollback().await {
                    log::error!("[MYSQL] rollback file_list_deleted batch add error: {}", e);
                }
                return Err(e.into());
            };
            if let Err(e) = tx.commit().await {
                log::error!("[MYSQL] commit file_list_deleted batch add error: {}", e);
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
            let pool = CLIENT.clone();
            let mut ids = Vec::with_capacity(files.len());
            for file in files {
                let (stream_key, date_key, file_name) =
                    parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
                DB_QUERY_NUMS
                    .with_label_values(&["select", "file_list_deleted"])
                    .inc();
                let ret: Option<i64> = match sqlx::query_scalar(
                    r#"SELECT id FROM file_list_deleted WHERE stream = ? AND date = ? AND file = ?"#,
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
                            "[MYSQL] query error: id should not empty from file_list_deleted"
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
                DB_QUERY_NUMS
                    .with_label_values(&["delete", "file_list_deleted"])
                    .inc();
                _ = pool.execute(sql.as_str()).await?;
            }
        }
        Ok(())
    }

    async fn get(&self, file: &str) -> Result<FileMeta> {
        let pool = CLIENT.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let start = std::time::Instant::now();
        let ret = sqlx::query_as::<_, super::FileRecord>(
            r#"
SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, index_size, flattened
    FROM file_list WHERE stream = ? AND date = ? AND file = ?;
            "#,
        )
        .bind(stream_key)
        .bind(date_key)
        .bind(file_name)
        .fetch_one(&pool)
        .await;
        let time = start.elapsed().as_secs_f64();
        DB_QUERY_TIME
            .with_label_values(&["get_single", "file_list"])
            .observe(time);
        Ok(FileMeta::from(&ret?))
    }

    async fn contains(&self, file: &str) -> Result<bool> {
        let pool = CLIENT.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let start = std::time::Instant::now();
        let ret =
            sqlx::query(r#"SELECT * FROM file_list WHERE stream = ? AND date = ? AND file = ?;"#)
                .bind(stream_key)
                .bind(date_key)
                .bind(file_name)
                .fetch_one(&pool)
                .await;
        let time = start.elapsed().as_secs_f64();
        DB_QUERY_TIME
            .with_label_values(&["contains", "file_list"])
            .observe(time);
        if let Err(sqlx::Error::RowNotFound) = ret {
            return Ok(false);
        }
        Ok(!ret.unwrap().is_empty())
    }

    async fn update_flattened(&self, file: &str, flattened: bool) -> Result<()> {
        let pool = CLIENT.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        DB_QUERY_NUMS
            .with_label_values(&["update", "file_list"])
            .inc();
        sqlx::query(
            r#"UPDATE file_list SET flattened = ? WHERE stream = ? AND date = ? AND file = ?;"#,
        )
        .bind(flattened)
        .bind(stream_key)
        .bind(date_key)
        .bind(file_name)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn list(&self) -> Result<Vec<(String, FileMeta)>> {
        return Ok(vec![]); // disallow list all data
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

        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let start = std::time::Instant::now();
        let ret = if flattened.is_some() {
            sqlx::query_as::<_, super::FileRecord>(
                r#"
SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, index_size, flattened
    FROM file_list
    WHERE stream = ? AND flattened = ? LIMIT 1000;
                "#,
            )
            .bind(stream_key)
            .bind(flattened.unwrap())
            .fetch_all(&pool)
            .await
        } else {
            let (time_start, time_end) = time_range.unwrap_or((0, 0));
            let max_ts_upper_bound = super::calculate_max_ts_upper_bound(time_end, stream_type);
            sqlx::query_as::<_, super::FileRecord>(
                r#"
SELECT stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, index_size, flattened
    FROM file_list
    WHERE stream = ? AND max_ts >= ? AND max_ts <= ? AND min_ts <= ?;
                "#,
            )
            .bind(stream_key)
            .bind(time_start)
            .bind(max_ts_upper_bound)
            .bind(time_end)
            .fetch_all(&pool)
            .await
        };
        let time = start.elapsed().as_secs_f64();
        DB_QUERY_TIME
            .with_label_values(&["get_multiple", "file_list"])
            .observe(time);
        Ok(ret?
            .into_iter()
            .map(|r| {
                (
                    "files/".to_string() + &r.stream + "/" + &r.date + "/" + &r.file,
                    FileMeta::from(&r),
                )
            })
            .collect())
    }

    async fn query_by_ids(&self, ids: &[i64]) -> Result<Vec<(i64, String, FileMeta)>> {
        if ids.is_empty() {
            return Ok(Vec::default());
        }
        let mut ret = Vec::new();
        let pool = CLIENT.clone();

        for chunk in ids.chunks(get_config().limit.file_list_id_batch_size) {
            if chunk.is_empty() {
                continue;
            }
            let ids = chunk
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            let query_str = format!(
                "SELECT id, stream, date, file, min_ts, max_ts, records, original_size, compressed_size, index_size FROM file_list WHERE id IN ({ids})"
            );
            DB_QUERY_NUMS
                .with_label_values(&["select", "file_list"])
                .inc();
            let start = std::time::Instant::now();
            let res = sqlx::query_as::<_, super::FileRecord>(&query_str)
                .fetch_all(&pool)
                .await;
            let time = start.elapsed().as_secs_f64();
            DB_QUERY_TIME
                .with_label_values(&["get_by_id", "file_list"])
                .observe(time);
            ret.extend_from_slice(&res?);
        }

        Ok(ret
            .into_iter()
            .map(|r| {
                (
                    r.id,
                    "files/".to_string() + &r.stream + "/" + &r.date + "/" + &r.file,
                    FileMeta::from(&r),
                )
            })
            .collect())
    }

    async fn query_ids(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_range: Option<(i64, i64)>,
    ) -> Result<Vec<super::FileId>> {
        if let Some((start, end)) = time_range {
            if start == 0 && end == 0 {
                return Ok(Vec::new());
            }
        }

        let stream_key = format!("{org_id}/{stream_type}/{stream_name}");
        let (time_start, time_end) = time_range.unwrap_or((0, 0));
        let start = std::time::Instant::now();

        let day_partitions = if time_end - time_start <= DAY_MICRO_SECS
            || time_end - time_start > DAY_MICRO_SECS * 30
            || !get_config().limit.file_list_multi_thread
        {
            vec![(time_start, time_end)]
        } else {
            let mut partitions = Vec::new();
            let mut start = time_start;
            while start < time_end {
                let end_of_day = std::cmp::min(end_of_the_day(start), time_end);
                partitions.push((start, end_of_day));
                start = end_of_day + 1; // next day, use end_of_day + 1 microsecond
            }
            partitions
        };
        log::debug!("file_list day_partitions: {:?}", day_partitions);

        let mut tasks = Vec::with_capacity(day_partitions.len());

        for (time_start, time_end) in day_partitions {
            let stream_key = stream_key.clone();
            tasks.push(tokio::task::spawn(async move {
                let pool = CLIENT.clone();
                DB_QUERY_NUMS
                .with_label_values(&["select", "file_list"])
                .inc();
                    let max_ts_upper_bound = super::calculate_max_ts_upper_bound(time_end, stream_type);
                    let query = "SELECT id, records, original_size FROM file_list WHERE stream = ? AND max_ts >= ? AND max_ts <= ? AND min_ts <= ?;";
                    sqlx::query_as::<_, super::FileId>(query)
                    .bind(stream_key)
                    .bind(time_start)
                    .bind(max_ts_upper_bound)
                    .bind(time_end)
                    .fetch_all(&pool)
                    .await
        }));
        }

        let mut rets = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(r)) => rets.extend(r),
                Ok(Err(e)) => {
                    return Err(e.into());
                }
                Err(e) => {
                    return Err(Error::Message(e.to_string()));
                }
            };
        }
        let time = start.elapsed().as_secs_f64();
        DB_QUERY_TIME
            .with_label_values(&["get_ids", "file_list"])
            .observe(time);
        Ok(rets)
    }

    async fn query_old_data_hours(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_range: Option<(i64, i64)>,
    ) -> Result<Vec<String>> {
        if let Some((start, end)) = time_range {
            if start == 0 && end == 0 {
                return Ok(Vec::new());
            }
        }

        let stream_key = format!("{org_id}/{stream_type}/{stream_name}");

        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let start = std::time::Instant::now();
        let (time_start, time_end) = time_range.unwrap_or((0, 0));
        let cfg = get_config();
        let max_ts_upper_bound = super::calculate_max_ts_upper_bound(time_end, stream_type);
        let sql = r#"
SELECT date
    FROM file_list 
    WHERE stream = ? AND max_ts >= ? AND max_ts <= ? AND min_ts <= ? AND records < ?
    GROUP BY date HAVING count(*) >= ?;
            "#;

        let ret = sqlx::query(sql)
            .bind(stream_key)
            .bind(time_start)
            .bind(max_ts_upper_bound)
            .bind(time_end)
            .bind(cfg.compact.old_data_min_records)
            .bind(cfg.compact.old_data_min_files)
            .fetch_all(&pool)
            .await?;

        let time = start.elapsed().as_secs_f64();
        DB_QUERY_TIME
            .with_label_values(&["query_old_data_hours", "file_list"])
            .observe(time);
        Ok(ret
            .into_iter()
            .map(|r| r.try_get::<String, &str>("date").unwrap_or_default())
            .collect())
    }

    async fn query_deleted(
        &self,
        org_id: &str,
        time_max: i64,
        limit: i64,
    ) -> Result<Vec<FileListDeleted>> {
        if time_max == 0 {
            return Ok(Vec::new());
        }
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list_deleted"])
            .inc();
        let ret = sqlx::query_as::<_, super::FileDeletedRecord>(
            r#"SELECT stream, date, file, index_file, flattened FROM file_list_deleted WHERE org = ? AND created_at < ? LIMIT ?;"#,
        )
        .bind(org_id)
        .bind(time_max)
        .bind(limit)
        .fetch_all(&pool)
        .await?;
        Ok(ret
            .iter()
            .map(|r| FileListDeleted {
                file: format!("files/{}/{}/{}", r.stream, r.date, r.file),
                index_file: r.index_file,
                flattened: r.flattened,
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let ret: Option<i64> = sqlx::query_scalar(
            r#"SELECT MIN(min_ts) AS id FROM file_list WHERE stream = ? AND min_ts > ?;"#,
        )
        .bind(stream_key)
        .bind(min_ts)
        .fetch_one(&pool)
        .await?;
        Ok(ret.unwrap_or_default())
    }

    async fn get_max_pk_value(&self) -> Result<i64> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let ret: Option<i64> = sqlx::query_scalar(r#"SELECT MAX(id) AS id FROM file_list;"#)
            .fetch_one(&pool)
            .await?;
        Ok(ret.unwrap_or_default())
    }

    async fn get_min_pk_value(&self) -> Result<i64> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let ret: Option<i64> = sqlx::query_scalar(r#"SELECT MIN(id) AS id FROM file_list;"#)
            .fetch_one(&pool)
            .await?;
        Ok(ret.unwrap_or_default())
    }

    async fn clean_by_min_pk_value(&self, _val: i64) -> Result<()> {
        Ok(()) // do nothing
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
SELECT stream, MIN(min_ts) AS min_ts, MAX(max_ts) AS max_ts, CAST(COUNT(*) AS SIGNED) AS file_num, 
    CAST(SUM(records) AS SIGNED) AS records, CAST(SUM(original_size) AS SIGNED) AS original_size, CAST(SUM(compressed_size) AS SIGNED) AS compressed_size, CAST(SUM(index_size) AS SIGNED) AS index_size
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "stream_stats"])
            .inc();
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
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["delete", "stream_stats"])
            .inc();
        sqlx::query(&sql).execute(&pool).await?;
        Ok(())
    }

    async fn set_stream_stats(
        &self,
        org_id: &str,
        streams: &[(String, StreamStats)],
    ) -> Result<()> {
        let pool = CLIENT.clone();
        let old_stats = self.get_stream_stats(org_id, None, None).await?;
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
            stats.format_by(item); // format stats
            update_streams.push((stream_key, stats));
        }

        let mut tx = pool.begin().await?;
        for stream_key in new_streams {
            let org_id = stream_key[..stream_key.find('/').unwrap()].to_string();
            DB_QUERY_NUMS
                .with_label_values(&["insert", "stream_stats"])
                .inc();
            if let Err(e) = sqlx::query(
                r#"
INSERT INTO stream_stats 
    (org, stream, file_num, min_ts, max_ts, records, original_size, compressed_size, index_size)
    VALUES (?, ?, 0, 0, 0, 0, 0, 0, 0);
                "#,
            )
            .bind(org_id)
            .bind(stream_key)
            .execute(&mut *tx)
            .await
            {
                if let Err(e) = tx.rollback().await {
                    log::error!("[MYSQL] rollback insert stream stats error: {}", e);
                }
                return Err(e.into());
            }
        }
        if let Err(e) = tx.commit().await {
            log::error!("[MYSQL] commit set stream stats error: {}", e);
            return Err(e.into());
        }

        let mut tx = pool.begin().await?;
        for (stream_key, stats) in update_streams {
            DB_QUERY_NUMS
                .with_label_values(&["update", "stream_stats"])
                .inc();
            if let Err(e) = sqlx::query(
                r#"
UPDATE stream_stats 
    SET file_num = file_num + ?, min_ts = ?, max_ts = ?, records = records + ?, original_size = original_size + ?, compressed_size = compressed_size + ?, index_size = index_size + ?
    WHERE stream = ?;
                "#,
            )
            .bind(stats.file_num)
            .bind(stats.doc_time_min)
            .bind(stats.doc_time_max)
            .bind(stats.doc_num)
            .bind(stats.storage_size)
            .bind(stats.compressed_size)
            .bind(stats.index_size)
            .bind(stream_key)
            .execute(&mut *tx)
            .await
            {
                if let Err(e) = tx.rollback().await {
                    log::error!("[MYSQL] rollback set stream stats error: {}", e);
                }
                return Err(e.into());
            }
        }
        if let Err(e) = tx.commit().await {
            log::error!("[MYSQL] commit set stream stats error: {}", e);
            return Err(e.into());
        }

        Ok(())
    }

    async fn reset_stream_stats(&self) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "stream_stats"])
            .inc();
        sqlx::query(r#"UPDATE stream_stats SET file_num = 0, min_ts = 0, max_ts = 0, records = 0, original_size = 0, compressed_size = 0, index_size = 0;"#)
             .execute(&pool)
            .await?;
        Ok(())
    }

    async fn reset_stream_stats_min_ts(
        &self,
        _org_id: &str,
        stream: &str,
        min_ts: i64,
    ) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "stream_stats"])
            .inc();
        sqlx::query(r#"UPDATE stream_stats SET min_ts = ? WHERE stream = ?;"#)
            .bind(min_ts)
            .bind(stream)
            .execute(&pool)
            .await?;
        DB_QUERY_NUMS
            .with_label_values(&["update", "stream_stats"])
            .inc();
        sqlx::query(
            r#"UPDATE stream_stats SET max_ts = min_ts WHERE stream = ? AND max_ts < min_ts;"#,
        )
        .bind(stream)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn len(&self) -> usize {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let ret = match sqlx::query(r#"SELECT CAST(COUNT(*) AS SIGNED) AS num FROM file_list;"#)
            .fetch_one(&pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[MYSQL] get file list len error: {}", e);
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
    ) -> Result<i64> {
        let stream_key = format!("{org_id}/{stream_type}/{stream}");
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["insert", "file_list_jobs"])
            .inc();
        match sqlx::query(
            "INSERT IGNORE INTO file_list_jobs (org, stream, offsets, status, node, started_at, updated_at) VALUES (?, ?, ?, ?, '', 0, 0);",
        )
        .bind(org_id)
        .bind(&stream_key)
        .bind(offset)
        .bind(super::FileListJobStatus::Pending)
        .execute(&pool)
        .await
        {
            Err(sqlx::Error::Database(e)) => if !e.is_unique_violation() {
                return Err(Error::Message(e.to_string()));
            }
            Err(e) => {
                return Err(e.into());
            },
            Ok(_) => {}
        };

        // get job id
        let ret = sqlx::query(
            "SELECT id FROM file_list_jobs WHERE org = ? AND stream = ? AND offsets = ?;",
        )
        .bind(org_id)
        .bind(&stream_key)
        .bind(offset)
        .fetch_one(&pool)
        .await?;
        Ok(ret.try_get::<i64, &str>("id").unwrap_or_default())
    }

    async fn get_pending_jobs(&self, node: &str, limit: i64) -> Result<Vec<super::MergeJobRecord>> {
        let lock_pool = CLIENT.clone();
        let lock_key = "file_list_jobs:get_pending_jobs";
        let lock_id = config::utils::hash::gxhash::new().sum64(lock_key);
        let lock_sql = format!(
            "SELECT GET_LOCK('{}', {})",
            lock_id,
            config::get_config().limit.meta_transaction_lock_timeout
        );
        let unlock_sql = format!("SELECT RELEASE_LOCK('{}')", lock_id);
        let mut lock_tx = lock_pool.begin().await?;
        DB_QUERY_NUMS.with_label_values(&["get_lock", ""]).inc();
        match sqlx::query_scalar::<_, i64>(&lock_sql)
            .fetch_one(&mut *lock_tx)
            .await
        {
            Ok(v) => {
                if v != 1 {
                    if let Err(e) = lock_tx.rollback().await {
                        log::error!("[MYSQL] rollback lock for get_pending_jobs error: {}", e);
                    }
                    return Err(Error::from(DbError::DBOperError(
                        "LockTimeout".to_string(),
                        lock_key.to_string(),
                    )));
                }
            }
            Err(e) => {
                if let Err(e) = lock_tx.rollback().await {
                    log::error!("[MYSQL] rollback lock for get_pending_jobs error: {}", e);
                }
                return Err(e.into());
            }
        };

        let pool = CLIENT.clone();
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                if let Err(e) = sqlx::query(&unlock_sql).execute(&mut *lock_tx).await {
                    log::error!("[MYSQL] unlock get_pending_jobs error: {}", e);
                }
                if let Err(e) = lock_tx.commit().await {
                    log::error!("[MYSQL] commit for unlock get_pending_jobs error: {}", e);
                }
                return Err(e.into());
            }
        };
        // get pending jobs group by stream and order by num desc
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list_jobs"])
            .inc();
        let ret = match sqlx::query_as::<_, super::MergeJobPendingRecord>(
            r#"
SELECT stream, max(id) as id, CAST(COUNT(*) AS SIGNED) AS num
    FROM file_list_jobs 
    WHERE status = ? 
    GROUP BY stream 
    ORDER BY num DESC 
    LIMIT ?;"#,
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
                        "[MYSQL] rollback select file_list_jobs pending jobs for update error: {e}"
                    );
                }
                DB_QUERY_NUMS.with_label_values(&["release_lock", ""]).inc();
                if let Err(e) = sqlx::query(&unlock_sql).execute(&mut *lock_tx).await {
                    log::error!("[MYSQL] unlock get_pending_jobs error: {}", e);
                }
                if let Err(e) = lock_tx.commit().await {
                    log::error!("[MYSQL] commit for unlock get_pending_jobs error: {}", e);
                }
                return Err(e.into());
            }
        };
        // update jobs status to running
        let ids = ret.iter().map(|r| r.id.to_string()).collect::<Vec<_>>();
        if ids.is_empty() {
            if let Err(e) = tx.rollback().await {
                log::error!("[MYSQL] rollback select file_list_jobs pending jobs error: {e}");
            }
            DB_QUERY_NUMS.with_label_values(&["release_lock", ""]).inc();
            if let Err(e) = sqlx::query(&unlock_sql).execute(&mut *lock_tx).await {
                log::error!("[MYSQL] unlock get_pending_jobs error: {}", e);
            }
            if let Err(e) = lock_tx.commit().await {
                log::error!("[MYSQL] commit for unlock get_pending_jobs error: {}", e);
            }
            return Ok(Vec::new());
        }
        let sql = format!(
            "UPDATE file_list_jobs SET status = ?, node = ?, started_at = ?, updated_at = ? WHERE id IN ({});",
            ids.join(",")
        );
        let now = config::utils::time::now_micros();
        DB_QUERY_NUMS
            .with_label_values(&["update", "file_list_jobs"])
            .inc();
        if let Err(e) = sqlx::query(&sql)
            .bind(super::FileListJobStatus::Running)
            .bind(node)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
        {
            if let Err(e) = tx.rollback().await {
                log::error!("[MYSQL] rollback update file_list_jobs status error: {e}");
            }
            DB_QUERY_NUMS.with_label_values(&["release_lock", ""]).inc();
            if let Err(e) = sqlx::query(&unlock_sql).execute(&mut *lock_tx).await {
                log::error!("[MYSQL] unlock get_pending_jobs error: {}", e);
            }
            if let Err(e) = lock_tx.commit().await {
                log::error!("[MYSQL] commit for unlock get_pending_jobs error: {}", e);
            }
            return Err(e.into());
        }
        // get jobs by ids
        let sql = format!(
            "SELECT * FROM file_list_jobs WHERE id IN ({});",
            ids.join(",")
        );
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list_jobs"])
            .inc();
        let ret = match sqlx::query_as::<_, super::MergeJobRecord>(&sql)
            .fetch_all(&mut *tx)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    log::error!("[MYSQL] rollback select file_list_jobs by ids error: {e}");
                }
                DB_QUERY_NUMS.with_label_values(&["release_lock", ""]).inc();
                if let Err(e) = sqlx::query(&unlock_sql).execute(&mut *lock_tx).await {
                    log::error!("[MYSQL] unlock get_pending_jobs error: {}", e);
                }
                if let Err(e) = lock_tx.commit().await {
                    log::error!("[MYSQL] commit for unlock get_pending_jobs error: {}", e);
                }
                return Err(e.into());
            }
        };
        if let Err(e) = tx.commit().await {
            log::error!("[MYSQL] commit select file_list_jobs pending jobs error: {e}");
            return Err(e.into());
        }
        DB_QUERY_NUMS.with_label_values(&["release_lock", ""]).inc();
        if let Err(e) = sqlx::query(&unlock_sql).execute(&mut *lock_tx).await {
            log::error!("[MYSQL] unlock get_pending_jobs error: {}", e);
        }
        if let Err(e) = lock_tx.commit().await {
            log::error!("[MYSQL] commit for unlock get_pending_jobs error: {}", e);
        }
        Ok(ret)
    }

    async fn set_job_pending(&self, ids: &[i64]) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "file_list_jobs"])
            .inc();
        let sql = format!(
            "UPDATE file_list_jobs SET status = ? WHERE id IN ({});",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        sqlx::query(&sql)
            .bind(super::FileListJobStatus::Pending)
            .execute(&pool)
            .await?;
        Ok(())
    }

    async fn set_job_done(&self, id: i64) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "file_list_jobs"])
            .inc();
        sqlx::query(r#"UPDATE file_list_jobs SET status = ?, updated_at = ? WHERE id = ?;"#)
            .bind(super::FileListJobStatus::Done)
            .bind(config::utils::time::now_micros())
            .bind(id)
            .execute(&pool)
            .await?;
        Ok(())
    }

    async fn update_running_jobs(&self, id: i64) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "file_list_jobs"])
            .inc();
        sqlx::query(r#"UPDATE file_list_jobs SET updated_at = ? WHERE id = ?;"#)
            .bind(config::utils::time::now_micros())
            .bind(id)
            .execute(&pool)
            .await?;
        Ok(())
    }

    async fn check_running_jobs(&self, before_date: i64) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["update", "file_list_jobs"])
            .inc();
        let ret = sqlx::query(
            r#"UPDATE file_list_jobs SET status = ? WHERE status = ? AND updated_at < ?;"#,
        )
        .bind(super::FileListJobStatus::Pending)
        .bind(super::FileListJobStatus::Running)
        .bind(before_date)
        .execute(&pool)
        .await?;
        if ret.rows_affected() > 0 {
            log::warn!("[MYSQL] reset running jobs status to pending");
        }
        Ok(())
    }

    async fn clean_done_jobs(&self, before_date: i64) -> Result<()> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS
            .with_label_values(&["delete", "file_list_jobs"])
            .inc();
        let ret = sqlx::query(r#"DELETE FROM file_list_jobs WHERE status = ? AND updated_at < ?;"#)
            .bind(super::FileListJobStatus::Done)
            .bind(before_date)
            .execute(&pool)
            .await?;
        if ret.rows_affected() > 0 {
            log::warn!("[MYSQL] clean done jobs");
        }
        Ok(())
    }

    async fn get_pending_jobs_count(&self) -> Result<stdHashMap<String, stdHashMap<String, i64>>> {
        let pool = CLIENT.clone();

        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list_jobs"])
            .inc();
        let ret =
            sqlx::query(r#"SELECT stream, status, count(*) as counts FROM file_list_jobs WHERE status = ? GROUP BY stream, status ORDER BY status desc;"#)
                .bind(super::FileListJobStatus::Pending)
                .fetch_all(&pool)
                .await?;

        let mut job_status: stdHashMap<String, stdHashMap<String, i64>> = stdHashMap::new();

        for r in ret.iter() {
            let stream = r.get::<String, &str>("stream");
            let status = r.get::<i32, &str>("status");
            let counts = if status == 0 {
                r.get::<i64, &str>("counts")
            } else {
                0
            };
            let parts: Vec<&str> = stream.split('/').collect();
            if parts.len() >= 2 {
                let org = parts[0].to_string();
                let stream_type = parts[1].to_string();
                job_status
                    .entry(org)
                    .or_default()
                    .entry(stream_type)
                    .and_modify(|e| *e = counts)
                    .or_insert(counts);
            }
        }
        Ok(job_status)
    }
}

impl MysqlFileList {
    async fn inner_add(&self, table: &str, file: &str, meta: &FileMeta) -> Result<()> {
        let pool = CLIENT.clone();
        let (stream_key, date_key, file_name) =
            parse_file_key_columns(file).map_err(|e| Error::Message(e.to_string()))?;
        let org_id = stream_key[..stream_key.find('/').unwrap()].to_string();
        DB_QUERY_NUMS.with_label_values(&["insert", table]).inc();
        match  sqlx::query(
            format!(r#"
INSERT IGNORE INTO {table} (org, stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, index_size, flattened)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
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
        .bind(meta.index_size)
        .bind(meta.flattened)
        .execute(&pool)
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
        let pool = CLIENT.clone();
        let chunks = files.chunks(100);
        for files in chunks {
            let mut tx = pool.begin().await?;
            let mut query_builder: QueryBuilder<MySql> = QueryBuilder::new(
                format!("INSERT INTO {table} (org, stream, date, file, deleted, min_ts, max_ts, records, original_size, compressed_size, index_size, flattened)").as_str(),
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
                    .push_bind(item.meta.index_size)
                    .push_bind(item.meta.flattened);
            });
            DB_QUERY_NUMS.with_label_values(&["insert", table]).inc();
            let need_single_insert = match query_builder.build().execute(&mut *tx).await {
                Ok(_) => false,
                Err(sqlx::Error::Database(e)) => {
                    if e.is_unique_violation() {
                        true
                    } else {
                        if let Err(e) = tx.rollback().await {
                            log::error!("[MYSQL] rollback {table} batch add error: {}", e);
                        }
                        return Err(Error::Message(e.to_string()));
                    }
                }
                Err(e) => {
                    if let Err(e) = tx.rollback().await {
                        log::error!("[MYSQL] rollback {table} batch add error: {}", e);
                    }
                    return Err(e.into());
                }
            };
            if need_single_insert {
                if let Err(e) = tx.rollback().await {
                    log::error!("[MYSQL] rollback {table} batch add error: {}", e);
                    return Err(e.into());
                }
                for item in files {
                    if let Err(e) = self.inner_add(table, &item.key, &item.meta).await {
                        log::error!("[MYSQL] single insert {table} add error: {}", e);
                        return Err(e);
                    }
                }
            } else if let Err(e) = tx.commit().await {
                log::error!("[MYSQL] commit {table} batch add error: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }
}

pub async fn create_table() -> Result<()> {
    let pool = CLIENT.clone();
    DB_QUERY_NUMS
        .with_label_values(&["create", "file_list"])
        .inc();
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list
(
    id        BIGINT not null primary key AUTO_INCREMENT,
    org       VARCHAR(100) not null,
    stream    VARCHAR(256) not null,
    date      VARCHAR(16)  not null,
    file      VARCHAR(496) not null,
    deleted   BOOLEAN default false not null,
    flattened BOOLEAN default false not null,
    min_ts    BIGINT not null,
    max_ts    BIGINT not null,
    records   BIGINT not null,
    original_size   BIGINT not null,
    compressed_size BIGINT not null,
    index_size      BIGINT not null
);
        "#,
    )
    .execute(&pool)
    .await?;

    DB_QUERY_NUMS
        .with_label_values(&["create", "file_list_history"])
        .inc();
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list_history
(
    id        BIGINT not null primary key AUTO_INCREMENT,
    org       VARCHAR(100) not null,
    stream    VARCHAR(256) not null,
    date      VARCHAR(16)  not null,
    file      VARCHAR(496) not null,
    deleted   BOOLEAN default false not null,
    flattened BOOLEAN default false not null,
    min_ts    BIGINT not null,
    max_ts    BIGINT not null,
    records   BIGINT not null,
    original_size   BIGINT not null,
    compressed_size BIGINT not null,
    index_size      BIGINT not null
);
        "#,
    )
    .execute(&pool)
    .await?;

    DB_QUERY_NUMS
        .with_label_values(&["create", "file_list_deleted"])
        .inc();
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list_deleted
(
    id         BIGINT not null primary key AUTO_INCREMENT,
    org        VARCHAR(100) not null,
    stream     VARCHAR(256) not null,
    date       VARCHAR(16)  not null,
    file       VARCHAR(496) not null,
    index_file BOOLEAN default false not null,
    flattened  BOOLEAN default false not null,
    created_at BIGINT not null
);
        "#,
    )
    .execute(&pool)
    .await?;

    DB_QUERY_NUMS
        .with_label_values(&["create", "file_list_jobs"])
        .inc();
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS file_list_jobs
(
    id         BIGINT not null primary key AUTO_INCREMENT,
    org        VARCHAR(100) not null,
    stream     VARCHAR(256) not null,
    offsets    BIGINT not null,
    status     INT not null,
    node       VARCHAR(100) not null,
    started_at BIGINT not null,
    updated_at BIGINT not null
);
        "#,
    )
    .execute(&pool)
    .await?;

    DB_QUERY_NUMS
        .with_label_values(&["create", "stream_stats"])
        .inc();
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS stream_stats
(
    id       BIGINT not null primary key AUTO_INCREMENT,
    org      VARCHAR(100) not null,
    stream   VARCHAR(256) not null,
    file_num BIGINT not null,
    min_ts   BIGINT not null,
    max_ts   BIGINT not null,
    records  BIGINT not null,
    original_size   BIGINT not null,
    compressed_size BIGINT not null,
    index_size      BIGINT not null
);
        "#,
    )
    .execute(&pool)
    .await?;

    // create column flattened for old version <= 0.10.5
    let column = "flattened";
    let data_type = "BOOLEAN default false not null";
    add_column("file_list", column, data_type).await?;
    add_column("file_list_history", column, data_type).await?;
    add_column("file_list_deleted", column, data_type).await?;

    // create column started_at for old version <= 0.10.8
    let column = "started_at";
    let data_type = "BIGINT default 0 not null";
    add_column("file_list_jobs", column, data_type).await?;

    // create column index_size for old version <= 0.13.1
    let column = "index_size";
    let data_type = "BIGINT default 0 not null";
    add_column("file_list", column, data_type).await?;
    add_column("file_list_history", column, data_type).await?;
    add_column("stream_stats", column, data_type).await?;
    let column = "index_file";
    let data_type = "BOOLEAN default false not null";
    add_column("file_list_deleted", column, data_type).await?;

    Ok(())
}

pub async fn create_table_index() -> Result<()> {
    let pool = CLIENT.clone();

    let indices: Vec<(&str, &str, &[&str])> = vec![
        ("file_list_org_idx", "file_list", &["org"]),
        (
            "file_list_stream_ts_idx",
            "file_list",
            &["stream", "max_ts", "min_ts"],
        ),
        ("file_list_history_org_idx", "file_list_history", &["org"]),
        (
            "file_list_history_stream_ts_idx",
            "file_list_history",
            &["stream", "max_ts", "min_ts"],
        ),
        (
            "file_list_deleted_created_at_idx",
            "file_list_deleted",
            &["org", "created_at"],
        ),
        (
            "file_list_deleted_stream_date_file_idx",
            "file_list_deleted",
            &["stream", "date", "file"],
        ),
        (
            "file_list_jobs_stream_status_idx",
            "file_list_jobs",
            &["status", "stream"],
        ),
        ("stream_stats_org_idx", "stream_stats", &["org"]),
    ];
    for (idx, table, fields) in indices {
        create_index(IndexStatement::new(idx, table, false, fields)).await?;
    }

    let unique_indices: Vec<(&str, &str, &[&str])> = vec![
        (
            "file_list_history_stream_file_idx",
            "file_list_history",
            &["stream", "date", "file"],
        ),
        (
            "file_list_jobs_stream_offsets_idx",
            "file_list_jobs",
            &["stream", "offsets"],
        ),
        ("stream_stats_stream_idx", "stream_stats", &["stream"]),
    ];
    for (idx, table, fields) in unique_indices {
        create_index(IndexStatement::new(idx, table, true, fields)).await?;
    }

    // This is special case where we want to MAKE the index unique if it is not
    let res = create_index(IndexStatement::new(
        "file_list_stream_file_idx",
        "file_list",
        true,
        &["stream", "date", "file"],
    ))
    .await;
    if let Err(e) = res {
        if !e.to_string().contains("Duplicate entry") {
            return Err(e);
        }

        log::warn!("[MYSQL] starting delete duplicate records");
        // delete duplicate records
        DB_QUERY_NUMS
            .with_label_values(&["select", "file_list"])
            .inc();
        let ret = sqlx::query(
                r#"SELECT stream, date, file, min(id) as id FROM file_list GROUP BY stream, date, file HAVING COUNT(*) > 1;"#,
            ).fetch_all(&pool).await?;
        log::warn!("[MYSQL] total: {} duplicate records", ret.len());
        for (i, r) in ret.iter().enumerate() {
            let stream = r.get::<String, &str>("stream");
            let date = r.get::<String, &str>("date");
            let file = r.get::<String, &str>("file");
            let id = r.get::<i64, &str>("id");
            DB_QUERY_NUMS
                .with_label_values(&["delete", "file_list"])
                .inc();
            sqlx::query(
                r#"DELETE FROM file_list WHERE id != ? AND stream = ? AND date = ? AND file = ?;"#,
            )
            .bind(id)
            .bind(stream)
            .bind(date)
            .bind(file)
            .execute(&pool)
            .await?;
            if i % 1000 == 0 {
                log::warn!("[MYSQL] delete duplicate records: {}/{}", i, ret.len());
            }
        }
        log::warn!(
            "[MYSQL] delete duplicate records: {}/{}",
            ret.len(),
            ret.len()
        );
        // create index again
        create_index(IndexStatement::new(
            "file_list_stream_file_idx",
            "file_list",
            true,
            &["stream", "date", "file"],
        ))
        .await?;
        log::warn!("[MYSQL] create table index(file_list_stream_file_idx) successfully");
    }

    Ok(())
}

async fn add_column(table: &str, column: &str, data_type: &str) -> Result<()> {
    let pool = CLIENT.clone();
    DB_QUERY_NUMS
        .with_label_values(&["select", "information_schema.columns"])
        .inc();
    let check_sql = format!(
        "SELECT count(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name='{table}' AND column_name='{column}';"
    );
    let has_column = sqlx::query_scalar::<_, i64>(&check_sql)
        .fetch_one(&pool)
        .await?;
    if has_column > 0 {
        return Ok(());
    }

    let alert_sql = format!("ALTER TABLE {table} ADD COLUMN {column} {data_type};");
    let mut tx = pool.begin().await?;
    DB_QUERY_NUMS.with_label_values(&["alter", table]).inc();
    if let Err(e) = sqlx::query(&alert_sql).execute(&mut *tx).await {
        if !e.to_string().contains("Duplicate column name") {
            // Check for the specific MySQL error code for duplicate column
            log::error!("[MYSQL] Unexpected error in adding column {column}: {}", e);
            if let Err(e) = tx.rollback().await {
                log::error!("[MYSQL] Error in rolling back transaction: {}", e);
            }
            return Err(e.into());
        }
    }
    if let Err(e) = tx.commit().await {
        log::info!("[MYSQL] Error in committing transaction: {}", e);
        return Err(e.into());
    };
    Ok(())
}
