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

use bytes::Bytes;
use chrono::Utc;
use config::{
    get_config,
    meta::search::Response,
    utils::{file::scan_files, json},
};
use infra::cache::{
    file_data::disk::{self, QUERY_RESULT_CACHE},
    meta::ResultCacheMeta,
};

use crate::{
    common::meta::search::{CachedQueryResponse, QueryDelta},
    service::search::{
        cache::result_utils::{get_ts_value, round_down_to_nearest_minute},
        sql::{generate_histogram_interval, SqlMode, RE_HISTOGRAM, RE_SELECT_FROM},
    },
};

#[allow(clippy::too_many_arguments)]
pub async fn check_cache(
    rpc_req: &proto::cluster_rpc::SearchRequest,
    req: &mut config::meta::search::Request,
    origin_sql: &mut String,
    parsed_sql: &config::meta::sql::Sql,
    file_path: &mut String,
    is_aggregate: bool,
    should_exec_query: &mut bool,
    trace_id: &str,
) -> CachedQueryResponse {
    let start = std::time::Instant::now();
    let cfg = get_config();
    // check sql_mode

    let meta: super::super::sql::Sql = match super::super::sql::Sql::new(rpc_req).await {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error parsing sql: {:?}", e);
            return CachedQueryResponse::default();
        }
    };
    let sql_mode: SqlMode = meta.sql_mode;

    // skip the count queries
    if sql_mode.eq(&SqlMode::Full) && req.query.track_total_hits {
        return CachedQueryResponse::default();
    }
    let mut result_ts_col = get_ts_col(parsed_sql, &cfg.common.column_timestamp, is_aggregate);
    if is_aggregate && sql_mode.eq(&SqlMode::Full) && result_ts_col.is_none() {
        return CachedQueryResponse::default();
    }

    // Hack select for _timestamp
    if !is_aggregate && parsed_sql.order_by.is_empty() && !origin_sql.contains('*') {
        let caps = RE_SELECT_FROM.captures(origin_sql.as_str()).unwrap();
        let cap_str = caps.get(1).unwrap().as_str();
        if !cap_str.contains(&cfg.common.column_timestamp) {
            *origin_sql = origin_sql.replace(
                cap_str,
                &format!("{}, {}", &cfg.common.column_timestamp, cap_str),
            );
        }
        req.query.sql = origin_sql.clone();
        result_ts_col = Some(cfg.common.column_timestamp.clone());
    }
    if !is_aggregate && origin_sql.contains('*') {
        result_ts_col = Some(cfg.common.column_timestamp.clone());
    }

    let result_ts_col = result_ts_col.unwrap();
    let mut discard_interval = -1;
    if let Some(interval) = meta.histogram_interval {
        *file_path = format!("{}_{}_{}", file_path, interval, result_ts_col);

        let mut req_time_range = (req.query.start_time, req.query.end_time);
        if req_time_range.1 == 0 {
            req_time_range.1 = chrono::Utc::now().timestamp_micros();
        }

        let meta_time_range_is_empty =
            parsed_sql.time_range.is_none() || parsed_sql.time_range == Some((0, 0));
        let q_time_range =
            if meta_time_range_is_empty && (req_time_range.0 > 0 || req_time_range.1 > 0) {
                Some(req_time_range)
            } else {
                parsed_sql.time_range
            };
        handle_historgram(origin_sql, q_time_range);
        req.query.sql = origin_sql.clone();
        discard_interval = interval * 1000 * 1000; //in microseconds
    };
    if req.query.size >= 0 {
        *file_path = format!("{}_{}_{}", file_path, req.query.from, req.query.size);
    }
    let query_key = file_path.replace('/', "_");

    let mut c_resp = match crate::service::search::cluster::cacher::get_cached_results(
        req.query.start_time,
        req.query.end_time,
        is_aggregate,
        query_key.to_owned(),
        file_path.to_string(),
        trace_id.to_owned(),
        result_ts_col.clone(),
        discard_interval,
    )
    .await
    {
        Some(mut cached_resp) => {
            let search_delta: Vec<QueryDelta> = cached_resp
                .deltas
                .iter()
                .filter(|d| !d.delta_removed_hits)
                .cloned()
                .collect();
            if search_delta.is_empty() {
                log::debug!("cached response found");
                *should_exec_query = false;
            };
            if cached_resp.cached_response.total == (meta.meta.limit as usize) {
                *should_exec_query = false;
                cached_resp.deltas = vec![];
            } else {
                cached_resp.deltas = search_delta;
            }

            cached_resp.cached_response.took = start.elapsed().as_millis() as usize;
            cached_resp
        }
        None => {
            log::debug!("cached response not found");
            CachedQueryResponse::default()
        }
    };
    c_resp.cache_query_response = true;
    c_resp.ts_column = result_ts_col;
    c_resp
}

#[allow(clippy::too_many_arguments)]
pub async fn get_cached_results(
    start_time: i64,
    end_time: i64,
    is_aggregate: bool,
    file_path: &str,
    result_ts_column: &str,
    trace_id: &str,
    discard_interval: i64,
) -> Option<CachedQueryResponse> {
    let r = QUERY_RESULT_CACHE.read().await;
    let query_key = file_path.replace('/', "_");
    let is_cached = r.get(&query_key).cloned();
    drop(r);

    if let Some(cache_metas) = is_cached {
        match cache_metas
            .iter()
            .filter(|cache_meta| {
                // to make sure there is overlap between cache time range and query time range

                cache_meta.start_time <= end_time && cache_meta.end_time >= start_time
            })
            .max_by_key(|result| result.end_time.min(end_time) - result.start_time.max(start_time))
        {
            Some(matching_meta) => {
                let file_name = format!(
                    "{}_{}_{}.json",
                    matching_meta.start_time,
                    matching_meta.end_time,
                    if is_aggregate { 1 } else { 0 }
                );
                let mut matching_cache_meta = matching_meta.clone();
                // calculate delta time range to fetch the delta data using search query
                let cfg = get_config();
                let discard_duration = cfg.common.result_cache_discard_duration * 1000 * 1000;

                let cache_duration = matching_cache_meta.end_time - matching_cache_meta.start_time;
                // return None if cache duration is less than 2 * discard_duration
                if cache_duration <= discard_duration {
                    return None;
                }

                let mut deltas = vec![];

                calculate_deltas_v1(&matching_cache_meta, start_time, end_time, &mut deltas);

                let remove_hits: Vec<&QueryDelta> =
                    deltas.iter().filter(|d| d.delta_removed_hits).collect();

                match get_results(file_path, &file_name).await {
                    Ok(v) => {
                        let mut cached_response: Response = json::from_str::<Response>(&v).unwrap();
                        // remove hits if time range is lesser than cached time range
                        let mut to_retain = Vec::new();

                        let first_ts =
                            get_ts_value(result_ts_column, cached_response.hits.first().unwrap());

                        let last_ts =
                            get_ts_value(result_ts_column, cached_response.hits.last().unwrap());

                        let discard_ts = if discard_interval > 0 {
                            // for histogram
                            if first_ts < last_ts {
                                last_ts
                            } else {
                                first_ts
                            }
                        } else if first_ts < last_ts {
                            // non-aggregate quer
                            let m_last_ts = round_down_to_nearest_minute(last_ts);
                            if Utc::now().timestamp_micros() - discard_duration < last_ts {
                                m_last_ts - discard_duration
                            } else {
                                matching_cache_meta.end_time
                            }
                        } else {
                            let m_first_ts = round_down_to_nearest_minute(first_ts);
                            if Utc::now().timestamp_micros() - discard_duration < m_first_ts {
                                m_first_ts - discard_duration
                            } else {
                                matching_cache_meta.start_time
                            }
                        };

                        if !remove_hits.is_empty() {
                            for delta in remove_hits {
                                for hit in &cached_response.hits {
                                    let hit_ts = get_ts_value(result_ts_column, hit);

                                    if !(hit_ts >= delta.delta_start_time
                                        && hit_ts < delta.delta_end_time)
                                        && (hit_ts <= end_time && hit_ts >= start_time)
                                        && hit_ts < discard_ts
                                    {
                                        to_retain.push(hit.clone());
                                    }
                                }
                            }
                            cached_response.hits = to_retain;
                            cached_response.total = cached_response.hits.len();
                            if discard_interval < 0 {
                                matching_cache_meta.end_time = discard_ts;
                            };
                        };

                        // recalculate deltas
                        let mut deltas = vec![];

                        let has_pre_cache_delta = calculate_deltas_v1(
                            &matching_cache_meta,
                            start_time,
                            end_time,
                            &mut deltas,
                        );

                        log::info!(
                            "[trace_id {trace_id}] Get results from disk success for query key: {} with start time {} - end time {} ",
                            query_key,
                            matching_cache_meta.start_time,
                            matching_cache_meta.end_time
                        );
                        Some(CachedQueryResponse {
                            cached_response,
                            deltas,
                            has_pre_cache_delta,
                            has_cached_data: true,
                            cache_query_response: true,
                            response_start_time: matching_cache_meta.start_time,
                            response_end_time: matching_cache_meta.end_time,
                            ts_column: result_ts_column.to_string(),
                        })
                    }
                    Err(e) => {
                        log::error!(
                            "[trace_id {trace_id}] Get results from disk failed : {:?}",
                            e
                        );
                        None
                    }
                }
            }
            None => {
                log::debug!("No matching cache found for query key: {}", query_key);
                None
            }
        }
    } else {
        None
    }
}

pub fn calculate_deltas_v1(
    result_meta: &ResultCacheMeta,
    start_time: i64,
    end_time: i64,
    deltas: &mut Vec<QueryDelta>,
) -> bool {
    let mut has_pre_cache_delta = false;
    if start_time == result_meta.start_time && end_time == result_meta.end_time {
        // If query start time and end time are the same as cache times, return results from cache
        return has_pre_cache_delta;
    }

    // Query Start time < ResultCacheMeta start time & Query End time > ResultCacheMeta End time
    if end_time != result_meta.end_time {
        if end_time > result_meta.end_time {
            // q end time : 11:00, r end time : 10:45
            deltas.push(QueryDelta {
                delta_start_time: result_meta.end_time,
                delta_end_time: end_time,
                delta_removed_hits: false,
            });
        } else {
            deltas.push(QueryDelta {
                delta_start_time: end_time,
                delta_end_time: result_meta.end_time,
                delta_removed_hits: true,
            });
        }
    }
    // Query Start time > ResultCacheMeta start time & Query End time > ResultCacheMeta End time ->
    // typical last x min/hours/days of data
    if start_time != result_meta.start_time {
        if start_time > result_meta.start_time {
            // q start time : 10:00,  r start time : 09:00
            // Fetch data between ResultCacheMeta Start time & Query start time
            deltas.push(QueryDelta {
                delta_start_time: result_meta.start_time,
                delta_end_time: start_time,
                delta_removed_hits: true,
            });
        } else {
            deltas.push(QueryDelta {
                delta_start_time: start_time,
                delta_end_time: result_meta.start_time,
                delta_removed_hits: false,
            });
            has_pre_cache_delta = true;
        }
    };
    has_pre_cache_delta
}

pub async fn cache_results_to_disk(
    trace_id: &str,
    file_path: &str,
    file_name: &str,
    data: String,
) -> std::io::Result<()> {
    let file = format!("results/{}/{}", file_path, file_name);
    match disk::set(trace_id, &file, Bytes::from(data)).await {
        Ok(_) => (),
        Err(e) => {
            log::error!("Error caching results to disk: {:?}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Error caching results to disk",
            ));
        }
    }
    Ok(())
}

pub async fn get_results(file_path: &str, file_name: &str) -> std::io::Result<String> {
    let file = format!("results/{}/{}", file_path, file_name);
    match disk::get(&file, None).await {
        Some(v) => Ok(String::from_utf8(v.to_vec()).unwrap()),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )),
    }
}

fn get_ts_col(
    parsed_sql: &config::meta::sql::Sql,
    ts_col: &str,
    is_aggregate: bool,
) -> Option<String> {
    for (original, alias) in &parsed_sql.field_alias {
        if original.contains("histogram") {
            return Some(alias.clone());
        }
    }
    if !is_aggregate
        && (parsed_sql.fields.contains(&ts_col.to_owned())
            || parsed_sql.order_by.iter().any(|v| v.0.eq(&ts_col)))
    {
        return Some(ts_col.to_string());
    }

    None
}

#[tracing::instrument]
pub async fn delete_cache(path: &str) -> std::io::Result<bool> {
    let root_dir = disk::get_dir().await;
    let pattern = format!("{}/results/{}", root_dir, path);
    let prefix = format!("{}/", root_dir);
    let files = scan_files(&pattern, "json", None).unwrap_or_default();
    let mut remove_files: Vec<String> = vec![];
    for file in files {
        match disk::remove("", file.strip_prefix(&prefix).unwrap()).await {
            Ok(_) => remove_files.push(file),
            Err(e) => {
                log::error!("Error deleting cache: {:?}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Error deleting cache",
                ));
            }
        }
    }
    for file in remove_files {
        let columns = file
            .strip_prefix(&prefix)
            .unwrap()
            .split('/')
            .collect::<Vec<&str>>();

        let query_key = format!(
            "{}_{}_{}_{}",
            columns[1], columns[2], columns[3], columns[4]
        );
        let mut r = QUERY_RESULT_CACHE.write().await;
        r.remove(&query_key);
    }
    Ok(true)
}

fn handle_historgram(origin_sql: &mut String, q_time_range: Option<(i64, i64)>) {
    let caps = RE_HISTOGRAM.captures(origin_sql.as_str()).unwrap();
    let attrs = caps
        .get(1)
        .unwrap()
        .as_str()
        .split(',')
        .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
        .collect::<Vec<&str>>();

    let interval = match attrs.get(1) {
        Some(v) => match v.parse::<u16>() {
            Ok(v) => generate_histogram_interval(q_time_range, v),
            Err(_) => v.to_string(),
        },
        None => generate_histogram_interval(q_time_range, 0),
    };

    *origin_sql = origin_sql.replace(
        caps.get(0).unwrap().as_str(),
        &format!("histogram(_timestamp,'{}')", interval),
    );
}
