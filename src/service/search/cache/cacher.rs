use bytes::Bytes;
use config::{get_config, meta::search::Response, utils::json};
use infra::cache::{
    file_data::disk::{self, QUERY_RESULT_CACHE},
    meta::ResultCacheMeta,
};

use crate::{
    common::meta::search::{CachedQueryResponse, QueryDelta},
    service::search::sql::{SqlMode, RE_SELECT_FROM, TS_WITH_ALIAS},
};

#[allow(clippy::too_many_arguments)]
pub async fn check_cache(
    rpc_req: &mut proto::cluster_rpc::SearchRequest,
    req: &mut config::meta::search::Request,
    origin_sql: &mut String,
    parsed_sql: &config::meta::sql::Sql,
    query_key: &str,
    file_path: &str,
    is_aggregate: bool,
    should_exec_query: &mut bool,
    trace_id: &str,
) -> CachedQueryResponse {
    let start = std::time::Instant::now();
    let cfg = get_config();

    // check sql_mode
    let sql_mode: SqlMode = rpc_req.query.as_ref().unwrap().sql_mode.as_str().into();

    // skip the count queries
    if sql_mode.eq(&SqlMode::Full) && req.query.track_total_hits {
        return CachedQueryResponse::default();
    }
    if is_aggregate && sql_mode.eq(&SqlMode::Full) && !should_cache_query(parsed_sql, &cfg.common.column_timestamp) {        
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
        rpc_req.query.as_mut().unwrap().sql = origin_sql.clone();
    }

    let mut c_resp = match crate::service::search::cluster::cacher::get_cached_results(
        req.query.start_time,
        req.query.end_time,
        is_aggregate,
        query_key.to_owned(),
        file_path.to_owned(),
        trace_id.to_owned(),
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
                log::info!("cached response found");
                *should_exec_query = false;
            };
            cached_resp.deltas = search_delta;
            cached_resp.cached_response.took = start.elapsed().as_millis() as usize;
            cached_resp
        }
        None => {
            log::debug!("cached response not found");
            CachedQueryResponse::default()
        }
    };
    c_resp.cache_query_response = true;
    c_resp
}

pub async fn get_cached_results(
    start_time: i64,
    end_time: i64,
    is_aggregate: bool,
    query_key: &str,
    file_path: &str,
) -> Option<CachedQueryResponse> {
    let cfg = get_config();
    let r = QUERY_RESULT_CACHE.read().await;
    let is_cached = r.get(query_key).cloned();
    drop(r);

    if let Some(cache_metas) = is_cached {
        match cache_metas
            .iter()
            .filter(|cache_meta| {
                // to make sure there is overlap between cache time range and query time range &
                // cache can at least serve query_cache_min_contribution

                let cached_duration = cache_meta.end_time - cache_meta.start_time;
                let query_duration = end_time - start_time;

                cached_duration > query_duration / cfg.limit.query_cache_min_contribution
                    && cache_meta.start_time <= end_time
                    && cache_meta.end_time >= start_time
            })
            .max_by_key(|result| result.end_time.min(end_time) - result.start_time.max(start_time))
        {
            Some(matching_cache_meta) => {
                // calculate delta time range to fetch the delta data using search query

                let mut deltas = vec![];
                let has_pre_cache_delta =
                    calculate_deltas_v1(matching_cache_meta, start_time, end_time, &mut deltas);

                let remove_hits: Vec<&QueryDelta> =
                    deltas.iter().filter(|d| d.delta_removed_hits).collect();

                let file_name = format!(
                    "{}_{}_{}.json",
                    matching_cache_meta.start_time,
                    matching_cache_meta.end_time,
                    if is_aggregate { 1 } else { 0 }
                );
                match get_results(file_path, &file_name).await {
                    Ok(v) => {
                        let mut cached_response: Response = json::from_str::<Response>(&v).unwrap();
                        // remove hits if time range is lesser than cached time range

                        if !remove_hits.is_empty() {
                            for delta in remove_hits {
                                cached_response.hits.retain(|hit| {
                                    let hit_ts = hit
                                        .get(&cfg.common.column_timestamp)
                                        .unwrap()
                                        .as_i64()
                                        .unwrap();

                                    !(hit_ts >= delta.delta_start_time
                                        && hit_ts < delta.delta_end_time)
                                });
                            }
                        };

                        Some(CachedQueryResponse {
                            cached_response,
                            deltas,
                            has_pre_cache_delta,
                            has_cached_data: true,
                            cache_query_response: true,
                            response_start_time: matching_cache_meta.start_time,
                            response_end_time: matching_cache_meta.end_time,
                        })
                    }
                    Err(e) => {
                        log::error!("Get results from disk failed : {:?}", e);
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

fn should_cache_query(parsed_sql: &config::meta::sql::Sql, ts_col: &String) -> bool {
    let non_timestamp_aliases: Vec<_> = parsed_sql
        .field_alias
        .iter()
        .filter(|(original, alias)| TS_WITH_ALIAS.is_match(original) && !alias.eq(ts_col))
        .collect();

    // Check if 'timestamp' is directly mentioned in the fields and not aliased to another name
    parsed_sql.fields.iter().any(|field| {
        field.eq(ts_col) 
    }) && non_timestamp_aliases.is_empty() ||
    // Check for any aliases that might represent a timestamp column
    parsed_sql.field_alias.iter().any(|(_, alias)| alias.eq(ts_col))
}
