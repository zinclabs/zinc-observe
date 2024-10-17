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
    db::postgres::{create_index, CLIENT},
    errors::{DbError, Error, Result},
    short_url::{ShortUrl, ShortUrlRecord},
};

pub struct PostgresShortUrl {}

impl PostgresShortUrl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PostgresShortUrl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShortUrl for PostgresShortUrl {
    /// Create table short_urls
    async fn create_table(&self) -> Result<()> {
        let pool = CLIENT.clone();
        // `created_ts` is a Unix timestamp in microseconds
        let query = r#"
            CREATE TABLE IF NOT EXISTS short_urls (
                id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                short_id VARCHAR(32) NOT NULL,
                original_url VARCHAR(2048) NOT NULL,
                created_ts BIGINT NOT NULL
            );
            "#;
        sqlx::query(query).execute(&pool).await?;
        manage_short_urls_schema().await?;
        Ok(())
    }

    /// Create index for short_urls at short_id and original_url
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

        let query = r#"INSERT INTO short_urls (short_id, original_url, created_ts) VALUES ($1, $2, $3);"#;
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
        let query = r#"DELETE FROM short_urls WHERE short_id = $1;"#;
        sqlx::query(query).bind(short_id).execute(&pool).await?;
        Ok(())
    }

    /// Get an entry from the short_urls table
    async fn get(&self, short_id: &str) -> Result<ShortUrlRecord> {
        let pool = CLIENT.clone();
        let query = r#"SELECT short_id, original_url FROM short_urls WHERE short_id = $1;"#;
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
            query.push_str(" LIMIT $1");
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
        let query = r#"SELECT 1 FROM short_urls WHERE short_id = $1"#;
        let rows = sqlx::query(query).bind(short_id).fetch_all(&pool).await?;
        Ok(!rows.is_empty())
    }

    /// Get the number of entries in the short_urls table
    async fn len(&self) -> usize {
        let pool = CLIENT.clone();
        let ret = match sqlx::query(r#"SELECT COUNT(*)::BIGINT AS num FROM short_urls;"#)
            .fetch_one(&pool)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("[POSTGRES] short_urls len error: {}", e);
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
            Err(e) => log::error!("[POSTGRES] short_urls table clear error: {}", e),
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
            WHERE created_ts < $1
            "#
        .to_string();

        if limit.is_some() {
            query.push_str(" LIMIT $2");
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

        let query = r#"
            DELETE FROM short_urls
            WHERE short_id = ANY($1)
        "#;

        sqlx::query(query).bind(&short_ids).execute(&pool).await?;
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
async fn column_exists(pool: &sqlx::Pool<sqlx::Postgres>, column_name: &str) -> Result<bool> {
    let query = format!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_name = 'short_urls'
            AND column_name = '{}'
        );
        "#,
        column_name
    );

    let exists: (bool,) = sqlx::query_as(&query).fetch_one(pool).await?;
    Ok(exists.0)
}

// Drops a column from the `short_urls` table if it exists.
async fn drop_column_if_exists(pool: &sqlx::Pool<sqlx::Postgres>, column_name: &str) -> Result<()> {
    if column_exists(pool, column_name).await? {
        log::info!(
            "[POSTGRES] Dropping {} column from short_urls table",
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
                "[POSTGRES] Unexpected error in dropping {} column: {}",
                column_name,
                e
            );
            return Err(e.into());
        }

        log::info!("[POSTGRES] Successfully dropped {} column", column_name);
    } else {
        log::info!(
            "[POSTGRES] {} column does not exist in short_urls table",
            column_name
        );
    }

    Ok(())
}

// Checks if an index exists on the `short_urls` table.
async fn index_exists(pool: &sqlx::Pool<sqlx::Postgres>, index_name: &str) -> Result<bool> {
    let query = format!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM pg_indexes
            WHERE tablename = 'short_urls'
            AND indexname = '{}'
        );
        "#,
        index_name
    );

    let exists: (bool,) = sqlx::query_as(&query).fetch_one(pool).await?;
    Ok(exists.0)
}

// Drops an index from the `short_urls` table if it exists.
async fn drop_index_if_exists(pool: &sqlx::Pool<sqlx::Postgres>, index_name: &str) -> Result<()> {
    if index_exists(pool, index_name).await? {
        log::info!("[POSTGRES] Dropping index {} on short_urls table", index_name);

        let query = format!(
            r#"
            DROP INDEX IF EXISTS {};
            "#,
            index_name
        );

        if let Err(e) = sqlx::query(&query).execute(pool).await {
            log::error!(
                "[POSTGRES] Unexpected error in dropping index {}: {}",
                index_name,
                e
            );
            return Err(e.into());
        }

        log::info!("[POSTGRES] Successfully dropped index {}", index_name);
    } else {
        log::info!(
            "[POSTGRES] No index named {} found on short_urls table",
            index_name
        );
    }

    Ok(())
}

// Adds the `created_ts` column to the `short_urls` table if it doesn't already exist.
async fn add_created_ts_column_if_not_exists(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<()> {
    if !column_exists(pool, "created_ts").await? {
        log::info!("[POSTGRES] Adding created_ts column to short_urls table");

        // Step 1: Add the column as nullable
        let query = r#"
            ALTER TABLE short_urls
            ADD COLUMN created_ts BIGINT;
        "#;

        if let Err(e) = sqlx::query(query).execute(pool).await {
            log::error!("[POSTGRES] Error adding nullable created_ts column: {}", e);
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
            log::error!("[POSTGRES] Error updating existing rows with created_ts: {}", e);
            return Err(e.into());
        }

        log::info!("[POSTGRES] Successfully updated existing rows with created_ts");

        // Step 3: Alter the column to be NOT NULL after populating existing rows
        let alter_query = r#"
            ALTER TABLE short_urls
            ALTER COLUMN created_ts SET NOT NULL;
        "#;

        if let Err(e) = sqlx::query(alter_query).execute(pool).await {
            log::error!("[POSTGRES] Error setting created_ts column to NOT NULL: {}", e);
            return Err(e.into());
        }

        log::info!("[POSTGRES] Successfully added and updated created_ts column");
    } else {
        log::info!("[POSTGRES] created_ts column already exists in short_urls table");
    }

    Ok(())
}
