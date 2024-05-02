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

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse};

use crate::{
    common::meta::{http::HttpResponse as MetaHttpResponse, synthetics::Synthetics},
    service::synthetics,
};

/// CreateSynthetic
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "CreateSynthetic",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(
        content = Synthetics,
        description = "Synthetic details",
    ),
    responses(
        (status = StatusCode::CREATED, description = "Synthetic test created", body = HttpResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error", body = HttpResponse),
    ),
)]
#[post("/{org_id}/synthetics")]
pub async fn create_synthetics(
    path: web::Path<String>,
    synthetics: web::Json<Synthetics>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    match synthetics::save(&org_id, "", synthetics.into_inner(), true).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Synthetic saved")),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// UpdateSynthetic
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "UpdateSynthetis",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Synthetics name"),
    ),
    request_body(
        content = Synthetics,
        description = "Synthetics details",
    ),
    responses(
        (status = StatusCode::OK, description = "Synthetics updated", body = HttpResponse),
        (status = StatusCode::NOT_FOUND, description = "Synthetics not found", body = HttpResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Failed to update the synthetics", body = HttpResponse),
    ),
)]
#[put("/{org_id}/synthetics/{name}")]
async fn update_synthetic(
    path: web::Path<(String, String)>,
    synthetics: web::Json<Synthetics>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match synthetics::save(&org_id, &name, synthetics.into_inner(), false).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Synthetics saved")),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// ListSynthetics
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "ListSynthetics",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = StatusCode::OK, body = Vec<Synthetics>),
    ),
)]
#[get("/{org_id}/synthetics")]
async fn list_synthetics(
    org_id: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();

    let mut _permitted = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = _req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            "synthetics",
        )
        .await
        {
            Ok(list) => {
                _permitted = list;
            }
            Err(e) => {
                return Ok(crate::common::meta::http::HttpResponse::forbidden(
                    e.to_string(),
                ));
            }
        }
        // Get List of allowed objects ends
    }

    match synthetics::list(&org_id, _permitted).await {
        Ok(data) => Ok(MetaHttpResponse::json(data)),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// GetSynthetic
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "GetSynthetics",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Synthetics name"),
    ),
    responses(
        (status = StatusCode::OK, body = Synthetics),
        (status = StatusCode::NOT_FOUND, description = "Synthetics not found", body = HttpResponse),
    ),
)]
#[get("/{org_id}/synthetics/{name}")]
async fn get_synthetics(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match synthetics::get(&org_id, &name).await {
        Ok(data) => Ok(MetaHttpResponse::json(data)),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// DeleteSynthetic
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "DeleteSynthetics",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Synthetics name"),
    ),
    responses(
        (status = StatusCode::OK, description = "Success", body = HttpResponse),
        (status = StatusCode::NOT_FOUND, description = "NotFound", body = HttpResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Error", body = HttpResponse),
    ),
)]
#[delete("/{org_id}/synthetics/{name}")]
async fn delete_synthetics(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match synthetics::delete(&org_id, &name).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Synthetics deleted")),
        Err(e) => match e {
            (http::StatusCode::FORBIDDEN, e) => Ok(MetaHttpResponse::forbidden(e)),
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// EnableSynthetics
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "EnableSynthetics",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Synthetics name"),
        ("value" = bool, Query, description = "Enable or disable synthetic"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/synthetics/{name}/enable")]
async fn enable_synthetics(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let enable = match query.get("value") {
        Some(v) => v.parse::<bool>().unwrap_or_default(),
        None => false,
    };
    let mut resp = HashMap::new();
    resp.insert("enabled".to_string(), enable);
    match synthetics::enable(&org_id, &name, enable).await {
        Ok(_) => Ok(MetaHttpResponse::json(resp)),
        Err(e) => match e {
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// TriggerSynthetics
#[utoipa::path(
    context_path = "/api",
    tag = "Synthetics",
    operation_id = "TriggerSynthetics",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Synthetics name"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/synthetics/{name}/trigger")]
async fn trigger_synthetics(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match synthetics::trigger(&org_id, &name).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Synthetics triggered")),
        Err(e) => match e {
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}
