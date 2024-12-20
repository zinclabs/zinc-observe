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

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    handler::http::models::destinations::Destination,
    service::{alerts::destinations, db::alerts::destinations::DestinationError},
};

impl From<DestinationError> for HttpResponse {
    fn from(value: DestinationError) -> Self {
        match value {
            DestinationError::InUse(e) => MetaHttpResponse::conflict(DestinationError::InUse(e)),
            DestinationError::InfraError(e) => {
                MetaHttpResponse::internal_error(DestinationError::InfraError(e))
            }
            DestinationError::NotFound => MetaHttpResponse::not_found(DestinationError::NotFound),
            other_err => MetaHttpResponse::bad_request(other_err),
        }
    }
}

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
    request_body(content = Destination, description = "Destination data", content_type = "application/json"),  
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/alerts/destinations")]
pub async fn save_destination(
    path: web::Path<String>,
    dest: web::Json<Destination>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let dest = match dest.into_inner().into(&org_id) {
        Ok(dest) => dest,
        Err(e) => return Ok(e.into()),
    };
    match destinations::save(&org_id, "", dest, true).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination saved")),
        Err(e) => Ok(e.into()),
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
    request_body(content = Destination, description = "Destination data", content_type = "application/json"),  
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/alerts/destinations/{destination_name}")]
pub async fn update_destination(
    path: web::Path<(String, String)>,
    dest: web::Json<Destination>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    let dest = match dest.into_inner().into(&org_id) {
        Ok(dest) => dest,
        Err(e) => return Ok(e.into()),
    };
    match destinations::save(&org_id, &name, dest, false).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert destination saved")),
        Err(e) => Ok(e.into()),
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
        (status = 200, description = "Success",  content_type = "application/json", body = Destination),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse), 
    )
)]
#[get("/{org_id}/alerts/destinations/{destination_name}")]
async fn get_destination(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    match destinations::get(&org_id, &name).await {
        Ok(data) => Ok(MetaHttpResponse::json(Destination::from(data))),
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
        (status = 200, description = "Success", content_type = "application/json", body = Vec<Destination>),
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
        Ok(data) => Ok(MetaHttpResponse::json(
            data.into_iter().map(Destination::from).collect::<Vec<_>>(),
        )),
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
        Err(e) => Ok(e.into()),
    }
}
