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

use std::{collections::HashMap, io::Error};

use actix_web::{delete, get, http::StatusCode, post, web, HttpRequest, HttpResponse};
use config::{
    get_config,
    meta::{search::SearchEventType, sql::resolve_stream_names, stream::StreamType},
    utils::json,
};
use infra::table::entity::search_jobs::Model as JobModel;
use tracing::Span;

use crate::{
    common::{
        meta::{self, http::HttpResponse as MetaHttpResponse},
        utils::http::{
            get_or_create_trace_id, get_search_event_context_from_request,
            get_stream_type_from_request,
        },
    },
    handler::http::request::search::{job::cancel_query_inner, utils::check_stream_permissions},
    service::{
        db::search_job::{search_job_partitions::*, search_jobs::*},
        search_jobs::get_result,
    },
};

// 1. submit
#[post("/{org_id}/search_jobs")]
pub async fn submit_job(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let cfg = get_config();

    let org_id = org_id.into_inner();
    let http_span = if cfg.common.tracing_search_enabled || cfg.common.tracing_enabled {
        tracing::info_span!("/api/{org_id}/_search", org_id = org_id.clone())
    } else {
        Span::none()
    };

    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or(StreamType::Logs),
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    // handle encoding for query and aggs
    let mut req: config::meta::search::Request = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };
    if let Err(e) = req.decode() {
        return Ok(MetaHttpResponse::bad_request(e));
    }

    // update timeout
    if req.timeout == 0 {
        req.timeout = cfg.limit.search_job_timeout;
    }

    // set search event type
    req.search_type = Some(SearchEventType::SearchJob);
    if req.search_event_context.is_none() {
        req.search_event_context = req
            .search_type
            .as_ref()
            .and_then(|event_type| get_search_event_context_from_request(event_type, &query));
    }

    // get stream name
    let stream_names = match resolve_stream_names(&req.query.sql) {
        Ok(v) => v.clone(),
        Err(e) => {
            return Ok(
                HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            );
        }
    };

    // Check permissions on stream
    for stream_name in stream_names.iter() {
        if let Some(res) =
            check_stream_permissions(stream_name, &org_id, &user_id, &stream_type).await
        {
            return Ok(res);
        }
    }

    // add stream_names for rbac
    let stream_names = json::to_string(&stream_names).unwrap();

    // submit query to db
    let res = submit(
        &trace_id,
        &org_id,
        &user_id,
        &stream_type.to_string(),
        &stream_names,
        &json::to_string(&req).unwrap(),
        req.query.start_time,
        req.query.end_time,
    )
    .await;

    match res {
        Ok(job_id) => Ok(MetaHttpResponse::ok(format!("job_id: {job_id}"))),
        Err(err) => {
            log::error!("[trace_id {trace_id}] sumbit query error: {}", err);
            Ok(MetaHttpResponse::internal_error(err.to_string()))
        }
    }
}

// 2. status_all
#[get("/{org_id}/search_jobs")]
pub async fn list_status(org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let res = list_status_by_org_id(&org_id).await;
    let res = match res {
        Ok(res) => res,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    Ok(HttpResponse::Ok().json(res))
}

// 3. status
#[get("/{org_id}/search_jobs/{job_id}/status")]
pub async fn get_status(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let org_id = path.0.clone();
    let job_id = path.1.clone();
    let res = get(&job_id, &org_id).await;
    let model = match res {
        Ok(res) => res,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    // check permissions
    if let Some(res) = check_permissions(&model, &org_id, &user_id).await {
        return Ok(res);
    }
    Ok(HttpResponse::Ok().json(model))
}

// 4. cancel
#[post("/{org_id}/search_jobs/{job_id}/cancel")]
pub async fn cancel_job(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let org_id = path.0.clone();
    let job_id = path.1.clone();
    match cancel_job_inner(&org_id, &job_id, &user_id).await {
        Ok(res) if res.status() != StatusCode::OK => Ok(res),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
        Ok(_) => Ok(MetaHttpResponse::ok(format!(
            "job_id: {job_id} cancel success"
        ))),
    }
}

// 5. get
#[get("/{org_id}/search_jobs/{job_id}/result")]
pub async fn get_job_result(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let org_id = path.0.clone();
    let job_id = path.1.clone();
    let res = get(&job_id, &org_id).await;
    let model = match res {
        Ok(res) => res,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    // check permissions
    if let Some(res) = check_permissions(&model, &org_id, &user_id).await {
        return Ok(res);
    }

    if model.error_message.is_some() {
        Ok(MetaHttpResponse::ok(format!(
            "job_id: {job_id} error: {}",
            model.error_message.unwrap()
        )))
    } else if model.result_path.is_none() || model.cluster.is_none() {
        Ok(MetaHttpResponse::not_found(format!(
            "job_id: {job_id} don't have result_path or cluster"
        )))
    } else {
        let path = model.result_path.clone().unwrap();
        let cluster = model.cluster.clone().unwrap();
        let response = get_result(&path, &cluster)
            .await
            .map_err(|e| Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(HttpResponse::Ok().json(response))
    }
}

// 6. delete
#[delete("/{org_id}/search_jobs/{job_id}")]
pub async fn delete_job(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let org_id = path.0.clone();
    let job_id = path.1.clone();

    // 1. cancel the query
    match cancel_job_inner(&org_id, &job_id, &user_id).await {
        Ok(res) if res.status() != StatusCode::OK && res.status() != StatusCode::BAD_REQUEST => {
            return Ok(res)
        }
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        _ => {}
    };

    // 2. make the job_id in search_job table delete
    match set_job_deleted(job_id.as_str()).await {
        Ok(true) => Ok(MetaHttpResponse::ok(format!(
            "job_id: {job_id} delete success"
        ))),
        Ok(false) => Ok(MetaHttpResponse::not_found(format!(
            "job_id: {job_id} not found"
        ))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

// 7. retry
#[post("/{org_id}/search_jobs/{job_id}/retry")]
pub async fn retry_job(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let org_id = path.0.clone();
    let job_id = path.1.clone();

    // 1. check the status of the job, only cancel, finish can be retry
    let status = get(&job_id, &org_id).await;
    let model = match status {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    // check permissions
    if let Some(res) = check_permissions(&model, &org_id, &user_id).await {
        return Ok(res);
    }

    if model.status != 2 && model.status != 3 {
        return Ok(MetaHttpResponse::forbidden(
            "Only canceled, finished job can be retry",
        ));
    }

    // 2. make the job_id as pending in search_job table
    let res = retry_search_job(&job_id).await;
    match res {
        Ok(_) => Ok(MetaHttpResponse::ok(format!(
            "job_id: {job_id} retry success"
        ))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

async fn cancel_job_inner(
    org_id: &str,
    job_id: &str,
    user_id: &str,
) -> Result<HttpResponse, Error> {
    // 1. use job_id to query the trace_id
    let job = get(job_id, org_id).await;
    if job.is_err() {
        return Ok(MetaHttpResponse::not_found(format!(
            "job_id: {job_id} not found"
        )));
    }
    let job = job.unwrap();

    // check permissions
    if let Some(res) = check_permissions(&job, org_id, user_id).await {
        return Ok(res);
    }

    // 2. use job_id to make search_job cancel
    let status = cancel_job_by_job_id(job_id).await;
    if let Err(e) = status {
        return Ok(MetaHttpResponse::bad_request(e));
    }

    // 3. use job_id to make background_partition_job cancel
    let status = status.unwrap();
    if status == 1 {
        if let Err(e) = cancel_partition_job(job_id).await {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    }

    // 4. use cancel query function to cancel the query
    cancel_query_inner(org_id, &[&job.trace_id]).await
}

// check permissions
async fn check_permissions(job: &JobModel, org_id: &str, user_id: &str) -> Option<HttpResponse> {
    let stream_type = StreamType::from(job.stream_type.as_str());
    let stream_names: Vec<String> = json::from_str(&job.stream_names).unwrap();
    for stream_name in stream_names.iter() {
        if let Some(res) =
            check_stream_permissions(stream_name, org_id, user_id, &stream_type).await
        {
            return Some(res);
        }
    }
    None
}
