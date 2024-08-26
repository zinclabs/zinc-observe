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

use std::str::FromStr;

use async_recursion::async_recursion;
use chrono::Utc;
use config::{get_config, meta::search::Response, utils::json};
use infra::cache::{file_data::disk::QUERY_RESULT_CACHE, meta::ResultCacheMeta};

use super::cacher::get_results;
use crate::{
    common::meta::search::{CacheQueryRequest, ResultCacheSelectionStrategy},
    service::search::cache::{
        result_utils::{get_ts_value, round_down_to_nearest_minute},
        CachedQueryResponse,
    },
};

pub async fn get_cached_results(
    file_path: &str,
    trace_id: &str,
    cache_req: CacheQueryRequest,
) -> Vec<CachedQueryResponse> {
    let mut res: Vec<_> = vec![];
    let r = QUERY_RESULT_CACHE.read().await;
    let query_key = file_path.replace('/', "_");
    let is_cached = r.get(&query_key).cloned();
    drop(r);
    if is_cached.is_none() {
        log::info!(
            "[CACHE RESULT {trace_id}] No cache found for query key: {}",
            query_key
        );
        return res;
    }

    if let Some(cache_metas) = is_cached {
        let _ = recursive_process_multiple_metas(
            &cache_metas,
            trace_id,
            cache_req,
            &mut res,
            &query_key,
            file_path,
        )
        .await;
        res
    } else {
        res
    }
}

#[async_recursion]
async fn recursive_process_multiple_metas(
    cache_metas: &[ResultCacheMeta],
    trace_id: &str,
    cache_req: CacheQueryRequest,
    results: &mut Vec<CachedQueryResponse>,
    query_key: &str,
    file_path: &str,
) -> Result<(), anyhow::Error> {
    if cache_metas.is_empty() {
        log::info!(
            "[CACHE RESULT {trace_id}] No cache found for query key: {}",
            query_key
        );
        return Ok(());
    }
    let selection_strategy: ResultCacheSelectionStrategy = ResultCacheSelectionStrategy::from_str(
        &get_config().common.result_cache_selection_strategy,
    )
    .unwrap_or_default();

    // Filter relevant metas that are within the overall query range
    let relevant_metas: Vec<ResultCacheMeta> = cache_metas
        .iter()
        .filter(|m| m.start_time <= cache_req.q_end_time && m.end_time >= cache_req.q_start_time)
        .cloned()
        .collect();

    if relevant_metas.is_empty() {
        log::info!(
            "[CACHE RESULT {trace_id}] No relevant cache found for query key: {}",
            query_key
        );
        return Ok(());
    }

    // Sort by start time to process them in sequence
    let mut sorted_metas = relevant_metas;
    sorted_metas.sort_by_key(|m| m.start_time);

    if
        let Some(largest_meta) = sorted_metas
            .iter()
            .filter(|cache_meta| {
                log::info!(
                    "[CACHE CANDIDATES {trace_id}] Got caches: cache_meta.response_start_time: {}, cache_meta.response_end_time: {}",
                    cache_meta.start_time,
                    cache_meta.end_time
                );
                cache_meta.start_time <= cache_req.q_end_time &&
                    cache_meta.end_time >= cache_req.q_start_time
            })
            .max_by_key(|result| select_cache_meta(result, &cache_req, &selection_strategy))
    {
        let file_name = format!(
            "{}_{}_{}_{}.json",
            largest_meta.start_time,
            largest_meta.end_time,
            if cache_req.is_aggregate {
                1
            } else {
                0
            },
            if cache_req.is_descending {
                1
            } else {
                0
            }
        );

        let mut matching_cache_meta = largest_meta.clone();

        // Calculate delta time range to fetch the delta data using search query
        let cfg = get_config();
        let discard_duration = cfg.common.result_cache_discard_duration * 1000 * 1000;

        let cache_duration = matching_cache_meta.end_time - matching_cache_meta.start_time;
        if
            cache_duration <= discard_duration &&
            matching_cache_meta.start_time > Utc::now().timestamp_micros() - discard_duration
        {
            return Ok(());
        }

        let result = match get_results(file_path, &file_name).await {
            Ok(v) => {
                match json::from_str::<Response>(&v) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        log::error!("[trace_id {trace_id}] Error parsing cached response: {:?}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::error!("[trace_id {trace_id}] Get results from disk failed: {:?}", e);
                None
            }
        };
        if let Some(mut cached_response) = result {
            let (hits_allowed_start_time, hits_allowed_end_time) = if
                cache_req.discard_interval > 0
            {
                (
                    cache_req.q_start_time - (cache_req.q_start_time % cache_req.discard_interval),
                    cache_req.q_end_time - (cache_req.q_end_time % cache_req.discard_interval),
                )
            } else {
                (cache_req.q_start_time, cache_req.q_end_time)
            };
            let discard_ts = get_allowed_up_to(&cached_response, &cache_req, discard_duration);
            cached_response.hits.retain(|hit| {
                let hit_ts = get_ts_value(&cache_req.ts_column, hit);
                hit_ts <= hits_allowed_end_time &&
                    hit_ts >= hits_allowed_start_time &&
                    hit_ts < discard_ts
            });

            cached_response.total = cached_response.hits.len();
            if cache_req.discard_interval < 0 {
                matching_cache_meta.end_time = discard_ts;
            }

            log::info!(
                "[CACHE RESULT {trace_id}] Get results from disk success for query key: {} with start time {} - end time {}",
                query_key,
                matching_cache_meta.start_time,
                matching_cache_meta.end_time
            );
            results.push(CachedQueryResponse {
                cached_response,
                deltas: vec![],
                has_cached_data: true,
                cache_query_response: true,
                response_start_time: matching_cache_meta.start_time,
                response_end_time: matching_cache_meta.end_time,
                ts_column: cache_req.ts_column.to_string(),
                is_descending: cache_req.is_descending,
                limit: -1,
            });
        }
        // Filter out the largest meta and call recursively with non-overlapping metas
        let remaining_metas: Vec<ResultCacheMeta> = sorted_metas
            .clone()
            .into_iter()
            .filter(|meta| {
                !largest_meta.eq(meta) &&
                    (meta.end_time <= largest_meta.start_time ||
                        meta.start_time >= largest_meta.end_time)
            })
            .collect();
        if !remaining_metas.is_empty() {
            return Ok(());
        }
        let _ = recursive_process_multiple_metas(
            &remaining_metas[..],
            trace_id,
            cache_req,
            results,
            query_key,
            file_path
        ).await;
    }
    Ok(())
}

fn select_cache_meta(
    meta: &ResultCacheMeta,
    req: &CacheQueryRequest,
    strategy: &ResultCacheSelectionStrategy,
) -> i64 {
    match strategy {
        ResultCacheSelectionStrategy::Overlap => {
            let overlap_start = meta.start_time.max(req.q_start_time);
            let overlap_end = meta.end_time.min(req.q_end_time);
            overlap_end - overlap_start
        }
        ResultCacheSelectionStrategy::Duration => meta.end_time - meta.start_time,
        ResultCacheSelectionStrategy::Both => {
            let overlap_start = req.q_start_time.max(meta.start_time);
            let overlap_end = req.q_end_time.min(meta.end_time);
            let overlap_duration = overlap_end - overlap_start;
            let cache_duration = meta.end_time - meta.start_time;
            if cache_duration > 0 {
                (overlap_duration * 100) / cache_duration
            } else {
                0
            }
        }
    }
}

fn get_allowed_up_to(
    cached_response: &Response,
    cache_req: &CacheQueryRequest,
    discard_duration: i64,
) -> i64 {
    let first_ts = get_ts_value(&cache_req.ts_column, cached_response.hits.first().unwrap());
    let last_ts = get_ts_value(&cache_req.ts_column, cached_response.hits.last().unwrap());

    if cache_req.is_descending {
        if cache_req.discard_interval > 0 {
            first_ts
        } else {
            let m_first_ts = round_down_to_nearest_minute(first_ts);
            if Utc::now().timestamp_micros() - discard_duration < m_first_ts {
                m_first_ts - discard_duration
            } else {
                first_ts
            }
        }
    } else if cache_req.discard_interval > 0 {
        last_ts
    } else {
        let m_last_ts = round_down_to_nearest_minute(last_ts);
        if Utc::now().timestamp_micros() - discard_duration < last_ts {
            m_last_ts - discard_duration
        } else {
            last_ts
        }
    }
}
