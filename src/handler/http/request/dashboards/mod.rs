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

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse, Responder};

use crate::{
    common::meta::{dashboards::MoveDashboard, http::HttpResponse as MetaHttpResponse},
    service::dashboards,
};

pub mod reports;

/// CreateDashboard
#[utoipa::path(
    context_path = "/api",
    tag = "Dashboards",
    operation_id = "CreateDashboard",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(
        content = Dashboard,
        description = "Dashboard details",
        example = json!({
            "title": "Network Traffic Overview",
            "description": "Traffic patterns and network performance of the infrastructure",
        }),
    ),
    responses(
        (status = StatusCode::CREATED, description = "Dashboard created", body = Dashboard),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error", body = HttpResponse),
    ),
)]
#[post("/{org_id}/dashboards")]
pub async fn create_dashboard(
    path: web::Path<String>,
    body: web::Bytes,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let folder = get_folder(req);
    let resp = match dashboards::create_dashboard(&org_id, &folder, body).await {
        Ok(resp) => resp,
        Err(_) => HttpResponse::InternalServerError().into(),
    };
    Ok(resp)
}

/// UpdateDashboard
#[utoipa::path(
    context_path = "/api",
    tag = "Dashboards",
    operation_id = "UpdateDashboard",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("dashboard_id" = String, Path, description = "Dashboard ID"),
    ),
    request_body(
        content = Dashboard,
        description = "Dashboard details",
    ),
    responses(
        (status = StatusCode::OK, description = "Dashboard updated", body = HttpResponse),
        (status = StatusCode::NOT_FOUND, description = "Dashboard not found", body = HttpResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Failed to update the dashboard", body = HttpResponse),
    ),
)]
#[put("/{org_id}/dashboards/{dashboard_id}")]
async fn update_dashboard(
    path: web::Path<(String, String)>,
    body: web::Bytes,
    req: HttpRequest,
) -> impl Responder {
    let (org_id, dashboard_id) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let folder = crate::common::utils::http::get_folder(&query);
    let hash = query.get("hash").map(|h| h.as_str());
    dashboards::update_dashboard(&org_id, &dashboard_id, &folder, body, hash).await
}

/// ListDashboards
#[utoipa::path(
    context_path = "/api",
    tag = "Dashboards",
    operation_id = "ListDashboards",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = StatusCode::OK, body = Dashboards),
    ),
)]
#[get("/{org_id}/dashboards")]
async fn list_dashboards(org_id: web::Path<String>, req: HttpRequest) -> impl Responder {
    let folder = get_folder(req);
    dashboards::list_dashboards(&org_id.into_inner(), &folder).await
}

/// GetDashboard
#[utoipa::path(
    context_path = "/api",
    tag = "Dashboards",
    operation_id = "GetDashboard",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("dashboard_id" = String, Path, description = "Dashboard ID"),
    ),
    responses(
        (status = StatusCode::OK, body = Dashboard),
        (status = StatusCode::NOT_FOUND, description = "Dashboard not found", body = HttpResponse),
    ),
)]
#[get("/{org_id}/dashboards/{dashboard_id}")]
async fn get_dashboard(path: web::Path<(String, String)>, req: HttpRequest) -> impl Responder {
    let (org_id, dashboard_id) = path.into_inner();
    let folder = get_folder(req);
    dashboards::get_dashboard(&org_id, &dashboard_id, &folder).await
}

/// DeleteDashboard
#[utoipa::path(
    context_path = "/api",
    tag = "Dashboards",
    operation_id = "DeleteDashboard",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("dashboard_id" = String, Path, description = "Dashboard ID"),
    ),
    responses(
        (status = StatusCode::OK, description = "Success", body = HttpResponse),
        (status = StatusCode::NOT_FOUND, description = "NotFound", body = HttpResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Error", body = HttpResponse),
    ),
)]
#[delete("/{org_id}/dashboards/{dashboard_id}")]
async fn delete_dashboard(path: web::Path<(String, String)>, req: HttpRequest) -> impl Responder {
    let (org_id, dashboard_id) = path.into_inner();
    let folder_id = get_folder(req);
    dashboards::delete_dashboard(&org_id, &dashboard_id, &folder_id).await
}

/// MoveDashboard
#[utoipa::path(
    context_path = "/api",
    tag = "Dashboards",
    operation_id = "MoveDashboard",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("dashboard_id" = String, Path, description = "Dashboard ID"),
    ),
     request_body(
        content = MoveDashboard,
        description = "MoveDashboard details",
        example = json!({
            "from": "Source folder id",
            "to": "Destination folder id",
        }),
    ),
    responses(
        (status = StatusCode::OK, description = "Dashboard Moved", body = HttpResponse),
        (status = StatusCode::NOT_FOUND, description = "Dashboard not found", body = HttpResponse),
    ),
)]
#[put("/{org_id}/folders/dashboards/{dashboard_id}")]
async fn move_dashboard(
    path: web::Path<(String, String)>,
    folder: web::Json<MoveDashboard>,
) -> Result<HttpResponse, Error> {
    let (org_id, dashboard_id) = path.into_inner();
    if folder.from.is_empty() || folder.to.is_empty() {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::BAD_REQUEST.into(),
                "Please specify from & to folder from dashboard movement".to_string(),
            )),
        );
    };

    let resp =
        match dashboards::move_dashboard(&org_id, &dashboard_id, &folder.from, &folder.to).await {
            Ok(resp) => resp,
            Err(_) => HttpResponse::InternalServerError().into(),
        };
    Ok(resp)
}

fn get_folder(req: HttpRequest) -> String {
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    crate::common::utils::http::get_folder(&query)
}
