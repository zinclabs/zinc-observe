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

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse};
use config::meta::{
    alerts::alert::{Alert, AlertListFilter},
    dashboards::datetime_now,
};
use svix_ksuid::Ksuid;

use crate::{
    common::{
        meta::http::HttpResponse as MetaHttpResponse,
        utils::{auth::UserEmail, http::get_stream_type_from_request},
    },
    handler::http::models::alerts::{
        requests::{ListAlertsQuery, ListAlertsQueryStreamParams},
        responses::{ListAlertsResponseBody, ListAlertsResponseBodyItem},
    },
    service::alerts::alert,
};

/// CreateAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "CreateAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    request_body(content = Alert, description = "Alert data", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/alerts")]
pub async fn create_alert(
    path: web::Path<String>,
    alert: web::Json<Alert>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let mut alert = alert.into_inner();
    let stream_name = alert.stream_name.clone();

    // Hack for frequency: convert minutes to seconds
    alert.trigger_condition.frequency *= 60;
    alert.owner = Some(user_email.user_id.clone());
    alert.last_edited_by = Some(user_email.user_id);
    alert.updated_at = Some(datetime_now());
    alert.last_triggered_at = None;
    alert.last_satisfied_at = None;

    match alert::save(&org_id, &stream_name, "", alert, true).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert saved")),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// UpdateAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "UpdateAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
      ),
    request_body(content = Alert, description = "Alert data", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/alerts/{alert_id}")]
pub async fn update_alert(
    path: web::Path<(String, Ksuid)>,
    alert: web::Json<Alert>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, _alert_id) = path.into_inner();

    // Hack for frequency: convert minutes to seconds
    let mut alert = alert.into_inner();
    let stream_name = alert.stream_name.clone();
    let alert_name = alert.name.clone();
    alert.trigger_condition.frequency *= 60;
    alert.last_edited_by = Some(user_email.user_id);
    alert.updated_at = Some(datetime_now());
    match alert::save(&org_id, &stream_name, &alert_name, alert, false).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert Updated")),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// ListAlerts
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListAlerts",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ListAlertsQuery
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/alerts")]
async fn list_alerts(path: web::Path<String>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let Ok(query) = web::Query::<ListAlertsQuery>::from_query(req.query_string()) else {
        return Ok(MetaHttpResponse::bad_request(
            "Error parsing query parameters",
        ));
    };
    let query = query.0;

    let mut _alert_list_from_rbac = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            "alert",
        )
        .await
        {
            Ok(stream_list) => {
                _alert_list_from_rbac = stream_list;
            }
            Err(e) => {
                return Ok(crate::common::meta::http::HttpResponse::forbidden(
                    e.to_string(),
                ));
            }
        }
        // Get List of allowed objects ends
    }

    let user_filter = query.owner;
    let enabled_filter = query.enabled;
    let (stream_type_filter, stream_name_filter) = match query.stream {
        Some(ListAlertsQueryStreamParams {
            stream_type,
            stream_name,
        }) => (Some(stream_type.into()), stream_name),
        None => (None, None),
    };

    let alert_filter = AlertListFilter {
        owner: user_filter,
        enabled: enabled_filter,
    };
    match alert::list(
        &org_id,
        stream_type_filter,
        stream_name_filter.as_deref(),
        _alert_list_from_rbac,
        alert_filter,
    )
    .await
    {
        Ok(mut data) => {
            // Hack for frequency: convert seconds to minutes
            for (_folder, alert) in data.iter_mut() {
                alert.trigger_condition.frequency /= 60;
            }

            let items_rslt: Result<Vec<_>, _> = data
                .into_iter()
                .map(ListAlertsResponseBodyItem::try_from)
                .collect();
            let Ok(items) = items_rslt else {
                return Ok(MetaHttpResponse::internal_error("Unexpected error"));
            };
            let resp_body = ListAlertsResponseBody { list: items };
            Ok(MetaHttpResponse::json(resp_body))
        }
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// GetAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "GetAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
      ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = Alert),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{alert_id}")]
async fn get_alert(
    path: web::Path<(String, String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    match alert::get(&org_id, stream_type, &stream_name, &name).await {
        Ok(mut data) => {
            // Hack for frequency: convert seconds to minutes
            if let Some(ref mut data) = data {
                data.trigger_condition.frequency /= 60;
            }
            Ok(MetaHttpResponse::json(data))
        }
        Err(e) => Ok(MetaHttpResponse::not_found(e)),
    }
}

/// DeleteAlert
#[deprecated]
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "DeleteAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/{stream_name}/alerts/{alert_name}")]
async fn delete_alert(
    path: web::Path<(String, String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    match alert::delete(&org_id, stream_type, &stream_name, &name).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert deleted")),
        Err(e) => match e {
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// EnableAlert
#[deprecated]
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "EnableAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
        ("value" = bool, Query, description = "Enable or disable alert"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/{stream_name}/alerts/{alert_name}/enable")]
async fn enable_alert(
    path: web::Path<(String, String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    let enable = match query.get("value") {
        Some(v) => v.parse::<bool>().unwrap_or_default(),
        None => false,
    };
    let mut resp = HashMap::new();
    resp.insert("enabled".to_string(), enable);
    match alert::enable(&org_id, stream_type, &stream_name, &name, enable).await {
        Ok(_) => Ok(MetaHttpResponse::json(resp)),
        Err(e) => match e {
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// TriggerAlert
#[deprecated]
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "TriggerAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/{stream_name}/alerts/{alert_name}/trigger")]
async fn trigger_alert(
    path: web::Path<(String, String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    match alert::trigger(&org_id, stream_type, &stream_name, &name).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert triggered")),
        Err(e) => match e {
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}
