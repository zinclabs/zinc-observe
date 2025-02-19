// Copyright (c) 2025. OpenObserve Inc.
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

use std::io::Error;

use actix_http::header::HeaderMap;
use actix_web::{get, post, web, HttpResponse};
use itertools::Itertools;
use o2_enterprise::enterprise::actions::{action_manager::trigger_action, meta::TriggerSource};
use tracing::{span, Level};

use crate::{
    common::utils::{
        auth::{check_permissions, UserEmail},
        http::get_or_create_trace_id,
    },
    handler::http::models::action::TestActionRequest,
};

/// Test Action
#[utoipa::path(
    context_path = "/api",
    tag = "Actions",
    operation_id = "InvokeAction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("action_id" = String, Path, description = "Action ID"),
    ),
    request_body(content = Template, description = "Template data", content_type =
"application/json"),     responses(
        (status = 200, description = "Success", content_type = "application/json", body =
HttpResponse),         (status = 400, description = "Error",   content_type = "application/json",
body = HttpResponse),     )
)]
#[post("/{org_id}/actions/test/{action_id}")]
pub async fn test_action(
    path: web::Path<(String, String)>,
    req: web::Json<TestActionRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, action_id) = path.into_inner();
    let mut req = req.into_inner();
    let inputs = req.inputs.drain(..).collect_vec();

    if !check_permissions(
        Some(action_id.clone()),
        &org_id,
        &user_email.user_id,
        "actions",
        "POST",
    )
    .await
    {
        return Ok(HttpResponse::Forbidden().body("Unauthorized Access"));
    }

    let trace_id = get_or_create_trace_id(&HeaderMap::new(), &span!(Level::TRACE, "action_test"));
    match trigger_action(
        &trace_id,
        &org_id,
        &action_id,
        inputs,
        TriggerSource::Manual,
    )
    .await
    {
        Ok(action) => Ok(crate::common::meta::http::HttpResponse::json(action)),
        Err(e) => Ok(crate::common::meta::http::HttpResponse::bad_request(e)),
    }
}

/// Pause Action
#[utoipa::path(
    context_path = "/api",
    tag = "Actions",
    operation_id = "PauseAction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("action_id" = String, Path, description = "Action ID"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body =
HttpResponse),         (status = 400, description = "Error",   content_type = "application/json",
body = HttpResponse),     )
)]
#[get("/{org_id}/actions/pause/{action_id}")]
pub async fn pause_action(_path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    unimplemented!()
}

/// Resume Action
#[utoipa::path(
    context_path = "/api",
    tag = "Actions",
    operation_id = "ResumeAction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("action_id" = String, Path, description = "Action ID"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body =
HttpResponse),         (status = 400, description = "Error",   content_type = "application/json",
body = HttpResponse),     )
)]
#[get("/{org_id}/actions/resume/{action_id}")]
pub async fn resume_action(_path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    unimplemented!()
}
