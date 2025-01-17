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

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

use config::{
    get_config,
    utils::{
        hash::{gxhash, Sum64},
        time::{now, now_micros, second_micros},
    },
};
use hashbrown::HashMap;
use infra::errors::{Error, Result};
use once_cell::sync::Lazy;
use prost::Message;
use tokio::sync::RwLock;

use super::{RangeValue, Value};

const METRICS_INDEX_CACHE_GC_TRIGGER_NUM: usize = 10;
const METRICS_INDEX_CACHE_GC_PERCENT: usize = 10; // 10% of the items will be removed
const METRICS_INDEX_CACHE_MAX_ITEMS: usize = 10;
const METRICS_INDEX_CACHE_BUCKETS: usize = 100;

static CACHE_KEY_SUFFIX: Lazy<AtomicI64> = Lazy::new(|| AtomicI64::new(now_micros()));

static GLOBAL_CACHE: Lazy<Vec<RwLock<MetricsIndex>>> = Lazy::new(|| {
    let cfg = get_config();
    let mut metrics = Vec::with_capacity(METRICS_INDEX_CACHE_BUCKETS);
    for _ in 0..METRICS_INDEX_CACHE_BUCKETS {
        metrics.push(RwLock::new(MetricsIndex::new(
            cfg.limit.metrics_cache_max_entries / METRICS_INDEX_CACHE_BUCKETS,
        )));
    }
    metrics
});

pub async fn init() -> Result<()> {
    let cfg = get_config();
    if !cfg.common.metrics_cache_enabled || !cfg.disk_cache.enabled {
        return Ok(());
    }

    tokio::task::spawn(async move {
        log::info!("Loading disk metrics cache start");
        loop {
            if infra::cache::file_data::disk::LOADING_FROM_DISK_DONE.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        let mut w = infra::cache::file_data::disk::METRICS_RESULT_CACHE
            .write()
            .await;
        let items = std::mem::take(&mut *w);
        for item in items.iter() {
            if let Err(e) = load(item).await {
                log::error!("load disk metrics cache error: {}", e);
            }
        }
        log::info!(
            "Loading disk metrics cache done, total items: {}",
            items.len()
        );
    });
    Ok(())
}

/// Get the samples from the cache
///
/// This function will return the samples from the cache if the samples are found.
/// If the samples are not found, it will return None.
pub async fn get(
    query: &str,
    start: i64,
    end: i64,
    step: i64,
) -> Result<Option<(i64, Vec<proto::cluster_rpc::Series>)>> {
    // get the bucket cache
    let key = get_hash_key(query, step);
    let bucket_id = get_bucket_id(&key);
    let r = GLOBAL_CACHE[bucket_id].read().await;
    let Some(index) = r.data.get(&key) else {
        return Ok(None);
    };
    if !index.query.is_empty() && index.query != query {
        log::warn!(
            "HASH conflict, query changed from {} to {}, skip cache",
            index.query,
            query
        );
        return Ok(None);
    }

    // get the best key
    let mut best_key = String::new();
    let mut best_diff = 0;
    for entry in index.entries.iter() {
        if start < entry.start {
            continue;
        }
        let mut d = entry.end - start;
        if end <= entry.end {
            d = end - start;
        }
        if d >= best_diff {
            best_key = entry.key.clone();
            best_diff = d;
        }
    }
    drop(r);
    if best_key.is_empty() {
        return Ok(None);
    }

    // get the data from disk cache
    let Some(data) = infra::cache::file_data::disk::get(&best_key, None).await else {
        // need to drop the key from index
        let mut w = GLOBAL_CACHE[bucket_id].write().await;
        if let Some(index) = w.data.get_mut(&key) {
            index.entries.retain(|entry| entry.key != best_key);
        }
        drop(w);
        return Ok(None);
    };
    let mut resp = match proto::cluster_rpc::MetricsQueryResponse::decode(data) {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("decode metrics query response error: {}", e);
            return Ok(None);
        }
    };
    if resp.series.is_empty() {
        return Ok(None);
    }

    let mut new_start = start;
    for series in resp.series.iter_mut() {
        // filter the samples, remove the samples over end
        let value_n = series.samples.len();
        let mut last_i = value_n;
        for i in 0..last_i {
            if series.samples[i].time > end {
                last_i = i;
                break;
            }
        }
        if last_i < value_n {
            series.samples.drain(last_i..);
        }

        // filter the exemplars, remove the exemplars over end
        if let Some(exemplars) = series.exemplars.as_mut() {
            let value_n = exemplars.exemplars.len();
            let mut last_i = value_n;
            for i in (0..last_i).rev() {
                if exemplars.exemplars[i].time < end {
                    last_i = i;
                    break;
                }
            }
            if last_i < value_n {
                exemplars.exemplars.drain(last_i..);
            }
        }

        // update the new start
        let ns = if let Some(exemplars) = series.exemplars.as_ref() {
            exemplars.exemplars.last().map(|v| v.time).unwrap_or(0)
        } else {
            series.samples.last().map(|v| v.time).unwrap_or(0)
        };
        if ns > new_start {
            new_start = ns;
        }
    }

    // if new_start == start, it means we have no data in cache, so we need to return None
    if new_start == start {
        return Ok(None);
    }

    // if new_start > start, it means we have data in cache, so we need to add step for next query
    new_start += step;
    Ok(Some((new_start, resp.series)))
}

pub async fn set(
    trace_id: &str,
    query: &str,
    start: i64,
    end: i64,
    step: i64,
    mut range_values: Vec<RangeValue>,
) -> Result<()> {
    // check time range, if over ZO_MAX_FILE_RETENTION_TIME, return
    let cfg = get_config();
    let max_ts = now_micros() - second_micros(cfg.limit.max_file_retention_time as i64);
    let new_end = if end > max_ts { max_ts } else { end };
    if range_values.is_empty() || start >= max_ts || new_end <= start + step {
        // all of the data in retention time, no need to store
        return Ok(());
    }

    // get the bucket cache
    let key = get_hash_key(query, step);
    let bucket_id = get_bucket_id(&key);
    let r = GLOBAL_CACHE[bucket_id].read().await;
    if let Some(index) = r.data.get(&key) {
        if !index.query.is_empty() && index.query != query {
            log::warn!(
                "HASH conflict, query changed from {} to {}, skip cache",
                index.query,
                query
            );
            return Ok(());
        }
        // check if the cache already converted
        if index
            .entries
            .iter()
            .any(|entry| entry.start <= start && entry.end >= new_end)
        {
            return Ok(());
        }
    }
    let need_gc = r.cacher.len() >= r.max_entries - METRICS_INDEX_CACHE_GC_TRIGGER_NUM;
    drop(r);

    if need_gc {
        if let Err(err) = gc(bucket_id).await {
            log::error!(
                "[trace_id {trace_id}] promql->search->cache: gc err: {:?}",
                err
            );
        }
    }

    // filter the samples
    if end >= max_ts {
        let mut empty_item_index = Vec::new();
        for (i, series) in range_values.iter_mut().enumerate() {
            let mut empty_data = false;
            // check samples
            let value_n = series.samples.len();
            let mut last_i = value_n;
            for i in (0..last_i).rev() {
                if series.samples[i].timestamp < max_ts {
                    last_i = i;
                    break;
                }
            }
            if last_i == value_n {
                // all of the data are over the retention time, no need to store
                empty_data = true;
            } else if last_i + 1 == value_n {
                // all of the data are not in retention time, no need to drain
            } else {
                // last_i is the last item not in retention time, so we need to drain the samples
                // after last_i
                series.samples.drain(last_i + 1..);
            }

            // check exemplars
            if let Some(exemplars) = series.exemplars.as_mut() {
                empty_data = false;
                let value_n = exemplars.len();
                let mut last_i = value_n;
                for i in (0..last_i).rev() {
                    if exemplars[i].timestamp < max_ts {
                        last_i = i;
                        break;
                    }
                }
                if last_i == value_n {
                    // all of the data are over the retention time, no need to store
                    empty_data = true;
                } else if last_i + 1 == value_n {
                    // all of the data are not in retention time, no need to drain
                } else {
                    // last_i is the last item not in retention time, so we need to drain the
                    // samples after last_i
                    exemplars.drain(last_i + 1..);
                }
            }

            if empty_data {
                empty_item_index.push(i);
            }
        }
        // remove the empty items
        if !empty_item_index.is_empty() {
            for i in empty_item_index.into_iter().rev() {
                range_values.remove(i);
            }
        }
    };

    // convert RangeValue to proto::cluster_rpc::MetricsQueryResponse then encode to vec
    let mut resp = proto::cluster_rpc::MetricsQueryResponse::default();
    super::grpc::add_value(&mut resp, Value::Matrix(range_values));
    let bytes_data = resp.encode_to_vec();

    // store the series to disk cache
    let cache_key = get_cache_item_key(&key, start, new_end);
    infra::cache::file_data::disk::set(trace_id, &cache_key, bytes_data.into())
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

    // store the cache item
    let cache_item = MetricsIndexCacheItem::new(&cache_key, start, new_end);
    let mut w = GLOBAL_CACHE[bucket_id].write().await;
    w.cacher.push_back(key.to_string());
    let index = w.data.entry(key).or_insert(MetricsIndexCache::new(query));
    if index.entries.len() >= METRICS_INDEX_CACHE_MAX_ITEMS {
        // remove the first half items
        index.entries.drain(0..METRICS_INDEX_CACHE_MAX_ITEMS / 2);
    }
    index.entries.push(Arc::new(cache_item));
    drop(w);

    Ok(())
}

/// load the cache item from the secondary storage
pub async fn load(cache_key: &str) -> Result<()> {
    let cfg = get_config();
    if !cfg.common.metrics_cache_enabled {
        return Ok(());
    }
    let Some((key, start, end)) = parse_cache_item_key(cache_key) else {
        return Ok(());
    };
    let bucket_id = get_bucket_id(&key);
    let cache_item = MetricsIndexCacheItem::new(cache_key, start, end);
    let mut w = GLOBAL_CACHE[bucket_id].write().await;
    w.cacher.push_back(key.to_string());
    let index = w.data.entry(key).or_insert(MetricsIndexCache::new(""));
    index.entries.push(Arc::new(cache_item));
    drop(w);

    Ok(())
}

async fn gc(bucket_id: usize) -> Result<()> {
    log::warn!("MetricsIndexCache is full, releasing 10% of the cache");
    let mut w = GLOBAL_CACHE[bucket_id].write().await;
    for _ in 0..(w.max_entries / METRICS_INDEX_CACHE_GC_PERCENT) {
        if let Some(key) = w.cacher.pop_front() {
            w.data.remove(&key);
        } else {
            break;
        }
    }
    drop(w);

    Ok(())
}

fn get_hash_key(query: &str, step: i64) -> String {
    config::utils::md5::hash(&format!("{}-{}", query, step))
}

fn get_cache_item_key(prefix: &str, start: i64, end: i64) -> String {
    format!(
        "metrics_results/{}/{}_{}_{}_{}.pb",
        now().format("%Y/%m/%d/%H"),
        prefix,
        start,
        end,
        CACHE_KEY_SUFFIX.fetch_add(1, Ordering::SeqCst)
    )
}

/// parse the cache item key
///
/// the key format is: metrics_results/{date}/{prefix}_{start}_{end}_{suffix}.json
fn parse_cache_item_key(key: &str) -> Option<(String, i64, i64)> {
    if !key.starts_with("metrics_results/") || !key.ends_with(".pb") {
        return None;
    }
    let item_key = key.split('/').last().unwrap_or("");
    let parts = item_key.split('_').collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }

    let prefix = parts[0];
    let Ok(start) = parts[1].parse::<i64>() else {
        return None;
    };
    let Ok(end) = parts[2].parse::<i64>() else {
        return None;
    };
    Some((prefix.to_string(), start, end))
}

fn get_bucket_id(key: &str) -> usize {
    let hash = gxhash::new().sum64(key);
    hash as usize % METRICS_INDEX_CACHE_BUCKETS
}

struct MetricsIndex {
    data: HashMap<String, MetricsIndexCache>,
    cacher: VecDeque<String>,
    max_entries: usize,
}

impl MetricsIndex {
    fn new(max_entries: usize) -> Self {
        Self {
            data: HashMap::new(),
            cacher: VecDeque::new(),
            max_entries,
        }
    }
}

struct MetricsIndexCache {
    query: String,
    entries: Vec<Arc<MetricsIndexCacheItem>>,
}

impl MetricsIndexCache {
    fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            entries: Vec::new(),
        }
    }
}

struct MetricsIndexCacheItem {
    key: String,
    start: i64,
    end: i64,
}

impl MetricsIndexCacheItem {
    fn new(key: &str, start: i64, end: i64) -> Self {
        Self {
            key: key.to_string(),
            start,
            end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::promql::{
        adjust_start_end,
        value::{Labels, Sample},
    };

    #[test]
    fn test_promql_cache_hash_key_generation() {
        let query = "test_query";
        let step = 60000000; // 60 seconds in microseconds

        let key = get_hash_key(query, step);
        assert_eq!(key, "b235015c612525ad7c11c109e3fdc261");
    }

    #[test]
    fn test_promql_cache_bucket_distribution() {
        let key1 = "test_query-60000000";
        let key2 = "test_query-60000000";

        let bucket1 = get_bucket_id(key1);
        let bucket2 = get_bucket_id(key2);

        assert!(bucket1 < METRICS_INDEX_CACHE_BUCKETS);
        assert!(bucket2 < METRICS_INDEX_CACHE_BUCKETS);
    }

    #[tokio::test]
    async fn test_promql_cache_set_and_get() {
        let trace_id = "test_trace1";
        let query = "test_query1";
        let end = now_micros();
        let start = end - second_micros(3600);
        let step = second_micros(15);
        let (start, end) = adjust_start_end(start, end, step, false);

        // Create test samples
        let mut range_values = vec![RangeValue {
            labels: Labels::new(),
            samples: vec![],
            exemplars: None,
            time_window: None,
        }];
        let max_ts = end - second_micros(get_config().limit.max_file_retention_time as i64);
        let mut valid_max_ts = 0;
        for i in 0..((end - start + step) / step) {
            let ts = start + step * i;
            if ts <= max_ts {
                valid_max_ts = ts;
            }
            range_values[0].samples.push(Sample {
                timestamp: ts,
                value: i as f64,
            });
        }

        let expected_value = range_values.first().unwrap().clone();

        // Test setting cache
        let set_result = set(trace_id, query, start, end, step, range_values).await;
        assert!(set_result.is_ok());

        // Test getting cache
        let get_result = get(query, start, end, step).await;
        assert!(get_result.is_ok());

        if let Ok(Some((new_start, cached_range_values))) = get_result {
            assert!(!cached_range_values.is_empty());
            assert_eq!(
                cached_range_values[0].samples[0].value,
                expected_value.samples[0].value
            );
            assert_eq!(new_start, valid_max_ts + step);
        } else {
            panic!("Failed to get cached values");
        }
    }

    #[tokio::test]
    async fn test_promql_cache_max_items() {
        let trace_id = "test_trace2";
        let query = "test_query2";
        let end = now_micros();
        let start = end - second_micros(3600);
        let step = second_micros(15);
        let (start, end) = adjust_start_end(start, end, step, false);

        // Add more than METRICS_INDEX_CACHE_MAX_ITEMS entries
        for i in 0..METRICS_INDEX_CACHE_MAX_ITEMS + 2 {
            let start = start + step * i as i64;
            let range_values = vec![RangeValue {
                labels: Labels::new(),
                samples: vec![Sample {
                    timestamp: start,
                    value: i as f64,
                }],
                exemplars: None,
                time_window: None,
            }];

            let set_result = set(trace_id, query, start, end, step, range_values.clone()).await;
            assert!(set_result.is_ok());
        }

        // Verify that the cache size is maintained
        let key = get_hash_key(query, step);
        let bucket_id = get_bucket_id(&key);
        let metrics = GLOBAL_CACHE[bucket_id].read().await;

        if let Some(index) = metrics.data.get(&key) {
            assert!(index.entries.len() <= METRICS_INDEX_CACHE_MAX_ITEMS);
        } else {
            panic!("Cache entry not found");
        }
    }

    #[test]
    fn test_parse_cache_item_key() {
        // Test valid key
        let key = "metrics_results/2024/01/01/00/prefix_1234_5678_suffix.pb";
        let result = parse_cache_item_key(key);
        assert!(result.is_some());
        let (prefix, start, end) = result.unwrap();
        assert_eq!(prefix, "prefix");
        assert_eq!(start, 1234);
        assert_eq!(end, 5678);

        // Test invalid keys
        let invalid_keys = vec![
            "invalid_key",                      // Too few parts
            "prefix_abc_def_suffix.pb",         // Non-numeric values
            "prefix_1234_5678",                 // Missing .pb extension
            "prefix/1234/5678/extra/suffix.pb", // Too many parts
        ];

        for invalid_key in invalid_keys {
            assert!(parse_cache_item_key(invalid_key).is_none());
        }
    }
}
