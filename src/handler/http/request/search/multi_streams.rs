// Copyright 2024 Zinc Labs Inc.
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

use std::{collections::HashMap, io::Error};

use actix_web::{get, http::StatusCode, post, web, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use config::{
    get_config,
    meta::{
        search,
        stream::StreamType,
        usage::{RequestStats, UsageType},
    },
    metrics,
    utils::{base64, json},
};
use infra::errors;
use tracing::Instrument;

use crate::{
    common::{
        meta::{self, http::HttpResponse as MetaHttpResponse},
        utils::{
            functions,
            http::{
                get_or_create_trace_id_and_span, get_search_type_from_request,
                get_stream_type_from_request,
            },
        },
    },
    service::{search as SearchService, usage::report_request_usage_stats},
};

/// SearchStreamData
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchSQL",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = SearchRequest, description = "Search query", content_type = "application/json", example = json!({
        "query": {
            "sql": "select * from k8s ",
            "start_time": 1675182660872049i64,
            "end_time": 1675185660872049i64,
            "from": 0,
            "size": 10
        },
        "aggs": {
            "histogram": "select histogram(_timestamp, '30 second') AS zo_sql_key, count(*) AS zo_sql_num from query GROUP BY zo_sql_key ORDER BY zo_sql_key"
        }
    })),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "hits": [
                {
                    "_p": "F",
                    "_timestamp": 1674213225158000i64,
                    "kubernetes": {
                        "container_hash": "dkr.ecr.us-west-2.amazonaws.com/openobserve@sha256:3dbbb0dc1eab2d5a3b3e4a75fd87d194e8095c92d7b2b62e7cdbd07020f54589",
                        "container_image": "dkr.ecr.us-west-2.amazonaws.com/openobserve:v0.0.3",
                        "container_name": "openobserve",
                        "docker_id": "eb0983bdb9ff9360d227e6a0b268fe3b24a0868c2c2d725a1516c11e88bf5789",
                        "host": "ip.us-east-2.compute.internal",
                        "namespace_name": "openobserve",
                        "pod_id": "35a0421f-9203-4d73-9663-9ff0ce26d409",
                        "pod_name": "openobserve-ingester-0"
                    },
                    "log": "[2023-01-20T11:13:45Z INFO  actix_web::middleware::logger] 10.2.80.192 \"POST /api/demo/_bulk HTTP/1.1\" 200 68",
                    "stream": "stderr"
                }
            ],
            "aggs": {
                "agg1": [
                    {
                        "key": "2023-01-15 14:00:00",
                        "num": 345940
                    },
                    {
                        "key": "2023-01-15 19:00:00",
                        "num": 384026
                    }
                ]
            },
            "total": 27179431,
            "from": 0,
            "size": 1,
            "scan_size": 28943
        })),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/_search_multi")]
pub async fn search_multi(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let org_id = org_id.into_inner();
    let cfg = get_config();
    let started_at = Utc::now().timestamp_micros();
    let (trace_id, http_span) =
        get_or_create_trace_id_and_span(in_req.headers(), format!("/api/{org_id}/_search_multi"));

    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or(StreamType::Logs),
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    let search_type = match get_search_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    // handle encoding for query and aggs
    let mut multi_req: search::MultiStreamRequest = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    let mut query_fn = multi_req
        .query_fn
        .as_ref()
        .and_then(|v| base64::decode_url(v).ok());

    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let user_id = in_req.headers().get("user_id").unwrap().to_str().unwrap();
    let mut queries = multi_req.to_query_req();
    let mut multi_res = search::Response::new(multi_req.from, multi_req.size);

    // Before making any rpc requests, first check the sql expressions can be decoded correctly
    for req in queries.iter_mut() {
        if let Err(e) = req.decode() {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    }
    let queries_len = queries.len();

    for mut req in queries {
        let mut rpc_req: proto::cluster_rpc::SearchRequest = req.to_owned().into();
        rpc_req.org_id = org_id.to_string();
        rpc_req.stream_type = stream_type.to_string();
        let resp: SearchService::sql::Sql = match crate::service::search::sql::Sql::new(&rpc_req)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return Ok(match e {
                    errors::Error::ErrorCode(code) => HttpResponse::InternalServerError()
                        .json(meta::http::HttpResponse::error_code(code)),
                    _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                        StatusCode::INTERNAL_SERVER_ERROR.into(),
                        e.to_string(),
                    )),
                });
            }
        };

        // Check permissions on stream
        #[cfg(feature = "enterprise")]
        {
            use crate::common::{
                infra::config::USERS,
                utils::auth::{is_root_user, AuthExtractor},
            };

            if !is_root_user(user_id) {
                let user: meta::user::User =
                    USERS.get(&format!("{org_id}/{user_id}")).unwrap().clone();

                if user.is_external
                    && !crate::handler::http::auth::validator::check_permissions(
                        user_id,
                        AuthExtractor {
                            auth: "".to_string(),
                            method: "GET".to_string(),
                            o2_type: format!("{}:{}", stream_type, resp.stream_name),
                            org_id: org_id.clone(),
                            bypass_check: false,
                            parent_id: "".to_string(),
                        },
                        Some(user.role),
                    )
                    .await
                {
                    return Ok(MetaHttpResponse::forbidden("Unauthorized Access"));
                }
            }
            // Check permissions on stream ends
        }

        req.query.query_fn = query_fn.clone();

        for fn_name in functions::get_all_transform_keys(&org_id).await {
            if req.query.sql.contains(&format!("{}(", fn_name)) {
                req.query.uses_zo_fn = true;
                break;
            }
        }

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
        log::info!("http search multi API wait in queue took: {}", took_wait);

        let trace_id = trace_id.clone();
        // do search
        let search_fut = SearchService::search(
            &trace_id,
            &org_id,
            stream_type,
            Some(user_id.to_string()),
            &req,
        );

        let search_res = if !cfg.common.tracing_enabled && cfg.common.tracing_search_enabled {
            search_fut.instrument(http_span.clone().unwrap()).await
        } else {
            search_fut.await
        };

        match search_res {
            Ok(mut res) => {
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
                res.set_trace_id(trace_id);
                res.set_local_took(start.elapsed().as_millis() as usize, took_wait);

                let req_stats = RequestStats {
                    records: res.hits.len() as i64,
                    response_time: time,
                    size: res.scan_size as f64,
                    request_body: Some(req.query.sql),
                    user_email: Some(user_id.to_string()),
                    min_ts: Some(req.query.start_time),
                    max_ts: Some(req.query.end_time),
                    cached_ratio: Some(res.cached_ratio),
                    search_type,
                    ..Default::default()
                };
                let num_fn = req.query.query_fn.is_some() as u16;
                report_request_usage_stats(
                    req_stats,
                    &org_id,
                    &resp.stream_name,
                    StreamType::Logs,
                    UsageType::Search,
                    num_fn,
                    started_at,
                )
                .await;

                multi_res.took += res.took;

                if res.total > multi_res.total {
                    multi_res.total = res.total
                }
                multi_res.from = res.from;
                multi_res.size += res.size;
                multi_res.file_count += res.file_count;
                multi_res.scan_size += res.scan_size;
                multi_res.scan_records += res.scan_records;
                multi_res.columns.append(&mut res.columns);
                multi_res.hits.append(&mut res.hits);
                multi_res.aggs.extend(res.aggs.into_iter());
                multi_res.response_type = res.response_type;
                multi_res.trace_id = res.trace_id;
                multi_res.cached_ratio = res.cached_ratio;
            }
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_search",
                        "500",
                        &org_id,
                        "",
                        stream_type.to_string().as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_search",
                        "500",
                        &org_id,
                        "",
                        stream_type.to_string().as_str(),
                    ])
                    .inc();

                log::error!("search error: {:?}", err);
                multi_res.function_error = format!("{};{:?}", multi_res.function_error, err);
                if let errors::Error::ErrorCode(code) = err {
                    if let errors::ErrorCodes::SearchCancelQuery(_) = code {
                        return Ok(HttpResponse::TooManyRequests().json(
                            meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            ),
                        ));
                    }
                }
            }
        }
    }

    multi_res.cached_ratio /= queries_len;
    multi_res.hits.sort_by(|a, b| {
        let a_ts = a.get("_timestamp").unwrap().as_i64().unwrap();
        let b_ts = b.get("_timestamp").unwrap().as_i64().unwrap();
        b_ts.cmp(&a_ts)
    });
    Ok(HttpResponse::Ok().json(multi_res))
}

/// SearchMultiStreamPartition
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchPartitionMulti",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = SearchRequest, description = "Search query", content_type = "application/json", example = json!({
        "sql": "select * from k8s ",
        "start_time": 1675182660872049i64,
        "end_time": 1675185660872049i64
    })),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "file_num": 10,
            "original_size": 10240,
            "compressed_size": 1024,
            "partitions": [
                [1674213225158000i64, 1674213225158000i64],
                [1674213225158000i64, 1674213225158000i64],
            ]
        })),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/_search_partition_multi")]
pub async fn _search_partition_multi(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();
    let (trace_id, http_span) = get_or_create_trace_id_and_span(
        in_req.headers(),
        format!("/api/{org_id}/_search_partition"),
    );

    let org_id = org_id.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or(StreamType::Logs),
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    let req: search::MultiSearchPartitionRequest = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    let search_fut = SearchService::search_partition_multi(&trace_id, &org_id, stream_type, &req);
    let search_res = if !cfg.common.tracing_enabled && cfg.common.tracing_search_enabled {
        search_fut.instrument(http_span.unwrap()).await
    } else {
        search_fut.await
    };
    // do search
    match search_res {
        Ok(res) => {
            let time = start.elapsed().as_secs_f64();
            metrics::HTTP_RESPONSE_TIME
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "200",
                    &org_id,
                    "",
                    stream_type.to_string().as_str(),
                ])
                .observe(time);
            metrics::HTTP_INCOMING_REQUESTS
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "200",
                    &org_id,
                    "",
                    stream_type.to_string().as_str(),
                ])
                .inc();
            Ok(HttpResponse::Ok().json(res))
        }
        Err(err) => {
            let time = start.elapsed().as_secs_f64();
            metrics::HTTP_RESPONSE_TIME
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "500",
                    &org_id,
                    "",
                    stream_type.to_string().as_str(),
                ])
                .observe(time);
            metrics::HTTP_INCOMING_REQUESTS
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "500",
                    &org_id,
                    "",
                    stream_type.to_string().as_str(),
                ])
                .inc();
            log::error!("search error: {:?}", err);
            Ok(match err {
                errors::Error::ErrorCode(code) => HttpResponse::InternalServerError().json(
                    meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                ),
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            })
        }
    }
}

/// SearchAround
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchAroundMulti",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_names" = String, Path, description = "base64 encoded comma separated stream names"),
        ("key" = i64, Query, description = "around key"),
        ("size" = i64, Query, description = "around size"),
        ("timeout" = Option<i64>, Query, description = "timeout, seconds"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "hits": [
                {
                    "_p": "F",
                    "_timestamp": 1674213225158000i64,
                    "kubernetes": {
                        "container_hash": "dkr.ecr.us-west-2.amazonaws.com/openobserve@sha256:3dbbb0dc1eab2d5a3b3e4a75fd87d194e8095c92d7b2b62e7cdbd07020f54589",
                        "container_image": "dkr.ecr.us-west-2.amazonaws.com/openobserve:v0.0.3",
                        "container_name": "openobserve",
                        "docker_id": "eb0983bdb9ff9360d227e6a0b268fe3b24a0868c2c2d725a1516c11e88bf5789",
                        "host": "ip.us-east-2.compute.internal",
                        "namespace_name": "openobserve",
                        "pod_id": "35a0421f-9203-4d73-9663-9ff0ce26d409",
                        "pod_name": "openobserve-ingester-0"
                    },
                    "log": "[2023-01-20T11:13:45Z INFO  actix_web::middleware::logger] 10.2.80.192 \"POST /api/demo/_bulk HTTP/1.1\" 200 68",
                    "stream": "stderr"
                }
            ],
            "total": 10,
            "from": 0,
            "size": 10,
            "scan_size": 28943
        })),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{stream_names}/_around_multi")]
pub async fn around_multi(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

    let (org_id, stream_names) = path.into_inner();
    let cfg = get_config();

    let (trace_id, http_span) = get_or_create_trace_id_and_span(
        in_req.headers(),
        format!("/api/{org_id}/{stream_names}/_around_multi"),
    );

    let stream_names = base64::decode_url(&stream_names)?;
    let mut uses_fn = false;
    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or(StreamType::Logs),
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    let around_key = match query.get("key") {
        Some(v) => v.parse::<i64>().unwrap_or(0),
        None => return Ok(MetaHttpResponse::bad_request("around key is empty")),
    };
    let mut query_fn = query
        .get("query_fn")
        .and_then(|v| base64::decode_url(v).ok());
    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let mut around_sqls = stream_names
        .split(',')
        .collect::<Vec<&str>>()
        .iter()
        .map(|name| format!("SELECT * FROM \"{}\" ", name))
        .collect::<Vec<String>>();
    if let Some(v) = query.get("sql") {
        let sqls = v.split(',').collect::<Vec<&str>>();
        for (i, sql) in sqls.into_iter().enumerate() {
            uses_fn = functions::get_all_transform_keys(&org_id)
                .await
                .iter()
                .any(|fn_name| v.contains(&format!("{}(", fn_name)));
            if uses_fn {
                if let Ok(sql) = base64::decode_url(sql) {
                    around_sqls[i] = sql;
                }
            }
        }
    }

    let around_size = query
        .get("size")
        .map_or(10, |v| v.parse::<i64>().unwrap_or(10));

    let regions = query.get("regions").map_or(vec![], |regions| {
        regions
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    let clusters = query.get("clusters").map_or(vec![], |clusters| {
        clusters
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    let timeout = query
        .get("timeout")
        .map_or(0, |v| v.parse::<i64>().unwrap_or(0));
    let around_start_time = around_key
        - Duration::try_seconds(900)
            .unwrap()
            .num_microseconds()
            .unwrap();
    let around_end_time = around_key
        + Duration::try_seconds(900)
            .unwrap()
            .num_microseconds()
            .unwrap();

    let mut multi_resp = search::Response {
        size: around_size,
        ..Default::default()
    };

    let user_id = in_req
        .headers()
        .get("user_id")
        .unwrap()
        .to_str()
        .ok()
        .map(|v| v.to_string());

    for around_sql in around_sqls.iter() {
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
        log::info!(
            "http search around multi API wait in queue took: {}",
            took_wait
        );

        // search forward
        let req = config::meta::search::Request {
            query: config::meta::search::Query {
                sql: around_sql.clone(),
                from: 0,
                size: around_size / 2,
                start_time: around_start_time,
                end_time: around_key,
                sort_by: Some(format!("{} DESC", cfg.common.column_timestamp)),
                sql_mode: "".to_string(),
                quick_mode: false,
                query_type: "".to_string(),
                track_total_hits: false,
                query_context: None,
                uses_zo_fn: uses_fn,
                query_fn: query_fn.clone(),
                skip_wal: false,
            },
            aggs: HashMap::new(),
            encoding: config::meta::search::RequestEncoding::Empty,
            regions: regions.clone(),
            clusters: clusters.clone(),
            timeout,
            search_type: Some(search::SearchEventType::UI),
        };
        let search_fut =
            SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req);
        let search_res = if !cfg.common.tracing_enabled && cfg.common.tracing_search_enabled {
            search_fut.instrument(http_span.clone().unwrap()).await
        } else {
            search_fut.await
        };
        let resp_forward = match search_res {
            Ok(res) => res,
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_around",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.to_string().as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_around",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.to_string().as_str(),
                    ])
                    .inc();
                log::error!("multi search around error: {:?}", err);
                return Ok(match err {
                    errors::Error::ErrorCode(code) => match code {
                        errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                            .json(meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            )),
                        _ => HttpResponse::InternalServerError().json(
                            meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            ),
                        ),
                    },
                    _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                        StatusCode::INTERNAL_SERVER_ERROR.into(),
                        err.to_string(),
                    )),
                });
            }
        };

        // search backward
        let req = config::meta::search::Request {
            query: config::meta::search::Query {
                sql: around_sql.clone(),
                from: 0,
                size: around_size / 2,
                start_time: around_key,
                end_time: around_end_time,
                sort_by: Some(format!("{} ASC", cfg.common.column_timestamp)),
                sql_mode: "".to_string(),
                quick_mode: false,
                query_type: "".to_string(),
                track_total_hits: false,
                query_context: None,
                uses_zo_fn: uses_fn,
                query_fn: query_fn.clone(),
                skip_wal: false,
            },
            aggs: HashMap::new(),
            encoding: config::meta::search::RequestEncoding::Empty,
            regions: regions.clone(),
            clusters: clusters.clone(),
            timeout,
            search_type: Some(search::SearchEventType::UI),
        };
        let search_fut =
            SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req);
        let search_res = if !cfg.common.tracing_enabled && cfg.common.tracing_search_enabled {
            search_fut.instrument(http_span.clone().unwrap()).await
        } else {
            search_fut.await
        };
        let resp_backward = match search_res {
            Ok(res) => res,
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_around",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.to_string().as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_around",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.to_string().as_str(),
                    ])
                    .inc();
                log::error!("multi search around error: {:?}", err);
                return Ok(match err {
                    errors::Error::ErrorCode(code) => match code {
                        errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                            .json(meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            )),
                        _ => HttpResponse::InternalServerError().json(
                            meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            ),
                        ),
                    },
                    _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                        StatusCode::INTERNAL_SERVER_ERROR.into(),
                        err.to_string(),
                    )),
                });
            }
        };

        let hits_num_backward = resp_backward.hits.len();
        for i in 0..hits_num_backward {
            multi_resp
                .hits
                .push(resp_backward.hits[hits_num_backward - 1 - i].to_owned());
        }
        let hits_num_forward = resp_forward.hits.len();
        for i in 0..hits_num_forward {
            multi_resp.hits.push(resp_forward.hits[i].to_owned());
        }
        let total_hits = hits_num_forward + hits_num_backward;
        let total_scan_size = resp_forward.scan_size + resp_backward.scan_size;
        multi_resp.total += total_hits;
        multi_resp.scan_size += total_scan_size;
        multi_resp.took += resp_forward.took + resp_backward.took;

        let time = start.elapsed().as_secs_f64();
        metrics::HTTP_RESPONSE_TIME
            .with_label_values(&[
                "/api/org/_around",
                "200",
                &org_id,
                &stream_names,
                stream_type.to_string().as_str(),
            ])
            .observe(time);
        metrics::HTTP_INCOMING_REQUESTS
            .with_label_values(&[
                "/api/org/_around",
                "200",
                &org_id,
                &stream_names,
                stream_type.to_string().as_str(),
            ])
            .inc();

        let user_id = match &user_id {
            Some(v) => v,
            None => "",
        };
        let req_stats = RequestStats {
            records: total_hits as i64,
            response_time: time,
            size: multi_resp.scan_size as f64,
            request_body: Some(around_sql.to_string()),
            user_email: Some(user_id.to_string()),
            min_ts: Some(around_start_time),
            max_ts: Some(around_end_time),
            ..Default::default()
        };
        let num_fn = query_fn.is_some() as u16;
        report_request_usage_stats(
            req_stats,
            &org_id,
            &stream_names,
            StreamType::Logs,
            UsageType::SearchAround,
            num_fn,
            started_at,
        )
        .await;
    }

    multi_resp.hits.sort_by(|a, b| {
        let a_ts = a.get("_timestamp").unwrap().as_i64().unwrap();
        let b_ts = b.get("_timestamp").unwrap().as_i64().unwrap();
        b_ts.cmp(&a_ts)
    });
    Ok(HttpResponse::Ok().json(multi_resp))
}
