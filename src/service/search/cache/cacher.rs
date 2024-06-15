use bytes::Bytes;
use config::{get_config, meta::search::Response, utils::json};
use infra::cache::file_data::disk;

use crate::common::{
    infra::config::QUERY_RESULT_CACHE,
    meta::search::{CachedQueryResponse, QueryDelta, ResultCacheMeta},
};

pub async fn get_cached_results(
    start_time: i64,
    end_time: i64,
    _is_aggregate: bool,
    query_key: String,
    file_path: String,
    file_name: String,
) -> Option<CachedQueryResponse> {
    let cfg = get_config();
    let r = QUERY_RESULT_CACHE.read().await;
    let is_cached = r.get(&query_key).cloned(); //org_streamy_type_stream_name_query -> 
    drop(r);

    if let Some(result_meta) = is_cached {
        // calculate delta time range to fetch the delta data using search query
        let mut deltas = vec![];
        calculate_deltas_v1(&result_meta, start_time, end_time, &mut deltas);

        let remove_hits: Vec<&QueryDelta> =
            deltas.iter().filter(|d| d.delta_removed_hits).collect();

        match get_results(&file_path, &file_name).await {
            Ok(v) => {
                let mut cached_response: Response = json::from_str::<Response>(&v).unwrap();
                // also remove hits if time range is lesser than cached time range

                if remove_hits.is_empty() {
                } else {
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

                                hit_ts >= delta.delta_start_time && hit_ts < delta.delta_end_time
                            })
                            .collect();
                    }
                };

                return Some(CachedQueryResponse {
                    cached_response,
                    deltas,
                });
            }
            Err(e) => {
                log::error!("Get results from disk failed: {:?}", e);
                None
            }
        }
    } else {
        None
    }
}

pub fn calculate_deltas(
    result_meta: &ResultCacheMeta,
    start_time: i64,
    end_time: i64,
    mut deltas: Vec<QueryDelta>,
) {
    if start_time == result_meta.start_time && end_time == result_meta.end_time {
        // If query start time and end time are the same as cache times, return results from cache
        return;
    }
    // Query Start time > ResultCacheMeta start time & Query End time > ResultCacheMeta End time ->
    // typical last x min/hours/days of data
    if start_time > result_meta.start_time && end_time > result_meta.end_time {
        // q start time : 10:00, q end time : 11:00, r start time : 09:00, r end time : 10:30
        // Drop data between Query End time & ResultCacheMeta End time
        deltas.push(QueryDelta {
            delta_start_time: result_meta.end_time,
            delta_end_time: end_time,
            delta_removed_hits: false,
        });
        // Fetch data between ResultCacheMeta Start time & Query start time
        deltas.push(QueryDelta {
            delta_start_time: result_meta.start_time,
            delta_end_time: start_time,
            delta_removed_hits: true,
        });
    }
    // Query Start time < ResultCacheMeta start time & Query End time > ResultCacheMeta End time
    else if start_time < result_meta.start_time && end_time > result_meta.end_time {
        // q start time : 10:00, q end time : 11:00, r start time : 10:30, r end time : 10:45
        // If query times are wider than cached times, fetch both ends
        deltas.push(QueryDelta {
            delta_start_time: result_meta.end_time,
            delta_end_time: end_time,
            delta_removed_hits: false,
        });
        deltas.push(QueryDelta {
            delta_start_time: start_time,
            delta_end_time: result_meta.start_time,
            delta_removed_hits: false,
        });
    }
    // Query Start time > ResultCacheMeta start time & Query End time < ResultCacheMeta End time
    else if start_time > result_meta.start_time && end_time < result_meta.end_time {
        // q start time : 10:00, q end time : 11:00, r start time : 9:30, r end time : 11:15
        // Fetch data between ResultCacheMeta Start time & Query start time
        deltas.push(QueryDelta {
            delta_start_time: result_meta.start_time,
            delta_end_time: start_time,
            delta_removed_hits: true,
        });
        deltas.push(QueryDelta {
            delta_start_time: end_time,
            delta_end_time: result_meta.end_time,
            delta_removed_hits: true,
        });
    } else if start_time < result_meta.start_time && end_time < result_meta.end_time {
        // If query starts before and ends before cache ends
        // q start time : 10:00, q end time : 11:00, r start time : 10:30, r end time : 11:15
        deltas.push(QueryDelta {
            delta_start_time: start_time,
            delta_end_time: result_meta.start_time,
            delta_removed_hits: false,
        });
        deltas.push(QueryDelta {
            delta_start_time: end_time,
            delta_end_time: result_meta.end_time,
            delta_removed_hits: true,
        });
    }
}

pub fn calculate_deltas_v1(
    result_meta: &ResultCacheMeta,
    start_time: i64,
    end_time: i64,
    deltas: &mut Vec<QueryDelta>,
) {
    if start_time == result_meta.start_time && end_time == result_meta.end_time {
        // If query start time and end time are the same as cache times, return results from cache
        return;
    }
    // Query Start time > ResultCacheMeta start time & Query End time > ResultCacheMeta End time ->
    // typical last x min/hours/days of data
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
    }
    // Query Start time < ResultCacheMeta start time & Query End time > ResultCacheMeta End time
    if end_time > result_meta.end_time {
        // q end time : 11:00, r end time : 10:45
        deltas.push(QueryDelta {
            delta_start_time: start_time,
            delta_end_time: result_meta.start_time,
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

pub async fn cache_results_to_disk(
    trace_id: &str,
    file_path: &str,
    file_name: &str,
    data: String,
) -> std::io::Result<()> {
    let file = format!("results/{}/{}", file_path, file_name);
    let _ = disk::set(trace_id, &file, Bytes::from(data)).await;
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
