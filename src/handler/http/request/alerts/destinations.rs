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

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse};

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    handler::http::models::alerts::{
        requests::{SaveDestinationRequestBody, UpdateDestinationRequestBody},
        responses::{GetDestinationResponseBody, ListDestinationsResponseBody},
    },
    service::alerts::destinations,
};

/// CreateDestination
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "CreateDestination",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    request_body(content = SaveDestinationRequestBody, description = "Destination data", content_type = "application/json"),  
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/alerts/destinations")]
pub async fn save_destination(
    path: web::Path<String>,
    req_body: web::Json<SaveDestinationRequestBody>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let dest = req_body.into_inner().into();
    match destinations::save(&org_id, "", dest, true).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination saved")),
        Err(e) => match e {
            (http::StatusCode::BAD_REQUEST, e) => Ok(MetaHttpResponse::bad_request(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// UpdateDestination
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "UpdateDestination",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("destination_name" = String, Path, description = "Destination name"),
      ),
    request_body(content = UpdateDestinationRequestBody, description = "Destination data", content_type = "application/json"),  
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/alerts/destinations/{destination_name}")]
pub async fn update_destination(
    path: web::Path<(String, String)>,
    req_body: web::Json<UpdateDestinationRequestBody>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    let dest = req_body.into_inner().into();
    match destinations::save(&org_id, &name, dest, false).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination saved")),
        Err(e) => match e {
            (http::StatusCode::BAD_REQUEST, e) => Ok(MetaHttpResponse::bad_request(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// GetDestination
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
        (status = 200, description = "Success",  content_type = "application/json", body = GetDestinationResponseBody),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse), 
    )
)]
#[get("/{org_id}/alerts/destinations/{destination_name}")]
async fn get_destination(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match destinations::get(&org_id, &name).await {
        Ok(data) => {
            let resp_body: GetDestinationResponseBody = data.into();
            Ok(MetaHttpResponse::json(resp_body))
        }
        Err(e) => Ok(MetaHttpResponse::not_found(e)),
    }
}

/// ListDestinations
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
        (status = 200, description = "Success", content_type = "application/json", body = ListDestinationsResponseBody),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/alerts/destinations")]
async fn list_destinations(
    path: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();

    let mut _permitted = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = _req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            "destination",
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

    match destinations::list(&org_id, _permitted).await {
        Ok(data) => {
            let resp_body: ListDestinationsResponseBody = data.into();
            Ok(MetaHttpResponse::json(resp_body))
        }
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// DeleteDestination
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
        (status = 409, description = "Conflict", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",   content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/alerts/destinations/{destination_name}")]
async fn delete_destination(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match destinations::delete(&org_id, &name).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination deleted")),
        Err(e) => match e {
            (http::StatusCode::CONFLICT, e) => Ok(MetaHttpResponse::conflict(e)),
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}
