// Copyright 2023 Zinc Labs Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_web::{delete, get, http, post, web, HttpResponse};
use std::io::Error;

use crate::common::meta::{alerts::AlertDestination, http::HttpResponse as MetaHttpResponse};
use crate::service::alerts::destinations;

/** CreateDestination */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "CreateDestination",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("destination_name" = String, Path, description = "Destination name"),
      ),
    request_body(content = AlertDestination, description = "Destination data", content_type = "application/json"),  
    responses(
        (status = 200, description="Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description="Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/alerts/destinations/{destination_name}")]
pub async fn save_destination(
    path: web::Path<(String, String)>,
    dest: web::Json<AlertDestination>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    let dest = dest.into_inner();
    match destinations::save_destination(&org_id, &name, dest).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination saved")),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/** ListDestinations */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListDestinations",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description="Success", content_type = "application/json", body = Vec<AlertDestinationResponse>),
        (status = 400, description="Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/alerts/destinations")]
async fn list_destinations(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    match destinations::list_destinations(&org_id).await {
        Ok(data) => Ok(MetaHttpResponse::json(data)),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/** GetDestination */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "GetDestination",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("destination_name" = String, Path, description = "Destination name"),
      ),
    responses(
        (status = 200, description="Success",  content_type = "application/json", body = AlertDestinationResponse),
        (status = 404, description="NotFound", content_type = "application/json", body = HttpResponse), 
    )
)]
#[get("/{org_id}/alerts/destinations/{destination_name}")]
async fn get_destination(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match destinations::get_destination(&org_id, &name).await {
        Ok(data) => Ok(MetaHttpResponse::json(data)),
        Err(e) => Ok(MetaHttpResponse::not_found(e)),
    }
}

/** DeleteDestination */
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "DeleteAlertDestination",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("destination_name" = String, Path, description = "Destination name"),
    ),
    responses(
        (status = 200, description = "Success",   content_type = "application/json", body = HttpResponse),
        (status = 403, description = "Forbidden", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Error",     content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/alerts/destinations/{destination_name}")]
async fn delete_destination(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match destinations::delete_destination(&org_id, &name).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination deleted")),
        Err(e) => match e {
            (http::StatusCode::FORBIDDEN, e) => Ok(MetaHttpResponse::forbidden(e)),
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}
