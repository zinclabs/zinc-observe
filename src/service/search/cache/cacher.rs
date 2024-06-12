pub async fn check_cache(
    query: &str,
    start_time: i64,
    end_time: i64,
    is_aggregate: bool,
) -> Option<String> {
    let r = QUERY_RESULT_CACHE.read().await;
    let is_cached = r.get(&query_key).cloned();
    drop(r);
    if let Some(result_meta) = is_cached {
        if req.query.start_time >= result_meta.start_time
            && req.query.end_time >= result_meta.end_time
            && is_aggregate == result_meta.is_aggregate
        {
            match result_writer::get_results(&file_path, &file_name).await {
                Ok(v) => {
                    let res = json::from_str::<config::meta::search::Response>(&v).unwrap();
                    let time = start.elapsed().as_secs_f64();
                    metrics::HTTP_RESPONSE_TIME
                        .with_label_values(&[
                            "/api/org/_search",
                            "200",
                            &org_id,
                            "",
                            stream_type.to_string().as_str(),
                        ])
                        .observe(time);
                    metrics::HTTP_INCOMING_REQUESTS
                        .with_label_values(&[
                            "/api/org/_search",
                            "200",
                            &org_id,
                            "",
                            stream_type.to_string().as_str(),
                        ])
                        .inc();
                    let req_stats = RequestStats {
                        records: res.hits.len() as i64,
                        response_time: time,
                        size: res.scan_size as f64,
                        request_body: Some(req.query.sql),
                        user_email: Some(user_id.to_str().unwrap().to_string()),
                        min_ts: Some(req.query.start_time),
                        max_ts: Some(req.query.end_time),
                        cached_ratio: Some(res.cached_ratio),
                        search_type,
                        trace_id: Some(trace_id),
                        ..Default::default()
                    };
                    let num_fn = req.query.query_fn.is_some() as u16;
                    report_request_usage_stats(
                        req_stats,
                        &org_id,
                        &stream_name,
                        StreamType::Logs,
                        UsageType::Search,
                        num_fn,
                    )
                    .await;
                    return Ok(HttpResponse::Ok()
                        .append_header(("O2_CACHED_RES", "true"))
                        .json(res));
                }
                Err(e) => {
                    println!("Get results from disk failed: {:?}", e);
                }
            }
        }
    };
}
