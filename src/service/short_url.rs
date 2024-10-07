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

use chrono::Utc;
use config::{get_config, utils::md5};
use infra::{
    errors::{DbError, Error},
    short_url::ShortUrlRecord,
};

use crate::service::db;

pub fn get_base_url() -> String {
    let config = get_config();
    format!("{}{}", config.common.web_url, config.common.base_uri)
}

async fn store_short_url(short_id: &str, original_url: &str) -> Result<String, anyhow::Error> {
    let entry = ShortUrlRecord::new(short_id.to_string(), original_url.to_string());
    db::short_url::set(short_id, entry.clone()).await?;
    Ok(format!("{}/short/{}", get_base_url(), entry.short_id))
}

fn generate_short_id(original_url: &str, timestamp: Option<i64>) -> String {
    match timestamp {
        Some(ts) => {
            let input = format!("{}{}", original_url, ts);
            md5::short_hash(&input)
        }
        None => md5::short_hash(original_url),
    }
}

/// Shortens the given original URL and stores it in the database
pub async fn shorten(original_url: &str) -> Result<String, anyhow::Error> {
    let mut short_id = generate_short_id(original_url, None);

    if let Ok(existing_url) = db::short_url::get(short_id.as_str()).await {
        if existing_url == original_url {
            return Ok(format!("{}/short/{}", get_base_url(), short_id));
        }
    }

    let result = store_short_url(&short_id, original_url).await;

    match result {
        Ok(url) => Ok(url),
        Err(e) => {
            if let Some(infra_error) = e.downcast_ref::<Error>() {
                match infra_error {
                    Error::DbError(DbError::UniqueViolation) => {
                        let timestamp = Utc::now().timestamp();
                        short_id = generate_short_id(original_url, Some(timestamp));
                        store_short_url(&short_id, original_url).await
                    }
                    _ => Err(e),
                }
            } else {
                Err(e)
            }
        }
    }
}

/// Retrieves the original URL corresponding to the given short ID
pub async fn retrieve(short_id: &str) -> Option<String> {
    db::short_url::get(short_id).await.ok()
}

/// Extracts the short ID from the shortened URL
pub fn get_short_id_from_url(short_url: &str) -> Option<String> {
    let prefix = format!("{}/short/", get_base_url());
    short_url.strip_prefix(&prefix).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_shorten_and_retrieve() {
        let original_url = "https://www.example.com/some/long/url";
        let short_url = shorten(original_url).await.unwrap();
        let short_id = get_short_id_from_url(&short_url).unwrap();

        let retrieved_url = retrieve(&short_id).await.expect("Failed to retrieve URL");
        assert_eq!(retrieved_url, original_url);

        let short_id = get_short_id_from_url(&short_url).unwrap();
        assert_eq!(short_id.len(), 16);
    }

    #[tokio::test]
    #[ignore]
    async fn test_retrieve_nonexistent_short_id() {
        let retrieved_url = retrieve("nonexistent_id").await;
        assert!(retrieved_url.is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_unique_original_urls() {
        let original_url = "https://www.example.com/some/long/url";

        let short_url1 = shorten(original_url).await.unwrap();
        let short_url2 = shorten(original_url).await.unwrap();

        // Should return the same short_id
        assert_eq!(short_url1, short_url2);
    }
}
