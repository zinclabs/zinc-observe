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
use crate::db::sqlite::delete_index;

fn create_short_urls_table_query(table_name: &str) -> String {
    // `created_ts` is a Unix timestamp in microseconds
    format!(
        r#"
        CREATE TABLE IF NOT EXISTS {table_name}
        (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            short_id     VARCHAR(32) NOT NULL,
            original_url VARCHAR(2048) NOT NULL,
            created_ts   BIGINT NOT NULL
        );
        "#,
        table_name = table_name
    )
}

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

        let create_table_query = create_short_urls_table_query("short_urls");

        sqlx::query(&create_table_query)
        .execute(&*client)
        .await?;

        // release lock
        drop(client);

        migrate_created_at_to_created_ts().await?;
        Ok(())
    }

    /// Creates indexes on the short_urls table
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

    /// Adds a new short URL entry
    async fn add(&self, short_id: &str, original_url: &str) -> Result<()> {
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        let created_ts = Utc::now().timestamp_micros();

        let mut tx = client.begin().await?;

        let query =
            r#"INSERT INTO short_urls (short_id, original_url, created_ts) VALUES ($1, $2, $3);"#;
        let result = sqlx::query(query)
            .bind(short_id)
            .bind(original_url)
            .bind(created_ts)
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
        let expired_before_ts = expired_before.timestamp_micros();

        let mut query = r#"
            SELECT short_id FROM short_urls
            WHERE created_at < $1
            "#
        .to_string();

        if limit.is_some() {
            query.push_str(" LIMIT $2");
        }

        let mut query = sqlx::query_as(&query).bind(expired_before_ts);

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

/// Main migration function to migrate from `created_at` to `created_ts`
async fn migrate_created_at_to_created_ts() -> Result<()> {
    migrate_data_to_new_table().await?;
    migrate_schema_changes().await?;
    log::info!("[SQLITE] Migration from `created_at` to `created_ts` completed successfully");
    Ok(())
}

async fn is_migration_required() -> Result<bool> {
    let client = CLIENT_RO.clone();

    // Query to get the table information
    let check_query = r#"PRAGMA table_info(short_urls);"#;
    let columns = sqlx::query(&check_query).fetch_all(&client).await?;

    // Check if `created_at` column exists
    let created_at_exists = columns.iter().any(|column| {
        let column_name: String = column.get("name");
        column_name == "created_at"
    });

    if created_at_exists {
        log::info!("[SQLITE] Migration required: `created_at` column exists");
    } else {
        log::info!("[SQLITE] Migration not required: `created_at` column does not exist");
    }

    Ok(created_at_exists)
}


/// Data migration function to copy data from `short_urls` to `short_urls_new` and convert `created_at` to `created_ts`
async fn migrate_data_to_new_table() -> Result<()> {
    if !is_migration_required().await? {
        return Ok(());
    }

    delete_index("short_urls_short_id_idx", "short_urls").await?;
    delete_index("short_urls_created_at_idx", "short_urls").await?;

    let client = CLIENT_RW.clone();
    let client = client.lock().await;

    // Step 1: Create a new table without `created_at` and with `created_ts`
    let mut tx = client.begin().await?;
    let create_table_query = create_short_urls_table_query("short_urls_new");
    sqlx::query(&create_table_query).execute(&mut *tx).await?;

    // Step 2: Fetch data from the old table
    let select_data_query = r#"
        SELECT id, short_id, original_url
        FROM short_urls;
    "#;

    let rows = sqlx::query(select_data_query)
        .fetch_all(&mut *tx)
        .await?;

    // Step 3: Insert data into the new table
    let created_ts = Utc::now().timestamp_micros();
    for row in rows {
        let id: i64 = row.get("id");
        let short_id: String = row.get("short_id");
        let original_url: String = row.get("original_url");

        // Insert the row into the new table
        let insert_query = r#"
            INSERT INTO short_urls_new (id, short_id, original_url, created_ts)
            VALUES ($1, $2, $3, $4);
        "#;

        sqlx::query(insert_query)
            .bind(id)
            .bind(short_id)
            .bind(original_url)
            .bind(created_ts)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    log::info!("[SQLITE] Data migration to `short_urls_new` completed successfully");

    Ok(())
}

/// Schema migration function to drop old table and indexes, and rename the new table
async fn migrate_schema_changes() -> Result<()> {
    let client = CLIENT_RW.clone();
    let client = client.lock().await;

    // Step 2: Start a new transaction for schema changes
    let mut tx = client.begin().await?;

    // Step 3: Drop the old table
    let drop_old_table_query = "DROP TABLE short_urls;";
    sqlx::query(drop_old_table_query).execute(&mut *tx).await?;

    // Step 4: Rename the new table to the original table name
    let rename_table_query = "ALTER TABLE short_urls_new RENAME TO short_urls;";
    sqlx::query(rename_table_query).execute(&mut *tx).await?;

    // Commit the schema changes transaction
    tx.commit().await?;
    log::info!("[SQLITE] Schema migration completed: renamed `short_urls_new` to `short_urls`");

    Ok(())
}
