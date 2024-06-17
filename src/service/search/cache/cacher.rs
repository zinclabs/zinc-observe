use bytes::Bytes;
use chrono::TimeZone;
use config::{get_config, meta::search::Response, utils::json};
use infra::cache::{
    file_data::disk::{self, QUERY_RESULT_CACHE},
    meta::ResultCacheMeta,
};

use crate::common::meta::search::{CachedQueryResponse, QueryDelta};

pub async fn get_cached_results(
    start_time: i64,
    end_time: i64,
    is_aggregate: bool,
    query_key: String,
    file_path: String,
) -> Option<CachedQueryResponse> {
    let cfg = get_config();
    let r = QUERY_RESULT_CACHE.read().await;
    let is_cached = r.get(&query_key).cloned(); //org_streamy_type_stream_name_query -> 
    drop(r);

    let st = chrono::Utc.timestamp_micros(start_time).unwrap();

    let end = chrono::Utc.timestamp_micros(end_time).unwrap();

    log::info!("#################################################");

    log::info!("Query start time & end time is {} - {}", st, end);

    if let Some(cache_metas) = is_cached {
        match cache_metas
            .iter()
            .filter(|cache_meta| {
                // to make sure there is overlap between cache time range and query time range &
                // cache can at least serve query_cache_min_contribution

                let cached_duration = cache_meta.end_time - cache_meta.start_time;
                let query_duration = end_time - start_time;

                let st = chrono::Utc.timestamp_micros(cache_meta.start_time).unwrap();

                let end = chrono::Utc.timestamp_micros(cache_meta.end_time).unwrap();

                log::info!(
                    "########## Cached start time & end time is {} - {}",
                    st,
                    end
                );
                cached_duration > query_duration / cfg.limit.query_cache_min_contribution
                    && cache_meta.start_time <= end_time
                    && cache_meta.end_time >= start_time
            })
            .max_by_key(|result| result.end_time.min(end_time) - result.start_time.max(start_time))
        {
            Some(matching_cache_meta) => {
                // calculate delta time range to fetch the delta data using search query
                log::info!("#################################################");
                let st = chrono::Utc
                    .timestamp_micros(matching_cache_meta.start_time)
                    .unwrap();

                let end = chrono::Utc
                    .timestamp_micros(matching_cache_meta.end_time)
                    .unwrap();

                log::info!(
                    "########## Used start time & end time is {} - {} ##########",
                    st,
                    end
                );

                let mut deltas = vec![];
                let has_pre_cache_delta =
                    calculate_deltas_v1(&matching_cache_meta, start_time, end_time, &mut deltas);

                let remove_hits: Vec<&QueryDelta> =
                    deltas.iter().filter(|d| d.delta_removed_hits).collect();

                let file_name = format!(
                    "{}_{}_{}.json",
                    matching_cache_meta.start_time,
                    matching_cache_meta.end_time,
                    if is_aggregate { 1 } else { 0 }
                );
                match get_results(&file_path, &file_name).await {
                    Ok(v) => {
                        let mut cached_response: Response = json::from_str::<Response>(&v).unwrap();
                        // remove hits if time range is lesser than cached time range

                        if !remove_hits.is_empty() {
                            for delta in remove_hits {
                                cached_response.hits = cached_response
                                    .hits
                                    .into_iter()
                                    .filter(|hit| {
                                        let hit_ts = hit
                                            .get(&cfg.common.column_timestamp)
                                            .unwrap()
                                            .as_i64()
                                            .unwrap();

                                        !(hit_ts >= delta.delta_start_time
                                            && hit_ts < delta.delta_end_time)
                                    })
                                    .collect();
                            }
                        };

                        return Some(CachedQueryResponse {
                            cached_response,
                            deltas,
                            has_pre_cache_delta,
                        });
                    }
                    Err(e) => {
                        log::error!("Get results from disk failed for : {:?}", e);
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

// pub fn calculate_deltas(
//     result_meta: &ResultCacheMeta,
//     start_time: i64,
//     end_time: i64,
//     mut deltas: Vec<QueryDelta>,
// ) -> bool {
//     let has_post_cache_delta = false;
//     if start_time == result_meta.start_time && end_time == result_meta.end_time {
//         // If query start time and end time are the same as cache times, return results from
// cache         return;
//     }
//     // Query Start time > ResultCacheMeta start time & Query End time > ResultCacheMeta End time
// ->     // typical last x min/hours/days of data
//     if start_time > result_meta.start_time && end_time > result_meta.end_time {
//         // q start time : 10:00, q end time : 11:00, r start time : 09:00, r end time : 10:30
//         // Drop data between Query End time & ResultCacheMeta End time
//         deltas.push(QueryDelta {
//             delta_start_time: result_meta.end_time,
//             delta_end_time: end_time,
//             delta_removed_hits: false,
//         });
//         // Fetch data between ResultCacheMeta Start time & Query start time
//         deltas.push(QueryDelta {
//             delta_start_time: result_meta.start_time,
//             delta_end_time: start_time,
//             delta_removed_hits: true,
//         });
//     }
//     // Query Start time < ResultCacheMeta start time & Query End time > ResultCacheMeta End time
//     else if start_time < result_meta.start_time && end_time > result_meta.end_time {
//         // q start time : 10:00, q end time : 11:00, r start time : 10:30, r end time : 10:45
//         // If query times are wider than cached times, fetch both ends
//         deltas.push(QueryDelta {
//             delta_start_time: result_meta.end_time,
//             delta_end_time: end_time,
//             delta_removed_hits: false,
//         });
//         deltas.push(QueryDelta {
//             delta_start_time: start_time,
//             delta_end_time: result_meta.start_time,
//             delta_removed_hits: false,
//         });
//     }
//     // Query Start time > ResultCacheMeta start time & Query End time < ResultCacheMeta End time
//     else if start_time > result_meta.start_time && end_time < result_meta.end_time {
//         // q start time : 10:00, q end time : 11:00, r start time : 9:30, r end time : 11:15
//         // Fetch data between ResultCacheMeta Start time & Query start time
//         deltas.push(QueryDelta {
//             delta_start_time: result_meta.start_time,
//             delta_end_time: start_time,
//             delta_removed_hits: true,
//         });
//         deltas.push(QueryDelta {
//             delta_start_time: end_time,
//             delta_end_time: result_meta.end_time,
//             delta_removed_hits: true,
//         });
//     } else if start_time < result_meta.start_time && end_time < result_meta.end_time {
//         // If query starts before and ends before cache ends
//         // q start time : 10:00, q end time : 11:00, r start time : 10:30, r end time : 11:15
//         deltas.push(QueryDelta {
//             delta_start_time: start_time,
//             delta_end_time: result_meta.start_time,
//             delta_removed_hits: false,
//         });
//         deltas.push(QueryDelta {
//             delta_start_time: end_time,
//             delta_end_time: result_meta.end_time,
//             delta_removed_hits: true,
//         });
//     }
//     has_post_cache_delta
// }

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
    println!("getting file: {:?}", file);
    match disk::get(&file, None).await {
        Some(v) => Ok(String::from_utf8(v.to_vec()).unwrap()),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        )),
    }
}
