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

use std::collections::HashMap;

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse, Responder};
use config::{
    meta::dashboards::{v1, v2, v3, v4, v5, Dashboard, ListDashboardsParams, MoveDashboard},
    utils::json,
};
use serde::Deserialize;

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    service::dashboards::{self, DashboardError},
};

pub mod reports;

impl From<DashboardError> for HttpResponse {
    fn from(value: DashboardError) -> Self {
        match value {
            DashboardError::InfraError(err) => MetaHttpResponse::internal_error(err),
            DashboardError::DashboardNotFound => MetaHttpResponse::not_found("Dashboard not found"),
            DashboardError::UpdateMissingHash => MetaHttpResponse::internal_error("Request to update existing dashboard with missing or invalid hash value. BUG"),
            DashboardError::UpdateConflictingHash => MetaHttpResponse::conflict("Conflict: Failed to save due to concurrent changes. Please refresh the page after backing up your work to avoid losing changes."),
            DashboardError::PutMissingTitle => MetaHttpResponse::internal_error("Dashboard should have title"),
            DashboardError::MoveMissingFolderParam => MetaHttpResponse::bad_request("Please specify from & to folder from dashboard movement"),
            DashboardError::MoveDestinationFolderNotFound => MetaHttpResponse::not_found("Folder not found"),
            DashboardError::CreateFolderNotFound => MetaHttpResponse::not_found("Folder not found"),
            DashboardError::CreateDefaultFolder => MetaHttpResponse::internal_error("Error saving default folder"),
        }
    }
}

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
) -> impl Responder {
    let org_id = path.into_inner();
    let folder = get_folder(req);
    let dashboard = match parse_dashboard(body) {
        Ok(dashboard) => dashboard,
        Err(err) => return MetaHttpResponse::bad_request(err),
    };
    match dashboards::create_dashboard(&org_id, &folder, dashboard).await {
        Ok(dashboard) => HttpResponse::Ok().json(dashboard),
        Err(err) => err.into(),
    }
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
    let dashboard = match parse_dashboard(body) {
        Ok(dashboard) => dashboard,
        Err(err) => return MetaHttpResponse::bad_request(err),
    };
    match dashboards::update_dashboard(&org_id, &dashboard_id, &folder, dashboard, hash).await {
        Ok(dashboard) => HttpResponse::Ok().json(dashboard),
        Err(err) => err.into(),
    }
}

/// HTTP URL query component that contains parameters for listing dashboards.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
pub struct ListQuery {
    /// Optional folder ID filter parameter
    ///
    /// If neither `folder` nor any other filter parameter are set then this
    /// will search for all dashboards in the "default" folder.
    ///
    /// If `folder` is not set and another filter parameter, such as `title`, is
    /// set then this will search for dashboards in all folders.
    folder: Option<String>,

    /// The optional case-insensitive title substring with which to filter
    /// dashboards.
    title: Option<String>,
}

impl ListQuery {
    pub fn into(self, org_id: &str) -> ListDashboardsParams {
        match self {
            Self {
                folder: Some(f),
                title: Some(t),
            } => ListDashboardsParams::new(org_id)
                .with_folder_id(&f)
                .where_title_contains(&t),
            Self {
                folder: None,
                title: Some(t),
            } => ListDashboardsParams::new(org_id).where_title_contains(&t),
            Self {
                folder: Some(f),
                title: None,
            } => ListDashboardsParams::new(org_id).with_folder_id(&f),
            Self {
                folder: None,
                title: None,
            } => {
                // To preserve backwards-compatability when no filter parameters
                // are given we will list the contents of the default folder.
                ListDashboardsParams::new(org_id)
                    .with_folder_id(config::meta::folder::DEFAULT_FOLDER)
            }
        }
    }
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
        ListQuery
    ),
    responses(
        (status = StatusCode::OK, body = Dashboards),
    ),
)]
#[get("/{org_id}/dashboards")]
async fn list_dashboards(org_id: web::Path<String>, req: HttpRequest) -> impl Responder {
    let Ok(query) = web::Query::<ListQuery>::from_query(req.query_string()) else {
        return MetaHttpResponse::bad_request("Error parsing query parameters");
    };
    let params = query.into_inner().into(&org_id.into_inner());
    match dashboards::list_dashboards(params).await {
        Ok(dashboards) => {
            let list = config::meta::dashboards::Dashboards { dashboards };
            HttpResponse::Ok().json(list)
        }
        Err(err) => err.into(),
    }
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
    match dashboards::get_dashboard(&org_id, &dashboard_id, &folder).await {
        Ok(dashboard) => HttpResponse::Ok().json(dashboard),
        Err(err) => err.into(),
    }
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
    match dashboards::delete_dashboard(&org_id, &dashboard_id, &folder_id).await {
        Ok(()) => HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Dashboard deleted".to_string(),
        )),
        Err(err) => err.into(),
    }
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
) -> impl Responder {
    let (org_id, dashboard_id) = path.into_inner();
    match dashboards::move_dashboard(&org_id, &dashboard_id, &folder.from, &folder.to).await {
        Ok(()) => HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Dashboard moved".to_string(),
        )),
        Err(err) => err.into(),
    }
}

fn get_folder(req: HttpRequest) -> String {
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    crate::common::utils::http::get_folder(&query)
}

/// Parses the bytes into a dashboard with the given version.
fn parse_dashboard(bytes: web::Bytes) -> Result<Dashboard, serde_json::Error> {
    let d_version: DashboardVersion = json::from_slice(&bytes)?;
    let version = d_version.version;

    let dash = match version {
        1 => {
            let inner: v1::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        2 => {
            let inner: v2::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        3 => {
            let inner: v3::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        4 => {
            let inner: v4::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        _ => {
            let inner: v5::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
    };
    Ok(dash)
}

#[derive(Clone, Debug, Deserialize)]
struct DashboardVersion {
    #[serde(default = "default_version")]
    pub version: i32,
}

fn default_version() -> i32 {
    1
}
