use config::{meta::search::Response, utils::json};

use crate::common::{
    infra::config::QUERY_RESULT_CACHE,
    meta::search::{CachedQueryResponse, QueryDelta, ResultMeta},
    utils::result_writer,
};

pub async fn get_cached_results(
    start_time: i64,
    end_time: i64,
    is_aggregate: bool,
    query_key: String,
    file_path: String,
    file_name: String,
) -> Option<CachedQueryResponse> {
    let r = QUERY_RESULT_CACHE.read().await;
    let is_cached = r.get(&query_key).cloned();
    drop(r);
    if let Some(result_meta) = is_cached {
        if start_time >= result_meta.start_time
            && end_time >= result_meta.end_time
            && is_aggregate == result_meta.is_aggregate
        {
            match result_writer::get_results(&file_path, &file_name).await {
                Ok(v) => {
                    let res = json::from_str::<Response>(&v).unwrap();
                    // calculate delta time range to fetch the delta data using search query
                    let mut deltas = vec![];

                    // also remove hits if time range is lesser than cached time range

                    Some(CachedQueryResponse {
                        cached_response: res,
                        deltas,
                    })
                }
                Err(e) => {
                    log::error!("Get results from disk failed: {:?}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn calculate_deltas(
    result_meta: &ResultMeta,
    start_time: i64,
    end_time: i64,
    mut deltas: Vec<QueryDelta>,
) {
    if start_time == result_meta.start_time && end_time == result_meta.end_time {
        // If query start time and end time are the same as cache times, return results from cache
        return;
    }

    // Query Start time > ResultMeta start time & Query End time > ResultMeta End time -> typical
    // last x min/hours/days of data
    if start_time > result_meta.start_time && end_time > result_meta.end_time {
        // Fetch data between Query End time & ResultMeta End time
        deltas.push(QueryDelta {
            delta_start_time: result_meta.end_time,
            delta_end_time: end_time,
            delta_removed_hits: false,
        });
        // Drop data between ResultMeta Start time & Query start time
        deltas.push(QueryDelta {
            delta_start_time: result_meta.start_time,
            delta_end_time: start_time,
            delta_removed_hits: true,
        });
    }
    // Query Start time < ResultMeta start time & Query End time > ResultMeta End time
    else if start_time < result_meta.start_time && end_time > result_meta.end_time {
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
    // Query Start time > ResultMeta start time & Query End time < ResultMeta End time
    else if start_time > result_meta.start_time && end_time < result_meta.end_time {
        // Drop data between Query End time & ResultMeta End time
        deltas.push(QueryDelta {
            delta_start_time: result_meta.start_time,
            delta_end_time: start_time,
            delta_removed_hits: true,
        });
        deltas.push(QueryDelta {
            delta_start_time: start_time,
            delta_end_time: result_meta.start_time,
            delta_removed_hits: false,
        });
    } else if start_time < result_meta.start_time && end_time < result_meta.end_time {
        // If query starts before and ends before cache ends
        drop_data(end_time, result_meta.end_time).await;
        drop_data(result_meta.start_time, start_time).await;
    }

    modify_cached_data().await; // Update the cache with new data
}
