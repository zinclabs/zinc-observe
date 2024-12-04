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

use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use config::{
    metrics::{DB_QUERY_NUMS, DB_QUERY_TIME},
    utils::{hash::Sum64, util::zero_or},
};
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, Pool, Postgres,
};
use tokio::sync::{mpsc, OnceCell};

use super::{DBIndex, IndexStatement};
use crate::errors::*;

pub static CLIENT: Lazy<Pool<Postgres>> = Lazy::new(connect);
static INDICES: OnceCell<HashSet<DBIndex>> = OnceCell::const_new();

fn connect() -> Pool<Postgres> {
    let cfg = config::get_config();
    let db_opts = PgConnectOptions::from_str(&cfg.common.meta_postgres_dsn)
        .expect("postgres connect options create failed")
        .disable_statement_logging();

    let acquire_timeout = zero_or(cfg.limit.sql_db_connections_idle_timeout, 30);
    let idle_timeout = zero_or(cfg.limit.sql_db_connections_idle_timeout, 600);
    let max_lifetime = zero_or(cfg.limit.sql_db_connections_max_lifetime, 1800);

    PgPoolOptions::new()
        .min_connections(cfg.limit.sql_db_connections_min)
        .max_connections(cfg.limit.sql_db_connections_max)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .idle_timeout(Some(Duration::from_secs(idle_timeout)))
        .max_lifetime(Some(Duration::from_secs(max_lifetime)))
        .connect_lazy_with(db_opts)
}

async fn cache_indices() -> HashSet<DBIndex> {
    let client = CLIENT.clone();
    DB_QUERY_NUMS
        .with_label_values(&["select", "pg_indexes"])
        .inc();
    let sql = r#"SELECT indexname, tablename FROM pg_indexes;"#;
    let res = sqlx::query_as::<_, (String, String)>(sql)
        .fetch_all(&client)
        .await;
    match res {
        Ok(r) => r
            .into_iter()
            .map(|(name, table)| DBIndex { name, table })
            .collect(),
        Err(_) => HashSet::new(),
    }
}
pub struct PostgresDb {}

impl PostgresDb {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PostgresDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::Db for PostgresDb {
    async fn create_table(&self) -> Result<()> {
        create_table().await
    }

    async fn stats(&self) -> Result<super::Stats> {
        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
        let keys_count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*)::BIGINT AS num FROM meta;"#)
            .fetch_one(&pool)
            .await
            .unwrap_or_default();
        Ok(super::Stats {
            bytes_len: 0,
            keys_count,
        })
    }

    async fn get(&self, key: &str) -> Result<Bytes> {
        let (module, key1, key2) = super::parse_key(key);
        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
        let query = format!(
            "SELECT value FROM meta WHERE module = '{}' AND key1 = '{}' AND key2 = '{}' ORDER BY start_dt DESC;",
            module, key1, key2
        );
        let value: String = match sqlx::query_scalar(&query).fetch_one(&pool).await {
            Ok(v) => v,
            Err(e) => {
                if let sqlx::Error::RowNotFound = e {
                    return Err(Error::from(DbError::KeyNotExists(key.to_string())));
                } else {
                    return Err(Error::from(DbError::DBOperError(
                        e.to_string(),
                        key.to_string(),
                    )));
                }
            }
        };
        Ok(Bytes::from(value))
    }

    async fn put(
        &self,
        key: &str,
        value: Bytes,
        need_watch: bool,
        start_dt: Option<i64>,
    ) -> Result<()> {
        let (module, key1, key2) = super::parse_key(key);
        let pool = CLIENT.clone();
        let local_start_dt = start_dt.unwrap_or_default();
        let mut tx = pool.begin().await?;
        DB_QUERY_NUMS.with_label_values(&["insert", "meta"]).inc();
        let start = std::time::Instant::now();
        if let Err(e) = sqlx::query(
            r#"INSERT INTO meta (module, key1, key2, start_dt, value) VALUES ($1, $2, $3, $4, '') ON CONFLICT DO NOTHING;"#
        )
        .bind(&module)
        .bind(&key1)
        .bind(&key2)
        .bind(local_start_dt)
        .execute(&mut *tx)
        .await
        {
            if let Err(e) = tx.rollback().await {
                log::error!("[POSTGRES] rollback put meta error: {}", e);
            }
            return Err(e.into());
        }
        // need commit it first to avoid the deadlock of insert and update
        if let Err(e) = tx.commit().await {
            log::error!("[POSTGRES] commit put meta error: {}", e);
            return Err(e.into());
        }

        DB_QUERY_NUMS.with_label_values(&["update", "meta"]).inc();
        let mut tx = pool.begin().await?;
        if let Err(e) = sqlx::query(
                r#"UPDATE meta SET value = $1 WHERE module = $2 AND key1 = $3 AND key2 = $4 AND start_dt = $5;"#
            )
            .bind(String::from_utf8(value.to_vec()).unwrap_or_default())
            .bind(&module)
            .bind(&key1)
            .bind(&key2)
            .bind(local_start_dt)
            .execute(&mut *tx)
            .await
            {
                if let Err(e) = tx.rollback().await {
                    log::error!("[POSTGRES] rollback put meta error: {}", e);
                }
                return Err(e.into());
            }
        if let Err(e) = tx.commit().await {
            log::error!("[POSTGRES] commit put meta error: {}", e);
            return Err(e.into());
        }
        let time = start.elapsed().as_secs_f64();

        // event watch
        if need_watch {
            let cluster_coordinator = super::get_coordinator().await;
            cluster_coordinator
                .put(key, Bytes::from(""), true, start_dt)
                .await?;
        }
        DB_QUERY_TIME
            .with_label_values(&["put_item", "meta"])
            .observe(time);

        Ok(())
    }

    async fn get_for_update(
        &self,
        key: &str,
        need_watch: bool,
        start_dt: Option<i64>,
        update_fn: Box<super::UpdateFn>,
    ) -> Result<()> {
        let (module, key1, key2) = super::parse_key(key);
        let pool = CLIENT.clone();
        let mut tx = pool.begin().await?;
        let lock_key = format!("get_for_update_{}", key);
        let lock_id = config::utils::hash::gxhash::new().sum64(&lock_key);
        let lock_id = if lock_id > i64::MAX as u64 {
            (lock_id >> 1) as i64
        } else {
            lock_id as i64
        };
        let lock_sql = format!("SELECT pg_advisory_xact_lock({lock_id})");
        DB_QUERY_NUMS.with_label_values(&["get_lock", ""]).inc();
        if let Err(e) = sqlx::query(&lock_sql).execute(&mut *tx).await {
            if let Err(e) = tx.rollback().await {
                log::error!("[POSTGRES] rollback get_for_update error: {}", e);
            }
            return Err(e.into());
        };
        let mut need_watch_dt = 0;
        let row = if let Some(start_dt) = start_dt {
            DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
            match sqlx::query_as::<_,super::MetaRecord>(
                r#"SELECT id, module, key1, key2, start_dt, value FROM meta WHERE module = $1 AND key1 = $2 AND key2 = $3 AND start_dt = $4;"#
            )
            .bind(&module)
            .bind(&key1)
            .bind(&key2)
            .bind(start_dt)
            .fetch_one(&mut *tx)
            .await
            {
                Ok(v) => Some(v),
                Err(e) => {
                    if e.to_string().contains("no rows returned") {
                        None
                    } else {
                        if let Err(e) = tx.rollback().await {
                            log::error!("[POSTGRES] rollback get_for_update error: {}", e);
                        }
                        return Err(e.into());
                    }
                }
            }
        } else {
            DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
            match sqlx::query_as::<_,super::MetaRecord>(
                r#"SELECT id, module, key1, key2, start_dt, value FROM meta WHERE module = $1 AND key1 = $2 AND key2 = $3 ORDER BY start_dt DESC, id DESC;"#
            )
            .bind(&module)
            .bind(&key1)
            .bind(&key2)
            .fetch_one(&mut *tx)
            .await
            {
                Ok(v) => Some(v),
                Err(e) => {
                    if e.to_string().contains("no rows returned") {
                        None
                    } else {
                        if let Err(e) = tx.rollback().await {
                            log::error!("[POSTGRES] rollback get_for_update error: {}", e);
                        }
                        return Err(e.into());
                    }
                }
            }
        };
        let exist = row.is_some();
        let row_id = row.as_ref().map(|r| r.id);
        let value = row.map(|r| Bytes::from(r.value));
        let (value, new_value) = match update_fn(value) {
            Err(e) => {
                if let Err(e) = tx.rollback().await {
                    log::error!("[POSTGRES] rollback get_for_update error: {}", e);
                }
                return Err(e);
            }
            Ok(None) => {
                if let Err(e) = tx.rollback().await {
                    log::error!("[POSTGRES] rollback get_for_update error: {}", e);
                }
                return Ok(());
            }
            Ok(Some(v)) => v,
        };

        // update value
        if let Some(value) = value {
            let ret = if exist {
                DB_QUERY_NUMS.with_label_values(&["update", "meta"]).inc();
                sqlx::query(r#"UPDATE meta SET value = $1 WHERE id = $2;"#)
                    .bind(String::from_utf8(value.to_vec()).unwrap_or_default())
                    .bind(row_id.unwrap())
                    .execute(&mut *tx)
                    .await
            } else {
                DB_QUERY_NUMS.with_label_values(&["insert", "meta"]).inc();
                sqlx::query(
                r#"INSERT INTO meta (module, key1, key2, start_dt, value) VALUES ($1, $2, $3, $4, $5);"#
            )
            .bind(&module)
            .bind(&key1)
            .bind(&key2)
            .bind(start_dt.unwrap_or_default())
            .bind(String::from_utf8(value.to_vec()).unwrap_or_default())
            .execute(&mut *tx)
            .await
            };
            if let Err(e) = ret {
                if let Err(e) = tx.rollback().await {
                    log::error!("[POSTGRES] rollback get_for_update error: {}", e);
                }
                return Err(e.into());
            }
        }

        // new value
        if let Some((new_key, new_value, new_start_dt)) = new_value {
            need_watch_dt = new_start_dt.unwrap_or_default();
            let (module, key1, key2) = super::parse_key(&new_key);
            DB_QUERY_NUMS.with_label_values(&["insert", "meta"]).inc();
            if let Err(e) = sqlx::query(
                r#"INSERT INTO meta (module, key1, key2, start_dt, value) VALUES ($1, $2, $3, $4, $5);"#
            )
            .bind(&module)
            .bind(&key1)
            .bind(&key2)
            .bind(new_start_dt.unwrap_or_default())
            .bind(String::from_utf8(new_value.to_vec()).unwrap_or_default())
            .execute(&mut *tx)
            .await
            {
                if let Err(e) = tx.rollback().await {
                    log::error!("[POSTGRES] rollback get_for_update error: {}", e);
                }
                return Err(e.into());
            }
        }

        if let Err(e) = tx.commit().await {
            log::error!("[POSTGRES] commit get_for_update error: {}", e);
            return Err(e.into());
        }

        // event watch
        if need_watch {
            let start_dt = if need_watch_dt > 0 {
                Some(need_watch_dt)
            } else {
                start_dt
            };
            let cluster_coordinator = super::get_coordinator().await;
            cluster_coordinator
                .put(key, Bytes::from(""), true, start_dt)
                .await?;
        }

        Ok(())
    }

    async fn delete(
        &self,
        key: &str,
        with_prefix: bool,
        need_watch: bool,
        start_dt: Option<i64>,
    ) -> Result<()> {
        // event watch
        if need_watch {
            // find all keys then send event
            let items = if with_prefix {
                self.list_keys(key).await?
            } else {
                vec![key.to_string()]
            };
            let cluster_coordinator = super::get_coordinator().await;
            tokio::task::spawn(async move {
                for key in items {
                    if let Err(e) = cluster_coordinator
                        .delete(&key, false, true, start_dt)
                        .await
                    {
                        log::error!("[POSTGRES] send event error: {}", e);
                    }
                }
            });
        }

        let (module, key1, key2) = super::parse_key(key);
        // Escape ' (single quote) character with ''
        let (key1, key2) = (key1.replace("'", "''"), key2.replace("'", "''"));
        let sql = if with_prefix {
            if key1.is_empty() {
                format!(r#"DELETE FROM meta WHERE module = '{}';"#, module)
            } else if key2.is_empty() {
                format!(
                    r#"DELETE FROM meta WHERE module = '{}' AND key1 = '{}';"#,
                    module, key1
                )
            } else {
                format!(
                    r#"DELETE FROM meta WHERE module = '{}' AND key1 = '{}' AND (key2 = '{}' OR key2 LIKE '{}/%');"#,
                    module, key1, key2, key2
                )
            }
        } else {
            format!(
                r#"DELETE FROM meta WHERE module = '{}' AND key1 = '{}' AND key2 = '{}';"#,
                module, key1, key2
            )
        };

        let sql = if let Some(start_dt) = start_dt {
            sql.replace(';', &format!(" AND start_dt = {};", start_dt))
        } else {
            sql
        };

        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["delete", "meta"]).inc();
        sqlx::query(&sql).execute(&pool).await?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<HashMap<String, Bytes>> {
        let (module, key1, key2) = super::parse_key(prefix);
        let mut sql = "SELECT id, module, key1, key2, start_dt, value FROM meta".to_string();
        if !module.is_empty() {
            sql = format!("{} WHERE module = '{}'", sql, module);
        }
        if !key1.is_empty() {
            sql = format!("{} AND key1 = '{}'", sql, key1);
        }
        if !key2.is_empty() {
            sql = format!("{} AND (key2 = '{}' OR key2 LIKE '{}/%')", sql, key2, key2);
        }
        sql = format!("{} ORDER BY start_dt ASC", sql);

        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
        let ret = sqlx::query_as::<_, super::MetaRecord>(&sql)
            .fetch_all(&pool)
            .await?;
        Ok(ret
            .into_iter()
            .map(|r| {
                (
                    super::build_key(&r.module, &r.key1, &r.key2, r.start_dt),
                    Bytes::from(r.value),
                )
            })
            .collect())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let (module, key1, key2) = super::parse_key(prefix);
        let mut sql = "SELECT id, module, key1, key2, start_dt, '' AS value FROM meta ".to_string();
        if !module.is_empty() {
            sql = format!("{} WHERE module = '{}'", sql, module);
        }
        if !key1.is_empty() {
            sql = format!("{} AND key1 = '{}'", sql, key1);
        }
        if !key2.is_empty() {
            sql = format!("{} AND (key2 = '{}' OR key2 LIKE '{}/%')", sql, key2, key2);
        }
        sql = format!("{} ORDER BY start_dt ASC", sql);
        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
        let ret = sqlx::query_as::<_, super::MetaRecord>(&sql)
            .fetch_all(&pool)
            .await?;
        Ok(ret
            .into_iter()
            .map(|r| format!("/{}/{}/{}", r.module, r.key1, r.key2))
            .collect())
    }

    async fn list_values(&self, prefix: &str) -> Result<Vec<Bytes>> {
        let mut items = self.list(prefix).await?;
        let mut keys = items.keys().map(|k| k.to_string()).collect::<Vec<_>>();
        keys.sort();
        Ok(keys
            .into_iter()
            .map(|k| items.remove(&k).unwrap())
            .collect())
    }

    async fn list_values_by_start_dt(
        &self,
        prefix: &str,
        start_dt: Option<(i64, i64)>,
    ) -> Result<Vec<(i64, Bytes)>> {
        if start_dt.is_none() || start_dt == Some((0, 0)) {
            let vals = self.list_values(prefix).await?;
            return Ok(vals.into_iter().map(|v| (0, v)).collect());
        }

        let (min_dt, max_dt) = start_dt.unwrap();
        let (module, key1, key2) = super::parse_key(prefix);
        let mut sql = "SELECT id, module, key1, key2, start_dt, value FROM meta".to_string();
        if !module.is_empty() {
            sql = format!("{} WHERE module = '{}'", sql, module);
        }
        if !key1.is_empty() {
            sql = format!("{} AND key1 = '{}'", sql, key1);
        }
        if !key2.is_empty() {
            sql = format!("{} AND (key2 = '{}' OR key2 LIKE '{}/%')", sql, key2, key2);
        }
        sql = format!(
            "{} AND start_dt >= {} AND start_dt <= {}",
            sql, min_dt, max_dt
        );
        sql = format!("{} ORDER BY start_dt ASC", sql);

        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
        let ret = sqlx::query_as::<_, super::MetaRecord>(&sql)
            .fetch_all(&pool)
            .await?;
        Ok(ret
            .into_iter()
            .map(|r| (r.start_dt, Bytes::from(r.value)))
            .collect())
    }

    async fn count(&self, prefix: &str) -> Result<i64> {
        let (module, key1, key2) = super::parse_key(prefix);
        let mut sql = "SELECT COUNT(*) AS num FROM meta".to_string();
        if !module.is_empty() {
            sql = format!("{} WHERE module = '{}'", sql, module);
        }
        if !key1.is_empty() {
            sql = format!("{} AND key1 = '{}'", sql, key1);
        }
        if !key2.is_empty() {
            sql = format!("{} AND (key2 = '{}' OR key2 LIKE '{}/%')", sql, key2, key2);
        }
        let pool = CLIENT.clone();
        DB_QUERY_NUMS.with_label_values(&["select", "meta"]).inc();
        let count: i64 = sqlx::query_scalar(&sql).fetch_one(&pool).await?;
        Ok(count)
    }

    async fn watch(&self, _prefix: &str) -> Result<Arc<mpsc::Receiver<super::Event>>> {
        Err(Error::NotImplemented)
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn add_start_dt_column(&self) -> Result<()> {
        create_meta_backup().await?;
        add_start_dt_column().await?;
        Ok(())
    }
}

pub async fn create_table() -> Result<()> {
    let pool = CLIENT.clone();

    DB_QUERY_NUMS.with_label_values(&["create", "meta"]).inc();
    // create table
    _ = sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS meta
(
    id       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    module   VARCHAR(100) not null,
    key1     VARCHAR(256) not null,
    key2     VARCHAR(256) not null,
    start_dt BIGINT not null,
    value    TEXT not null
);
    "#,
    )
    .execute(&pool)
    .await?;

    // create start_dt column for old version <= 0.9.2
    DB_QUERY_NUMS
        .with_label_values(&["select", "information_schema.columns"])
        .inc();
    let has_start_dt = sqlx::query_scalar::<_,i64>("SELECT count(*) FROM INFORMATION_SCHEMA.COLUMNS WHERE table_name='meta' AND column_name='start_dt';")
            .fetch_one(&pool)
            .await?;
    if has_start_dt == 0 {
        add_start_dt_column().await?;
    }

    // create table index
    create_index(IndexStatement::new(
        "meta_module_idx",
        "meta",
        false,
        &["module"],
    ))
    .await?;
    create_index(IndexStatement::new(
        "meta_module_key1_idx",
        "meta",
        false,
        &["module", "key1"],
    ))
    .await?;
    create_index(IndexStatement::new(
        "meta_module_start_dt_idx",
        "meta",
        true,
        &["module", "key1", "key2", "start_dt"],
    ))
    .await?;

    Ok(())
}

async fn add_start_dt_column() -> Result<()> {
    let pool = CLIENT.clone();
    let mut tx = pool.begin().await?;
    DB_QUERY_NUMS.with_label_values(&["alter", "meta"]).inc();
    if let Err(e) = sqlx::query(
        r#"ALTER TABLE meta ADD COLUMN IF NOT EXISTS start_dt BIGINT NOT NULL DEFAULT 0;"#,
    )
    .execute(&mut *tx)
    .await
    {
        log::error!("[POSTGRES] Error in adding column start_dt: {}", e);
        if let Err(e) = tx.rollback().await {
            log::error!("[POSTGRES] Error in rolling back transaction: {}", e);
        }
        return Err(e.into());
    }
    tx.commit().await?;

    // Proceed to drop the index if it exists and create a new one if it does not exist
    create_index(IndexStatement::new(
        "meta_module_start_dt_idx",
        "meta",
        true,
        &["module", "key1", "key2", "start_dt"],
    ))
    .await?;
    delete_index("meta_module_key2_idx", "meta").await?;

    Ok(())
}

async fn create_meta_backup() -> Result<()> {
    let pool = CLIENT.clone();
    let mut tx = pool.begin().await?;
    DB_QUERY_NUMS
        .with_label_values(&["create", "meta_backup_20240330"])
        .inc();
    if let Err(e) =
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS meta_backup_20240330 AS SELECT * FROM meta;"#)
            .execute(&mut *tx)
            .await
    {
        if let Err(e) = tx.rollback().await {
            log::error!(
                "[POSTGRES] rollback create table meta_backup_20240330 error: {}",
                e
            );
        }
        return Err(e.into());
    }
    if let Err(e) = tx.commit().await {
        log::error!(
            "[POSTGRES] commit create table meta_backup_20240330 error: {}",
            e
        );
        return Err(e.into());
    }
    Ok(())
}

pub async fn create_index(index: IndexStatement<'_>) -> Result<()> {
    let client = CLIENT.clone();
    let indices = INDICES.get_or_init(cache_indices).await;
    if indices.contains(&DBIndex {
        name: index.idx_name.into(),
        table: index.table.into(),
    }) {
        return Ok(());
    }
    let unique_str = if index.unique { "UNIQUE" } else { "" };
    log::info!(
        "[POSTGRES] creating index {} on table {}",
        index.idx_name,
        index.table
    );
    DB_QUERY_NUMS
        .with_label_values(&["create", index.table])
        .inc();
    let sql = format!(
        "CREATE {} INDEX IF NOT EXISTS {} ON {} ({});",
        unique_str,
        index.idx_name,
        index.table,
        index.fields.join(",")
    );

    let start = std::time::Instant::now();
    sqlx::query(&sql).execute(&client).await?;
    let time = start.elapsed().as_secs_f64();
    DB_QUERY_TIME
        .with_label_values(&["create_index", index.table])
        .observe(time);
    log::info!("[POSTGRES] index {} created successfully", index.idx_name);
    Ok(())
}

pub async fn delete_index(idx_name: &str, table: &str) -> Result<()> {
    let client = CLIENT.clone();
    let indices = INDICES.get_or_init(cache_indices).await;
    if !indices.contains(&DBIndex {
        name: idx_name.into(),
        table: table.into(),
    }) {
        return Ok(());
    }
    log::info!("[POSTGRES] deleting index {} on table {}", idx_name, table);
    let sql = format!("DROP INDEX IF EXISTS {};", idx_name,);
    DB_QUERY_NUMS.with_label_values(&["drop", table]).inc();
    let start = std::time::Instant::now();
    sqlx::query(&sql).execute(&client).await?;
    let time = start.elapsed().as_secs_f64();
    DB_QUERY_TIME
        .with_label_values(&["drop_index", table])
        .observe(time);
    log::info!("[POSTGRES] index {} deleted successfully", idx_name);
    Ok(())
}
