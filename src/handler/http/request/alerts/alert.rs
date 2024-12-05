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

use crate::{
    common::{
        meta::http::HttpResponse as MetaHttpResponse,
        utils::{auth::UserEmail, http::get_stream_type_from_request},
    },
    handler::http::models::alerts::{
        requests::{SaveAlertRequestBody, UpdateAlertRequestBody},
        responses::{
            EnableAlertResponseBody, GetAlertResponseBody, ListAlertsResponseBody,
            ListStreamAlertsResponseBody,
        },
    },
    service::alerts::alert,
};

/// CreateAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "SaveAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
      ),
    request_body(content = SaveAlertRequestBody, description = "Alert data", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/{stream_name}/alerts")]
pub async fn save_alert(
    path: web::Path<(String, String)>,
    req_body: web::Json<SaveAlertRequestBody>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();

    // Hack for frequency: convert minutes to seconds
    let mut alert: Alert = req_body.into_inner().into();
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
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
      ),
    request_body(content = UpdateAlertRequestBody, description = "Alert data", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/{stream_name}/alerts/{alert_name}")]
pub async fn update_alert(
    path: web::Path<(String, String, String)>,
    alert: web::Json<UpdateAlertRequestBody>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();

    // Hack for frequency: convert minutes to seconds
    let mut alert: Alert = alert.into_inner().into();
    alert.trigger_condition.frequency *= 60;
    alert.last_edited_by = Some(user_email.user_id);
    alert.updated_at = Some(datetime_now());
    match alert::save(&org_id, &stream_name, &name, alert, false).await {
        Ok(_) => Ok(MetaHttpResponse::ok("Alert Updated")),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// ListStreamAlerts
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListStreamAlerts",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = ListStreamAlertsResponseBody),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{stream_name}/alerts")]
async fn list_stream_alerts(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    let user_filter = query.get("owner").map(|v| v.to_string());
    let enabled_filter = query
        .get("enabled")
        .and_then(|field| match field.parse::<bool>() {
            Ok(value) => Some(value),
            Err(_) => None,
        });
    let alert_filter = AlertListFilter {
        owner: user_filter,
        enabled: enabled_filter,
    };
    match alert::list(
        &org_id,
        stream_type,
        Some(stream_name.as_str()),
        None,
        alert_filter,
    )
    .await
    {
        Ok(mut data) => {
            // Hack for frequency: convert seconds to minutes
            for alert in data.iter_mut() {
                alert.trigger_condition.frequency /= 60;
            }

            let resp_body: ListStreamAlertsResponseBody = data.into();
            Ok(MetaHttpResponse::json(resp_body))
        }
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
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = ListAlertsResponseBody),
    )
)]
#[get("/{org_id}/alerts")]
async fn list_alerts(path: web::Path<String>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();

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

    let user_filter = query.get("owner").map(|v| v.to_string());
    let enabled_filter = query
        .get("enabled")
        .and_then(|field| match field.parse::<bool>() {
            Ok(value) => Some(value),
            Err(_) => None,
        });
    let stream_type_filter = get_stream_type_from_request(&query).unwrap_or_default();
    let stream_name_filter = query.get("stream_name").map(|v| v.as_str());

    let alert_filter = AlertListFilter {
        owner: user_filter,
        enabled: enabled_filter,
    };
    match alert::list(
        &org_id,
        stream_type_filter,
        stream_name_filter,
        _alert_list_from_rbac,
        alert_filter,
    )
    .await
    {
        Ok(mut data) => {
            // Hack for frequency: convert seconds to minutes
            for alert in data.iter_mut() {
                alert.trigger_condition.frequency /= 60;
            }

            let resp_body: ListAlertsResponseBody = data.into();
            Ok(MetaHttpResponse::json(resp_body))
        }
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

/// GetAlertByName
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "GetAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("alert_name" = String, Path, description = "Alert name"),
      ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = GetAlertResponseBody),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{stream_name}/alerts/{alert_name}")]
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
        Ok(Some(mut data)) => {
            // Hack for frequency: convert seconds to minutes
            data.trigger_condition.frequency /= 60;

            let resp_body: GetAlertResponseBody = data.into();
            Ok(MetaHttpResponse::json(resp_body))
        }
        Ok(None) => Ok(MetaHttpResponse::not_found("")),
        Err(_) => Ok(MetaHttpResponse::internal_error("")),
    }
}

/// DeleteAlert
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
        (status = 200, description = "Success",  content_type = "application/json", body = EnableAlertResponseBody),
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

    match alert::enable(&org_id, stream_type, &stream_name, &name, enable).await {
        Ok(_) => {
            let resp_body = EnableAlertResponseBody { enabled: enable };
            Ok(MetaHttpResponse::json(resp_body))
        }
        Err(e) => match e {
            (http::StatusCode::NOT_FOUND, e) => Ok(MetaHttpResponse::not_found(e)),
            (_, e) => Ok(MetaHttpResponse::internal_error(e)),
        },
    }
}

/// TriggerAlert
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
