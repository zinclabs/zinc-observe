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

use chrono::Utc;
use config::{
    get_config,
    meta::{
        search,
        stream::StreamType,
        usage::{RequestStats, UsageType},
    },
    metrics,
    utils::{base64, hash::Sum64, json, sql::is_aggregate_query},
};
use infra::{
    cache::{file_data::disk::QUERY_RESULT_CACHE, meta::ResultCacheMeta},
    errors::Error,
};
use result_utils::get_ts_value;
use tracing::Instrument;

use crate::{
    common::{
        meta::search::{CachedQueryResponse, MultiCachedQueryResponse, QueryDelta},
        utils::functions,
    },
    service::{
        search::{self as SearchService, cache::cacher::check_cache},
        usage::{http_report_metrics, report_request_usage_stats},
    },
};

pub mod cacher;
pub mod multi;
pub mod result_utils;

#[tracing::instrument(name = "service:search:cacher:search", skip_all)]
pub async fn search(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    user_id: Option<String>,
    in_req: &search::Request,
    use_cache: bool,
) -> Result<search::Response, Error> {
    let started_at = Utc::now().timestamp_micros();
    let start = std::time::Instant::now();
    let cfg = get_config();

    // Result caching check start
    let mut origin_sql = in_req.query.sql.clone();
    origin_sql = origin_sql.replace('\n', " ");
    let is_aggregate = is_aggregate_query(&origin_sql).unwrap_or_default();
    let parsed_sql = match config::meta::sql::Sql::new(&origin_sql) {
        Ok(v) => v,
        Err(e) => {
            return Err(Error::Message(e.to_string()));
        }
    };

    let stream_name = &parsed_sql.source;

    let mut req = in_req.clone();
    let mut query_fn = req
        .query
        .query_fn
        .as_ref()
        .and_then(|v| base64::decode_url(v).ok());

    let mut h = config::utils::hash::gxhash::new();
    let hashed_query = if let Some(vrl_function) = &query_fn {
        h.sum64(&format!("{}{}", origin_sql, vrl_function))
    } else {
        h.sum64(&origin_sql)
    };

    let mut should_exec_query = true;
    let mut ext_took_wait = 0;

    let mut rpc_req: proto::cluster_rpc::SearchRequest = req.to_owned().into();
    rpc_req.org_id = org_id.to_string();
    rpc_req.stream_type = stream_type.to_string();

    let mut file_path = format!(
        "{}/{}/{}/{}",
        org_id, stream_type, stream_name, hashed_query
    );
    let mut c_resp: MultiCachedQueryResponse = if use_cache {
        check_cache(
            trace_id,
            &rpc_req,
            &mut req,
            &mut origin_sql,
            &mut file_path,
            is_aggregate,
            &mut should_exec_query,
        )
        .await
    } else {
        MultiCachedQueryResponse::default()
    };

    // No cache data present, add delta for full query
    if !c_resp.has_cached_data {
        c_resp.deltas.push(QueryDelta {
            delta_start_time: req.query.start_time,
            delta_end_time: req.query.end_time,
            delta_removed_hits: false,
        })
    } else if use_cache {
        log::info!(
            "[trace_id {trace_id}] Query deltas are: {:?}",
            c_resp.deltas
        );
        log::info!(
            "[trace_id {trace_id}] Query original start time: {}, end time : {}",
            req.query.start_time,
            req.query.end_time
        );
    }

    // Result caching check ends, start search
    let mut results = Vec::new();
    let mut res = if !should_exec_query {
        merge_response(
            trace_id,
            &mut c_resp.cached_response,
            &mut vec![],
            &c_resp.ts_column,
            c_resp.limit,
            c_resp.is_descending,
            c_resp.took,
        )
    } else {
        if let Some(vrl_function) = &query_fn {
            if !vrl_function.trim().ends_with('.') {
                query_fn = Some(format!("{} \n .", vrl_function));
            }
        }
        req.query.query_fn = query_fn;

        for fn_name in functions::get_all_transform_keys(org_id).await {
            if req.query.sql.contains(&format!("{}(", fn_name)) {
                req.query.uses_zo_fn = true;
                break;
            }
        }

        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[org_id])
            .inc();

        // get a local search queue lock
        #[cfg(not(feature = "enterprise"))]
        let locker = SearchService::QUEUE_LOCKER.clone();
        #[cfg(not(feature = "enterprise"))]
        let locker = locker.lock().await;
        #[cfg(not(feature = "enterprise"))]
        if !cfg.common.feature_query_queue_enabled {
            drop(locker);
        }
        #[cfg(not(feature = "enterprise"))]
        let took_wait = start.elapsed().as_millis() as usize;
        #[cfg(feature = "enterprise")]
        let took_wait = 0;
        ext_took_wait = took_wait;
        log::info!(
            "[trace_id {trace_id}] http search API wait in queue took: {} ms",
            took_wait
        );

        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[org_id])
            .dec();

        let mut tasks = Vec::new();

        let delta_len = c_resp.deltas.len();
        for (i, delta) in c_resp.deltas.into_iter().enumerate() {
            let mut req = req.clone();
            let org_id = org_id.to_string();
            let trace_id = if delta_len <= 1 {
                trace_id.to_string()
            } else {
                format!("{}-{}", trace_id, i)
            };
            let user_id = user_id.clone();

            let enter_span = tracing::span::Span::current();
            let task = tokio::task::spawn(async move {
                let trace_id = trace_id.clone();
                req.query.start_time = delta.delta_start_time;
                req.query.end_time = delta.delta_end_time;

                let cfg = get_config();
                if cfg.common.result_cache_enabled
                    && cfg.common.print_key_sql
                    && c_resp.has_cached_data
                {
                    log::info!(
                        "[trace_id {trace_id}] Query new start time: {}, end time : {}",
                        req.query.start_time,
                        req.query.end_time
                    );
                }

                SearchService::search(&trace_id, &org_id, stream_type, user_id, &req)
                    .instrument(enter_span)
                    .await
            });
            tasks.push(task);
        }

        for task in tasks {
            results.push(task.await.map_err(|e| Error::Message(e.to_string()))??);
        }
        if c_resp.has_cached_data {
            merge_response(
                trace_id,
                &mut c_resp.cached_response,
                &mut results,
                &c_resp.ts_column,
                c_resp.limit,
                c_resp.is_descending,
                c_resp.took,
            )
        } else {
            let mut reps = results[0].clone();
            sort_response(c_resp.is_descending, &mut reps, &c_resp.ts_column);
            reps
        }
    };

    // do search
    let time = start.elapsed().as_secs_f64();
    http_report_metrics(start, org_id, stream_type, "", "200", "_search");
    res.set_trace_id(trace_id.to_string());
    res.set_local_took(start.elapsed().as_millis() as usize, ext_took_wait);

    let req_stats = RequestStats {
        records: res.hits.len() as i64,
        response_time: time,
        size: res.scan_size as f64,
        request_body: Some(req.query.sql),
        user_email: user_id,
        min_ts: Some(req.query.start_time),
        max_ts: Some(req.query.end_time),
        cached_ratio: Some(res.cached_ratio),
        search_type: req.search_type,
        trace_id: Some(trace_id.to_string()),
        took_wait_in_queue: if res.took_detail.is_some() {
            let resp_took = res.took_detail.as_ref().unwrap();
            // Consider only the cluster wait queue duration
            Some(resp_took.cluster_wait_queue)
        } else {
            None
        },
        result_cache_ratio: Some(res.result_cache_ratio),
        ..Default::default()
    };
    let num_fn = req.query.query_fn.is_some() as u16;
    report_request_usage_stats(
        req_stats,
        org_id,
        stream_name,
        StreamType::Logs,
        UsageType::Search,
        num_fn,
        started_at,
    )
    .await;

    // result cache save changes start
    if cfg.common.result_cache_enabled
        && should_exec_query
        && c_resp.cache_query_response
        && (results.first().is_some_and(|res| !res.hits.is_empty())
            || results.last().is_some_and(|res| !res.hits.is_empty()))
    {
        write_results(
            trace_id,
            &c_resp.ts_column,
            req.query.start_time,
            req.query.end_time,
            &res,
            file_path,
            is_aggregate,
            c_resp.is_descending,
        )
        .await;
    }
    // result cache save changes Ends

    Ok(res)
}

// based on _timestamp of first record in config::meta::search::Response either add it in start
// or end to cache response
fn merge_response(
    trace_id: &str,
    cache_responses: &mut Vec<config::meta::search::Response>,
    search_response: &mut Vec<config::meta::search::Response>,
    ts_column: &str,
    limit: i64,
    is_descending: bool,
    cache_took: usize,
) -> config::meta::search::Response {
    cache_responses.retain(|res| !res.hits.is_empty());

    search_response.retain(|res| !res.hits.is_empty());

    if cache_responses.is_empty() && search_response.is_empty() {
        return config::meta::search::Response::default();
    }

    let mut cache_response = if cache_responses.is_empty() {
        config::meta::search::Response::default()
    } else {
        let mut resp = config::meta::search::Response::default();
        for res in cache_responses {
            resp.total += res.total;
            resp.scan_size += res.scan_size;

            resp.scan_records += res.scan_records;

            if res.hits.is_empty() {
                continue;
            }
            resp.hits.extend(res.hits.clone());
        }
        resp.took = cache_took;
        resp
    };

    if cache_response.hits.is_empty()
        && search_response.is_empty()
        && search_response.first().unwrap().hits.is_empty()
        && search_response.last().unwrap().hits.is_empty()
    {
        for res in search_response {
            cache_response.total += res.total;
            cache_response.scan_size += res.scan_size;
            cache_response.took += res.took;
        }
        return cache_response;
    }
    let cache_hits_len = cache_response.hits.len();

    cache_response.scan_size = 0;

    let mut files_cache_ratio = 0;
    let mut result_cache_len = 0;

    for res in search_response.clone() {
        cache_response.total += res.total;
        cache_response.scan_size += res.scan_size;
        cache_response.took += res.took;
        files_cache_ratio += res.cached_ratio;
        cache_response.histogram_interval = res.histogram_interval;

        result_cache_len += res.total;

        if res.hits.is_empty() {
            continue;
        }
        cache_response.hits.extend(res.hits.clone());
    }
    sort_response(is_descending, &mut cache_response, ts_column);

    if cache_response.hits.len() > limit as usize {
        cache_response.hits.truncate(limit as usize);
    }
    if !search_response.is_empty() {
        cache_response.cached_ratio = files_cache_ratio / search_response.len();
    }
    cache_response.size = cache_response.hits.len() as i64;
    log::info!(
        "[trace_id {trace_id}] cache_response.hits.len: {}, Result cache len: {}",
        cache_hits_len,
        result_cache_len,
    );
    cache_response.result_cache_ratio =
        (cache_hits_len as f64 * 100_f64 / (result_cache_len + cache_hits_len) as f64) as usize;
    cache_response
}

fn sort_response(is_descending: bool, cache_response: &mut search::Response, ts_column: &str) {
    if is_descending {
        cache_response
            .hits
            .sort_by_key(|b| std::cmp::Reverse(get_ts_value(ts_column, b)));
    } else {
        cache_response
            .hits
            .sort_by_key(|a| get_ts_value(ts_column, a));
    }
}

#[allow(clippy::too_many_arguments)]
async fn write_results(
    trace_id: &str,
    ts_column: &str,
    req_query_start_time: i64,
    req_query_end_time: i64,
    res: &config::meta::search::Response,
    file_path: String,
    is_aggregate: bool,
    is_descending: bool,
) {
    let last_rec_ts = get_ts_value(ts_column, res.hits.last().unwrap());
    let first_rec_ts = get_ts_value(ts_column, res.hits.first().unwrap());

    let smallest_ts = std::cmp::max(first_rec_ts, last_rec_ts);
    let discard_duration = get_config().common.result_cache_discard_duration * 1000 * 1000;

    if (last_rec_ts - first_rec_ts).abs() < discard_duration
        && smallest_ts > Utc::now().timestamp_micros() - discard_duration
    {
        return;
    }

    let largest_ts = std::cmp::max(first_rec_ts, last_rec_ts);

    let cache_end_time = if largest_ts > 0 && largest_ts < req_query_end_time {
        largest_ts
    } else {
        req_query_end_time
    };
    let file_name = format!(
        "{}_{}_{}_{}.json",
        req_query_start_time,
        cache_end_time,
        if is_aggregate { 1 } else { 0 },
        if is_descending { 1 } else { 0 }
    );

    let res_cache = json::to_string(&res).unwrap();
    let query_key = file_path.replace('/', "_");
    let trace_id = trace_id.to_string();
    tokio::spawn(async move {
        let file_path_local = file_path.clone();

        match SearchService::cache::cacher::cache_results_to_disk(
            &trace_id,
            &file_path_local,
            &file_name,
            res_cache,
        )
        .await
        {
            Ok(_) => {
                let mut w = QUERY_RESULT_CACHE.write().await;
                w.entry(query_key)
                    .or_insert_with(Vec::new)
                    .push(ResultCacheMeta {
                        start_time: req_query_start_time,
                        end_time: cache_end_time,
                        is_aggregate,
                        is_descending,
                    });
                drop(w);
            }
            Err(e) => {
                log::error!("Cache results to disk failed: {:?}", e);
            }
        }
    });
}
