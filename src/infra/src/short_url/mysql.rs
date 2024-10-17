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
    db::mysql::{create_index, CLIENT},
    errors::{DbError, Error, Result},
    short_url::{ShortUrl, ShortUrlRecord},
};

pub struct MysqlShortUrl {}

impl MysqlShortUrl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MysqlShortUrl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShortUrl for MysqlShortUrl {
    /// Create table short_urls
    async fn create_table(&self) -> Result<()> {
        let pool = CLIENT.clone();
        // `created_ts` is unix timestamp in microseconds
        let query = r#"
            CREATE TABLE IF NOT EXISTS short_urls (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                short_id VARCHAR(32) NOT NULL,
                original_url VARCHAR(2048) NOT NULL,
                created_ts BIGINT NOT NULL
            );
        "#;
        sqlx::query(query).execute(&pool).await?;
        manage_short_urls_schema().await?;
        Ok(())
    }

    /// Create index for short_urls at short_id and created_ts
    async fn create_table_index(&self) -> Result<()> {
        create_index("short_urls_short_id_idx", "short_urls", true, &["short_id"]).await?;
        create_index(
            "short_urls_created_ts_idx",
            "short_urls",
            false,
            &["created_ts"],
        )
        .await?;
        Ok(())
    }

    /// Add a new entry to the short_urls table
    async fn add(&self, short_id: &str, original_url: &str) -> Result<()> {
        let pool = CLIENT.clone();
        let created_ts = Utc::now().timestamp_micros();

        let query =
            r#"INSERT INTO short_urls (short_id, original_url, created_ts) VALUES (?, ?, ?);"#;
        let result = sqlx::query(query)
            .bind(short_id)
            .bind(original_url)
            .bind(created_ts)
            .execute(&pool)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(err)) if err.is_unique_violation() => {
                Err(Error::DbError(DbError::UniqueViolation))
            }
            Err(e) => Err(Error::SqlxError(e)),
        }
    }

    /// Remove an entry from the short_urls table
    async fn remove(&self, short_id: &str) -> Result<()> {
        let pool = CLIENT.clone();
        let query = r#"DELETE FROM short_urls WHERE short_id = ?;"#;
        sqlx::query(query).bind(short_id).execute(&pool).await?;
        Ok(())
    }

    /// Get an entry from the short_urls table
    async fn get(&self, short_id: &str) -> Result<ShortUrlRecord> {
        let pool = CLIENT.clone();
        let query = r#"SELECT short_id, original_url FROM short_urls WHERE short_id = ?;"#;
        let row = sqlx::query_as::<_, ShortUrlRecord>(query)
            .bind(short_id)
            .fetch_one(&pool)
            .await?;
        Ok(row)
    }

    /// List all entries from the short_urls table
    async fn list(&self, limit: Option<i64>) -> Result<Vec<ShortUrlRecord>> {
        let pool = CLIENT.clone();
        let mut query =
            r#"SELECT short_id, original_url FROM short_urls ORDER BY created_ts DESC"#.to_string();

        if limit.is_some() {
            query.push_str(" LIMIT ?");
        }

        let query_builder = sqlx::query_as::<_, ShortUrlRecord>(&query);
        let query_builder = if let Some(limit_value) = limit {
            query_builder.bind(limit_value)
        } else {
            query_builder
        };

        let rows = query_builder.fetch_all(&pool).await?;
        Ok(rows)
    }

    /// Check if an entry exists in the short_urls table
    async fn contains(&self, short_id: &str) -> Result<bool> {
        let pool = CLIENT.clone();
        let query = r#"SELECT 1 FROM short_urls WHERE short_id = ?;"#;
        let rows = sqlx::query(query).bind(short_id).fetch_all(&pool).await?;
        Ok(!rows.is_empty())
    }

    /// Get the number of entries in the short_urls table
    async fn len(&self) -> usize {
        let pool = CLIENT.clone();
        let ret = match sqlx::query(r#"SELECT COUNT(*) AS num FROM short_urls;"#)
            .fetch_one(&pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[MYSQL] short_urls len error: {}", e);
                return 0;
            }
        };

        match ret.try_get::<i64, &str>("num") {
            Ok(v) => v as usize,
            _ => 0,
        }
    }

    /// Clear all entries from the short_urls table
    async fn clear(&self) -> Result<()> {
        let pool = CLIENT.clone();
        let query = r#"DELETE FROM short_urls;"#;
        match sqlx::query(query).execute(&pool).await {
            Ok(_) => log::info!("[SHORT_URL] short_urls table cleared"),
            Err(e) => log::error!("[MYSQL] short_urls table clear error: {}", e),
        }
        Ok(())
    }

    /// Check if the short_urls table is empty
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    async fn get_expired(
        &self,
        expired_before: DateTime<Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<String>> {
        let pool = CLIENT.clone();
        let expired_before_ts = expired_before.timestamp_micros();

        let mut query = r#"
            SELECT short_id FROM short_urls
            WHERE created_ts < ?
            "#
        .to_string();

        if limit.is_some() {
            query.push_str(" LIMIT ?");
        }

        let mut query = sqlx::query_as(&query).bind(expired_before_ts);

        if let Some(limit_value) = limit {
            query = query.bind(limit_value);
        }

        let expired_short_ids: Vec<(String,)> = query.fetch_all(&pool).await?;
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
        let pool = CLIENT.clone();

        let query = format!(
            "DELETE FROM short_urls WHERE short_id IN ({})",
            short_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        );
        let mut sql_query = sqlx::query(&query);
        for short_id in &short_ids {
            sql_query = sql_query.bind(short_id);
        }

        sql_query.execute(&pool).await?;

        Ok(())
    }
}

// Main function to manage the schema changes for `short_urls` table.
//
// This function performs the following steps:
// 1. Drops the `created_at` column if it exists.
// 2. Drops the index on `created_at` if it exists.
// 3. Adds the `created_ts` column if it doesn't exist.
async fn manage_short_urls_schema() -> Result<()> {
    let pool = CLIENT.clone();

    drop_column_if_exists(&pool, "created_at").await?;
    drop_index_if_exists(&pool, "short_urls_created_at_idx").await?;
    add_created_ts_column_if_not_exists(&pool).await?;

    Ok(())
}

// Checks if a column exists in the `short_urls` table.
async fn column_exists(pool: &sqlx::Pool<sqlx::MySql>, column_name: &str) -> Result<bool> {
    let query = format!(
        r#"
        SELECT COUNT(*)
        FROM information_schema.columns
        WHERE table_name = 'short_urls'
        AND column_name = '{}';
        "#,
        column_name
    );

    let exists: (i64,) = sqlx::query_as(&query).fetch_one(pool).await?;
    Ok(exists.0 > 0)
}

// Drops a column from the `short_urls` table if it exists.
async fn drop_column_if_exists(pool: &sqlx::Pool<sqlx::MySql>, column_name: &str) -> Result<()> {
    if column_exists(pool, column_name).await? {
        log::info!(
            "[MYSQL] Dropping {} column from short_urls table",
            column_name
        );

        let query = format!(
            r#"
            ALTER TABLE short_urls
            DROP COLUMN {};
            "#,
            column_name
        );

        if let Err(e) = sqlx::query(&query).execute(pool).await {
            log::error!(
                "[MYSQL] Unexpected error in dropping {} column: {}",
                column_name,
                e
            );
            return Err(e.into());
        }

        log::info!("[MYSQL] Successfully dropped {} column", column_name);
    } else {
        log::info!(
            "[MYSQL] {} column does not exist in short_urls table",
            column_name
        );
    }

    Ok(())
}

// Checks if an index exists on the `short_urls` table.
async fn index_exists(pool: &sqlx::Pool<sqlx::MySql>, index_name: &str) -> Result<bool> {
    let query = format!(
        r#"
        SELECT COUNT(*)
        FROM information_schema.statistics
        WHERE table_name = 'short_urls'
        AND index_name = '{}';
        "#,
        index_name
    );

    let exists: (i64,) = sqlx::query_as(&query).fetch_one(pool).await?;
    Ok(exists.0 > 0)
}

// Drops an index from the `short_urls` table if it exists.
async fn drop_index_if_exists(pool: &sqlx::Pool<sqlx::MySql>, index_name: &str) -> Result<()> {
    if index_exists(pool, index_name).await? {
        log::info!("[MYSQL] Dropping index {} on short_urls table", index_name);

        let query = format!(
            r#"
            DROP INDEX {} ON short_urls;
            "#,
            index_name
        );

        if let Err(e) = sqlx::query(&query).execute(pool).await {
            log::error!(
                "[MYSQL] Unexpected error in dropping index {}: {}",
                index_name,
                e
            );
            return Err(e.into());
        }

        log::info!("[MYSQL] Successfully dropped index {}", index_name);
    } else {
        log::info!(
            "[MYSQL] No index named {} found on short_urls table",
            index_name
        );
    }

    Ok(())
}

// Adds the `created_ts` column to the `short_urls` table if it doesn't already exist.
// If the column is added, it populates existing rows with a default value.
async fn add_created_ts_column_if_not_exists(pool: &sqlx::Pool<sqlx::MySql>) -> Result<()> {
    if !column_exists(pool, "created_ts").await? {
        log::info!("[MYSQL] Adding created_ts column to short_urls table");

        // Step 1: Add the column as nullable
        let query = r#"
            ALTER TABLE short_urls
            ADD COLUMN created_ts BIGINT;
        "#;

        if let Err(e) = sqlx::query(query).execute(pool).await {
            log::error!("[MYSQL] Error adding nullable created_ts column: {}", e);
            return Err(e.into());
        }

        // Step 2: Update existing rows with a default timestamp
        let fallback_timestamp = Utc::now().timestamp_micros();
        let update_query = format!(
            r#"
            UPDATE short_urls
            SET created_ts = {}
            WHERE created_ts IS NULL;
            "#,
            fallback_timestamp
        );

        if let Err(e) = sqlx::query(&update_query).execute(pool).await {
            log::error!(
                "[MYSQL] Error updating existing rows with created_ts: {}",
                e
            );
            return Err(e.into());
        }

        log::info!("[MYSQL] Successfully updated existing rows with created_ts");

        // Step 3: Alter the column to be NOT NULL after populating existing rows
        let alter_query = r#"
            ALTER TABLE short_urls
            MODIFY COLUMN created_ts BIGINT NOT NULL;
        "#;

        if let Err(e) = sqlx::query(alter_query).execute(pool).await {
            log::error!("[MYSQL] Error setting created_ts column to NOT NULL: {}", e);
            return Err(e.into());
        }

        log::info!("[MYSQL] Successfully added and updated created_ts column");
    } else {
        log::info!("[MYSQL] created_ts column already exists in short_urls table");
    }

    Ok(())
}
