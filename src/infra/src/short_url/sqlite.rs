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
use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::{
    db::sqlite::{create_index, CLIENT_RO, CLIENT_RW},
    errors::{DbError, Error, Result},
    short_url::{ShortUrl, ShortUrlRecord},
};

pub struct SqliteShortUrl {}

impl SqliteShortUrl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SqliteShortUrl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShortUrl for SqliteShortUrl {
    /// Creates the short_urls table if it does not exist
    async fn create_table(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;

        sqlx::query(
            r#"
                CREATE TABLE IF NOT EXISTS short_urls
                (
                    id           INTEGER PRIMARY KEY AUTOINCREMENT,
                    short_id     VARCHAR(32) NOT NULL,
                    original_url VARCHAR(2048) NOT NULL,
                    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    created_ts   BIGINT DEFAULT (CAST(strftime('%s', 'now') AS BIGINT) * 1000000)
                );
                "#,
        )
        .execute(&*client)
        .await?;

        // release lock
        drop(client);

        add_created_ts_column().await?;
        Ok(())
    }

    /// Creates indexes on the short_urls table
    async fn create_table_index(&self) -> Result<()> {
        create_index("short_urls_short_id_idx", "short_urls", true, &["short_id"]).await?;
        create_index(
            "short_urls_created_at_idx",
            "short_urls",
            false,
            &["created_at"],
        )
        .await?;
        Ok(())
    }

    /// Adds a new short URL entry
    async fn add(&self, short_id: &str, original_url: &str) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let mut tx = client.begin().await?;
        let query = r#"INSERT INTO short_urls (short_id, original_url) VALUES ($1, $2);"#;
        let result = sqlx::query(query)
            .bind(short_id)
            .bind(original_url)
            .execute(&mut *tx)
            .await;

        if result.is_ok() {
            tx.commit().await?;
        }

        // release lock
        drop(client);

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
                Err(Error::DbError(DbError::UniqueViolation))
            }
            Err(e) => Err(Error::SqlxError(e)),
        }
    }

    /// Removes a short URL entry by short_id
    async fn remove(&self, short_id: &str) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let query = r#"DELETE FROM short_urls WHERE short_id = $1;"#;
        sqlx::query(query).bind(short_id).execute(&*client).await?;
        drop(client);

        Ok(())
    }

    /// Retrieves a short URL entry by short_id
    async fn get(&self, short_id: &str) -> Result<ShortUrlRecord> {
        let client = CLIENT_RO.clone();
        let query = r#"SELECT short_id, original_url FROM short_urls WHERE short_id = $1;"#;
        let row = sqlx::query_as::<_, ShortUrlRecord>(query)
            .bind(short_id)
            .fetch_one(&client)
            .await?;
        Ok(row)
    }

    /// Lists all short URL entries
    async fn list(&self, limit: Option<i64>) -> Result<Vec<ShortUrlRecord>> {
        let client = CLIENT_RO.clone();
        let mut query =
            r#"SELECT short_id, original_url FROM short_urls ORDER BY id DESC"#.to_string();

        if limit.is_some() {
            query.push_str(" LIMIT $1");
        }

        let query_builder = sqlx::query_as::<_, ShortUrlRecord>(&query);
        let query_builder = if let Some(limit_value) = limit {
            query_builder.bind(limit_value)
        } else {
            query_builder
        };

        let rows = query_builder.fetch_all(&client).await?;
        Ok(rows)
    }

    /// Checks if a short_id exists in the database
    async fn contains(&self, short_id: &str) -> Result<bool> {
        let client = CLIENT_RO.clone();
        let query = r#"SELECT 1 FROM short_urls WHERE short_id = $1"#;
        let rows = sqlx::query(query).bind(short_id).fetch_all(&client).await?;
        Ok(rows.is_empty())
    }

    /// Returns the number of entries in the short_urls table
    async fn len(&self) -> usize {
        let client = CLIENT_RO.clone();

        let result = match sqlx::query(r#"SELECT COUNT(*) as num FROM short_urls;"#)
            .fetch_one(&client)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                log::error!("[SQLITE] short_urls len error: {}", e);
                return 0;
            }
        };

        match result.try_get::<i64, &str>("num") {
            Ok(count) => count as usize,
            Err(_) => 0,
        }
    }

    /// Clears all entries from the short_urls table
    async fn clear(&self) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;

        sqlx::query(r#"DELETE FROM short_urls;"#)
            .execute(&*client)
            .await?;

        drop(client);

        Ok(())
    }

    /// Checks if the short_urls table is empty
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    async fn get_expired(
        &self,
        expired_before: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<String>> {
        let client = CLIENT_RO.clone();

        let mut query = r#"
            SELECT short_id FROM short_urls
            WHERE created_at < $1
            "#
        .to_string();

        if limit.is_some() {
            query.push_str(" LIMIT $2");
        }

        let mut query = sqlx::query_as(&query).bind(expired_before);

        if let Some(limit_value) = limit {
            query = query.bind(limit_value);
        }

        let expired_short_ids: Vec<(String,)> = query.fetch_all(&client).await?;
        let expired_short_ids: Vec<String> = expired_short_ids
            .into_iter()
            .map(|(short_id,)| short_id)
            .collect();
        Ok(expired_short_ids)
    }

    async fn batch_remove(&self, short_ids: Vec<String>) -> Result<()> {
        if short_ids.is_empty() {
            return Ok(());
        }
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let mut tx = client.begin().await?;

        let query = format!(
            "
            DELETE FROM short_urls
            WHERE short_id IN ({})
        ",
            short_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        );

        let mut sql_query = sqlx::query(&query);
        for short_id in &short_ids {
            sql_query = sql_query.bind(short_id);
        }

        let result = sql_query.execute(&mut *tx).await;

        if result.is_ok() {
            tx.commit().await?;
        }

        // release lock
        drop(client);

        Ok(())
    }
}

async fn add_created_ts_column() -> Result<()> {
    let client = CLIENT_RW.clone();
    let client = client.lock().await;
    let mut tx = client.begin().await?;

    // Check if the created_ts column exists
    let check_query = r#"
        PRAGMA table_info(short_urls);
    "#;

    let columns = sqlx::query_as::<_, (i32, String, String, i32, Option<String>, i32)>(check_query)
        .fetch_all(&mut *tx)
        .await?;

    let column_exists = columns.iter().any(|(_, name, ..)| name == "created_ts");

    if !column_exists {
        log::info!("[SQLITE] Adding created_ts column to short_urls table");

        // Add the created_ts column
        let alter_query = r#"
            ALTER TABLE short_urls
            ADD COLUMN created_ts BIGINT;
        "#;

        if let Err(e) = sqlx::query(alter_query).execute(&mut *tx).await {
            log::error!("[SQLITE] Error adding created_ts column: {}", e);
            return Err(e.into());
        }

        // Update existing rows with the current timestamp in microseconds
        let update_query = r#"
            UPDATE short_urls
            SET created_ts = CAST(strftime('%s', 'now') AS BIGINT) * 1000000;
        "#;

        if let Err(e) = sqlx::query(update_query).execute(&mut *tx).await {
            log::error!("[SQLITE] Error updating created_ts column: {}", e);
            return Err(e.into());
        }

        tx.commit().await?;
        log::info!("[SQLITE] created_ts column successfully added and updated");
    } else {
        log::info!("[SQLITE] created_ts column already exists in short_urls table");
    }

    // release lock
    drop(client);

    Ok(())
}
