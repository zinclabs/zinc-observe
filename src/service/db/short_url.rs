// Copyright 2025 OpenObserve Inc.
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

use std::sync::Arc;

use anyhow::{anyhow, Context};
use chrono::Utc;
use config::get_config;
use infra::{db::Event, table::short_urls};

use crate::{common::infra::config::SHORT_URLS, service::db};

// DBKey to set short URL's
pub const SHORT_URL_KEY: &str = "/short_urls/";
// GC interval for `SHORT_URLS` cache in days
const SHORT_URL_GC_INTERVAL: i64 = 1; // days
const SHORT_URL_CACHE_LIMIT: i64 = 10_000; // records

pub async fn get(short_id: &str) -> Result<String, anyhow::Error> {
    if let Some(v) = SHORT_URLS.get(short_id) {
        return Ok(v.original_url.to_string());
    }

    let val = short_urls::get(short_id)
        .await
        .map_err(|_| anyhow!("Short URL not found in db"))?;
    let original_url = val.original_url.clone();
    SHORT_URLS.insert(short_id.to_string(), val);
    Ok(original_url)
}

pub async fn set(short_id: &str, entry: short_urls::ShortUrlRecord) -> Result<(), anyhow::Error> {
    if let Err(e) = short_urls::add(short_id, &entry.original_url).await {
        log::error!("Failed to add short URL to DB : {}", e);
        return Err(e).context("Failed to add short URL to DB");
    }

    // trigger watch event cluster coordinator
    cluster::emit_put_event(short_id).await?;
    // trigger watch event super cluster
    #[cfg(feature = "enterprise")]
    super_cluster::emit_put_event(short_id, entry).await?;

    Ok(())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = SHORT_URL_KEY;
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching short URLs");

    // Spawn a background task for garbage collection
    let config = get_config();
    tokio::spawn(run_gc_task(
        days_to_minutes(SHORT_URL_GC_INTERVAL),
        days_to_minutes(config.limit.short_url_retention_days),
    ));

    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_short_url: event channel closed");
                return Ok(());
            }
        };

        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value = match short_urls::get(item_key).await {
                    Ok(val) => val,
                    Err(e) => {
                        log::error!("Error getting value: {}", e);
                        continue;
                    }
                };
                SHORT_URLS.insert(item_key.to_string(), item_value);
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                SHORT_URLS.remove(item_key);
            }
            Event::Empty => {}
        }
    }
}

/// Preload all short URLs from the database into the cache at startup.
pub async fn cache() -> Result<(), anyhow::Error> {
    let ret = short_urls::list(Some(SHORT_URL_CACHE_LIMIT)).await?;
    for row in ret.into_iter() {
        SHORT_URLS.insert(row.short_id.to_owned(), row);
    }
    log::info!("[SHORT_URLS] Cached with len: {}", SHORT_URLS.len());
    Ok(())
}

// Background task to run GC at a regular interval
async fn run_gc_task(gc_interval_minutes: i64, retention_period_minutes: i64) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        gc_interval_minutes as u64 * 60,
    ));

    loop {
        interval.tick().await;

        if let Err(e) = gc_cache(retention_period_minutes).await {
            log::error!("Error during garbage collection: {}", e);
        }
    }
}

// Garbage Collection to clean up expired entries from cache and db
pub async fn gc_cache(retention_period_minutes: i64) -> Result<(), anyhow::Error> {
    log::info!(
        "[SHORT_URLS] Garbage collection start with cache size: {}",
        SHORT_URLS.len(),
    );

    let retention_period = chrono::Duration::minutes(retention_period_minutes);
    let expired_before = Utc::now() - retention_period;

    // get expired ids
    if let Ok(expired_short_ids) = short_urls::get_expired(
        expired_before.timestamp_micros(),
        Some(SHORT_URL_CACHE_LIMIT),
    )
    .await
    {
        if !expired_short_ids.is_empty() {
            // delete from db
            short_urls::batch_remove(expired_short_ids.clone()).await?;

            // delete from cache & notify super cluster
            for short_id in expired_short_ids {
                cluster::emit_delete_event(&short_id).await?;
                #[cfg(feature = "enterprise")]
                super_cluster::emit_delete_event(&short_id).await?;
            }
        }
    }

    log::info!(
        "[SHORT_URLS] Garbage collection end with cache size: {}",
        SHORT_URLS.len(),
    );
    Ok(())
}

fn days_to_minutes(days: i64) -> i64 {
    days * 24 * 60
}

/// Helper functions for sending events to cache watchers in the cluster.
mod cluster {
    use super::SHORT_URL_KEY;

    /// Sends event to the cluster cache watchers indicating that a new short URL entry has been
    /// added.
    pub async fn emit_put_event(short_id: &str) -> Result<(), infra::errors::Error> {
        let key = short_url_key(short_id);
        let value = bytes::Bytes::new();
        let cluster_coordinator = infra::db::get_coordinator().await;
        cluster_coordinator
            .put(&key, value, infra::db::NEED_WATCH, None)
            .await?;
        Ok(())
    }

    /// Sends event to the cluster cache watchers indicating that a short URL entry has been
    /// deleted.
    pub async fn emit_delete_event(short_id: &str) -> Result<(), infra::errors::Error> {
        let key = short_url_key(short_id);
        let cluster_coordinator = infra::db::get_coordinator().await;
        cluster_coordinator
            .delete(&key, false, infra::db::NEED_WATCH, None)
            .await?;
        Ok(())
    }

    fn short_url_key(short_id: &str) -> String {
        format!("{SHORT_URL_KEY}{short_id}")
    }
}

/// Helper fuunctions for sending events to the super cluster queue.
#[cfg(feature = "enterprise")]
mod super_cluster {
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

    use super::{short_urls, SHORT_URL_KEY};

    /// Sends event to super cluster queue for a new short URL entry.
    pub async fn emit_put_event(
        short_id: &str,
        entry: short_urls::ShortUrlRecord,
    ) -> Result<(), infra::errors::Error> {
        let key = short_url_key(short_id);
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::short_url_put(
                &key,
                entry.original_url.into(),
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Sends event to super cluster queue for a deleted short URL entry.
    pub async fn emit_delete_event(short_id: &str) -> Result<(), infra::errors::Error> {
        let key = short_url_key(short_id);
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::short_url_delete(
                &key,
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    fn short_url_key(short_id: &str) -> String {
        format!("{SHORT_URL_KEY}{short_id}")
    }
}
