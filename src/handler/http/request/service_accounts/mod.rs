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

use actix_web::{
    delete, get,
    http::{self},
    post, put, web, HttpResponse,
};
use config::utils::rand::generate_random_string;
#[cfg(feature = "enterprise")]
use {
    crate::service::self_reporting::audit, o2_enterprise::enterprise::common::auditor::AuditMessage,
};

use crate::{
    common::{
        meta::{
            self,
            service_account::ServiceAccountRequest,
            user::{SignInResponse, UpdateUser, UserRequest, UserRole},
        },
        utils::auth::UserEmail,
    },
    service::users,
};

/// ListUsers
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserList",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = UserList),
    )
)]
#[get("/{org_id}/service_accounts")]
pub async fn list(org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    users::list_users(&org_id, Some(UserRole::ServiceAccount)).await
}

/// CreateUser
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserSave",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = UserRequest, description = "User data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/service_accounts")]
pub async fn save(
    org_id: web::Path<String>,
    service_account: web::Json<ServiceAccountRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let initiator_id = user_email.user_id;
    let service_account = service_account.into_inner();
    let user = UserRequest {
        email: service_account.email.trim().to_string(),
        first_name: service_account.first_name.trim().to_string(),
        last_name: service_account.last_name.trim().to_string(),
        password: generate_random_string(16),
        role: meta::user::UserRole::ServiceAccount,
        is_external: false,
    };

    users::post_user(&org_id, user, &initiator_id).await
}

/// UpdateUser
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserUpdate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "User's email id"),
    ),
    request_body(content = UpdateUser, description = "User data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/service_accounts/{email_id}")]
pub async fn update(
    params: web::Path<(String, String)>,
    service_account: web::Json<ServiceAccountRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = params.into_inner();
    let email_id = email_id.trim().to_string();

    let service_account = service_account.into_inner();
    if service_account.eq(&ServiceAccountRequest::default()) {
        return Ok(
            HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Please specify appropriate fields to update service account".to_string(),
            )),
        );
    }

    let user = UpdateUser {
        change_password: false,
        first_name: Some(service_account.first_name.trim().to_string()),
        last_name: Some(service_account.last_name.trim().to_string()),
        old_password: None,
        new_password: None,
        role: None,
        token: None,
    };
    let initiator_id = &user_email.user_id;

    users::update_user(&org_id, &email_id, false, initiator_id, user).await
}

/// RemoveUserFromOrganization
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "RemoveUserFromOrg",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "User name"),
      ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/service_accounts/{email_id}")]
pub async fn delete(
    path: web::Path<(String, String)>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = path.into_inner();
    let initiator_id = user_email.user_id;
    users::remove_user_from_org(&org_id, &email_id, &initiator_id).await
}

fn unauthorized_error(mut resp: SignInResponse) -> Result<HttpResponse, Error> {
    resp.status = false;
    resp.message = "Invalid credentials".to_string();
    Ok(HttpResponse::Unauthorized().json(resp))
}

#[cfg(feature = "enterprise")]
async fn audit_unauthorized_error(mut audit_message: AuditMessage) {
    use chrono::Utc;

    audit_message._timestamp = Utc::now().timestamp_micros();
    audit_message.response_code = 401;
    // Even if the user_email of audit_message is not set, still the event should be audited
    audit(audit_message).await;
}
