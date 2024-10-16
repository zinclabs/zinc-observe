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
        let query = r#"
            CREATE TABLE IF NOT EXISTS short_urls (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                short_id VARCHAR(32) NOT NULL,
                original_url VARCHAR(2048) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_ts BIGINT NOT NULL DEFAULT (UNIX_TIMESTAMP(CURRENT_TIMESTAMP(6)) * 1000000)
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
        let query = r#"INSERT INTO short_urls (short_id, original_url) VALUES (?, ?);"#;
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
            r#"SELECT short_id, original_url FROM short_urls ORDER BY id DESC"#.to_string();

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

        let mut query = r#"
            SELECT short_id FROM short_urls
            WHERE created_at < ?
            "#
        .to_string();

        if limit.is_some() {
            query.push_str(" LIMIT ?");
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

async fn add_created_ts_column() -> Result<()> {
    let pool = CLIENT.clone();

    // Check if the created_ts column exists
    let check_query = r#"
            SELECT COUNT(*)
            FROM information_schema.columns
            WHERE table_name = 'short_urls'
            AND column_name = 'created_ts';
        "#;

    let exists: (i64,) = sqlx::query_as(check_query).fetch_one(&pool).await?;

    if exists.0 == 0 {
        log::info!("[MYSQL] Adding created_ts column to short_urls table");

        // Column does not exist, so we can add it
        let alter_query = r#"
                ALTER TABLE short_urls
                ADD COLUMN created_ts BIGINT NOT NULL DEFAULT (UNIX_TIMESTAMP(CURRENT_TIMESTAMP(6)) * 1000000);
            "#;

        if let Err(e) = sqlx::query(alter_query).execute(&pool).await {
            log::error!(
                "[MYSQL] Unexpected error in adding created_ts column: {}",
                e
            );
            return Err(e.into());
        }
    } else {
        log::info!("[MYSQL] created_ts column already exists in short_urls table");
    }

    Ok(())
}
