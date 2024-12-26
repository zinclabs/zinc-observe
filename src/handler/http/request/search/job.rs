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

use std::io::Error;

use actix_web::{delete, get, put, web, HttpResponse};
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

use crate::common::meta::http::HttpResponse as MetaHttpResponse;

#[delete("/{org_id}/query_manager/{trace_id}")]
pub async fn cancel_query(params: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, trace_id) = params.into_inner();
    let trace_ids = trace_id.split(',').collect::<Vec<&str>>();
    cancel_query_inner(&org_id, &trace_ids).await
}

#[put("/{org_id}/query_manager/cancel")]
pub async fn cancel_multiple_query(
    params: web::Path<String>,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let org_id = params.into_inner();
    let trace_ids: Vec<String> = match config::utils::json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    let trace_ids = trace_ids.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
    cancel_query_inner(&org_id, &trace_ids).await
}

#[get("/{org_id}/query_manager/status")]
pub async fn query_status(_params: web::Path<String>) -> Result<HttpResponse, Error> {
    let res = crate::service::search::query_status().await;
    match res {
        Ok(query_status) => Ok(HttpResponse::Ok().json(query_status)),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

pub async fn cancel_query_inner(org_id: &str, trace_ids: &[&str]) -> Result<HttpResponse, Error> {
    if trace_ids.is_empty() {
        return Ok(HttpResponse::BadRequest().json("Invalid trace_id"));
    }
    let mut res = Vec::with_capacity(trace_ids.len());
    for trace_id in trace_ids {
        if trace_id.is_empty() {
            continue;
        }
        let ret = if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::search::cancel_query(org_id, trace_id).await
        } else {
            crate::service::search::cancel_query(org_id, trace_id).await
        };
        match ret {
            Ok(status) => res.push(status),
            Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        }
    }
    Ok(HttpResponse::Ok().json(res))
}
