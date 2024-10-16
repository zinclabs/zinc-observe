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
        let query = r#"
            CREATE TABLE IF NOT EXISTS short_urls (
                id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                short_id VARCHAR(32) NOT NULL,
                original_url VARCHAR(2048) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_ts BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) * 1000000)::BIGINT
            );
            "#;
        sqlx::query(query).execute(&pool).await?;
        add_created_ts_column().await?;
        Ok(())
    }

    /// Create index for short_urls at short_id and original_url
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

    /// Add a new entry to the short_urls table
    async fn add(&self, short_id: &str, original_url: &str) -> Result<()> {
        let pool = CLIENT.clone();
        let query = r#"INSERT INTO short_urls (short_id, original_url) VALUES ($1, $2);"#;
        let result = sqlx::query(query)
            .bind(short_id)
            .bind(original_url)
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

async fn add_created_ts_column() -> Result<()> {
    let pool = CLIENT.clone();

    // Check if the created_ts column exists
    let check_query = r#"
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.columns
                WHERE table_name='short_urls'
                AND column_name='created_ts'
            );
        "#;

    let exists: (bool,) = sqlx::query_as(check_query).fetch_one(&pool).await?;

    if !exists.0 {
        log::info!("[POSTGRES] Adding created_ts column to short_urls table");

        // Column does not exist, so we can add it
        let alter_query = r#"
                ALTER TABLE short_urls
                ADD COLUMN created_ts BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) * 1000000)::BIGINT;
            "#;

        if let Err(e) = sqlx::query(alter_query).execute(&pool).await {
            log::error!(
                "[POSTGRES] Unexpected error in adding created_ts column: {}",
                e
            );
            return Err(e.into());
        }
    } else {
        log::info!("[POSTGRES] created_ts column already exists in short_urls table");
    }

    Ok(())
}
