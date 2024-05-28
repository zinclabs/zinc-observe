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

use std::io::Error;

use actix_web::{
    cookie, delete, get,
    http::{self},
    post, put, web, HttpRequest, HttpResponse,
};
use chrono::Utc;
use config::{
    utils::{base64, json},
    CONFIG,
};
use strum::IntoEnumIterator;

use crate::{
    common::{
        meta::{
            self,
            user::{
                AuthTokens, RolesResponse, SignInResponse, SignInUser, UpdateUser, UserOrgRole,
                UserRequest, UserRole,
            },
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
#[get("/{org_id}/users")]
pub async fn list(org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    users::list_users(&org_id).await
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
#[post("/{org_id}/users")]
pub async fn save(
    org_id: web::Path<String>,
    user: web::Json<UserRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let initiator_id = user_email.user_id;
    let mut user = user.into_inner();
    user.email = user.email.trim().to_string();

    if user.role.eq(&meta::user::UserRole::Root) {
        return Ok(
            HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Not allowed".to_string(),
            )),
        );
    }
    #[cfg(not(feature = "enterprise"))]
    {
        user.role = meta::user::UserRole::Admin;
    }
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
#[put("/{org_id}/users/{email_id}")]
pub async fn update(
    params: web::Path<(String, String)>,
    user: web::Json<UpdateUser>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = params.into_inner();
    let email_id = email_id.trim().to_string();
    #[cfg(not(feature = "enterprise"))]
    let mut user = user.into_inner();
    #[cfg(feature = "enterprise")]
    let user = user.into_inner();
    if user.eq(&UpdateUser::default()) {
        return Ok(
            HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Please specify appropriate fields to update user".to_string(),
            )),
        );
    }
    #[cfg(not(feature = "enterprise"))]
    {
        user.role = Some(meta::user::UserRole::Admin);
    }
    let initiator_id = &user_email.user_id;
    let self_update = user_email.user_id.eq(&email_id);
    users::update_user(&org_id, &email_id, self_update, initiator_id, user).await
}

/// AddUserToOrganization
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "AddUserToOrg",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "User's email id"),
    ),
    request_body(content = UserOrgRole, description = "User role", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/users/{email_id}")]
pub async fn add_user_to_org(
    params: web::Path<(String, String)>,
    _role: web::Json<UserOrgRole>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = params.into_inner();
    // let role = role.into_inner().role;
    let role = meta::user::UserRole::Admin;
    let initiator_id = user_email.user_id;
    users::add_user_to_org(&org_id, &email_id, role, &initiator_id).await
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
#[delete("/{org_id}/users/{email_id}")]
pub async fn delete(
    path: web::Path<(String, String)>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = path.into_inner();
    let initiator_id = user_email.user_id;
    users::remove_user_from_org(&org_id, &email_id, &initiator_id).await
}

/// AuthenticateUser
#[utoipa::path(
    context_path = "/auth",
    tag = "Auth",
    operation_id = "UserLoginCheck",
    request_body(content = SignInUser, description = "User login", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SignInResponse),
    )
)]
#[post("/login")]
pub async fn authentication(
    auth: Option<web::Json<SignInUser>>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    use o2_enterprise::enterprise::common::infra::config::O2_CONFIG;
    #[cfg(feature = "enterprise")]
    let native_login_enabled = O2_CONFIG.dex.native_login_enabled;
    #[cfg(not(feature = "enterprise"))]
    let native_login_enabled = true;

    if !native_login_enabled {
        return Ok(HttpResponse::Forbidden().json("Not Supported"));
    }

    let mut resp = SignInResponse::default();
    let auth = match auth {
        Some(auth) => auth.into_inner(),
        None => {
            // get Authorization header from request
            #[cfg(feature = "enterprise")]
            {
                let auth_header = _req.headers().get("Authorization");
                if auth_header.is_some() {
                    let auth_header = auth_header.unwrap().to_str().unwrap();
                    if let Some((name, password)) =
                        o2_enterprise::enterprise::dex::service::auth::get_user_from_token(
                            auth_header,
                        )
                    {
                        SignInUser { name, password }
                    } else {
                        return unauthorized_error(resp);
                    }
                } else {
                    return unauthorized_error(resp);
                }
            }
            #[cfg(not(feature = "enterprise"))]
            {
                return unauthorized_error(resp);
            }
        }
    };

    match crate::handler::http::auth::validator::validate_user(&auth.name, &auth.password).await {
        Ok(v) => {
            if v.is_valid {
                resp.status = true;
            } else {
                return unauthorized_error(resp);
            }
        }
        Err(_e) => {
            return unauthorized_error(resp);
        }
    };
    if resp.status {
        let access_token = format!(
            "Basic {}",
            base64::encode(&format!("{}:{}", auth.name, auth.password))
        );
        let tokens = json::to_string(&AuthTokens {
            access_token,
            refresh_token: "".to_string(),
        })
        .unwrap();
        let mut auth_cookie = cookie::Cookie::new("auth_tokens", tokens);
        auth_cookie.set_expires(
            cookie::time::OffsetDateTime::now_utc()
                + cookie::time::Duration::seconds(CONFIG.auth.cookie_max_age),
        );
        auth_cookie.set_http_only(true);
        auth_cookie.set_secure(CONFIG.auth.cookie_secure_only);
        auth_cookie.set_path("/");
        if CONFIG.auth.cookie_same_site_lax {
            auth_cookie.set_same_site(cookie::SameSite::Lax);
        } else {
            auth_cookie.set_same_site(cookie::SameSite::None);
        }
        Ok(HttpResponse::Ok().cookie(auth_cookie).json(resp))
    } else {
        unauthorized_error(resp)
    }
}

#[get("/login")]
pub async fn get_auth(_req: HttpRequest) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        use actix_web::http::header;

        use crate::handler::http::auth::validator::ID_TOKEN_HEADER;

        let mut resp = SignInResponse::default();

        let query = web::Query::<std::collections::HashMap<String, String>>::from_query(
            _req.query_string(),
        )
        .unwrap();

        let mut req_time = None;
        let mut exp_in = 300;
        let req_ts;

        let auth_header = if let Some(s) = query.get("auth") {
            match query.get("request_time") {
                Some(req_time_str) => {
                    if let Ok(ts) = config::utils::time::parse_str_to_time(req_time_str) {
                        req_ts = ts.timestamp();
                    } else {
                        return unauthorized_error(resp);
                    }
                    req_time = Some(req_time_str);
                }
                None => {
                    return unauthorized_error(resp);
                }
            };
            match query.get("exp_in") {
                Some(exp_in_str) => {
                    exp_in = exp_in_str.parse::<i64>().unwrap();
                }
                None => {
                    return unauthorized_error(resp);
                }
            };
            if Utc::now().timestamp() - req_ts > exp_in {
                return unauthorized_error(resp);
            }
            format!("q_auth {}", s)
        } else if let Some(auth_header) = _req.headers().get("Authorization") {
            match auth_header.to_str() {
                Ok(auth_header_str) => auth_header_str.to_string(),
                Err(_) => {
                    return unauthorized_error(resp);
                }
            }
        } else {
            return unauthorized_error(resp);
        };

        if let Some((name, password)) =
            o2_enterprise::enterprise::dex::service::auth::get_user_from_token(&auth_header)
        {
            match crate::handler::http::auth::validator::validate_user_for_query_params(
                &name, &password, req_time, exp_in,
            )
            .await
            {
                Ok(v) => {
                    if v.is_valid {
                        resp.status = true;
                    } else {
                        return unauthorized_error(resp);
                    }
                }
                Err(_) => {
                    return unauthorized_error(resp);
                }
            };

            if resp.status {
                let access_token = format!(
                    "Basic {}",
                    base64::encode(&format!("{}:{}", &name, &password))
                );

                let id_token = config::utils::json::json!({
                    "email": name,
                    "name": name,
                });
                let tokens = json::to_string(&AuthTokens {
                    access_token,
                    refresh_token: "".to_string(),
                })
                .unwrap();

                let mut auth_cookie = cookie::Cookie::new("auth_tokens", tokens);
                auth_cookie.set_expires(
                    cookie::time::OffsetDateTime::now_utc()
                        + cookie::time::Duration::seconds(CONFIG.auth.cookie_max_age),
                );
                auth_cookie.set_http_only(true);
                auth_cookie.set_secure(CONFIG.auth.cookie_secure_only);
                auth_cookie.set_path("/");

                if CONFIG.auth.cookie_same_site_lax {
                    auth_cookie.set_same_site(cookie::SameSite::Lax);
                } else {
                    auth_cookie.set_same_site(cookie::SameSite::None);
                }
                let url = format!(
                    "{}{}/web/cb#id_token={}.{}",
                    CONFIG.common.web_url,
                    CONFIG.common.base_uri,
                    ID_TOKEN_HEADER,
                    base64::encode(&id_token.to_string())
                );
                return Ok(HttpResponse::Found()
                    .append_header((header::LOCATION, url))
                    .cookie(auth_cookie)
                    .json(resp));
            } else {
                unauthorized_error(resp)
            }
        } else {
            unauthorized_error(resp)
        }
    }

    #[cfg(not(feature = "enterprise"))]
    {
        Ok(HttpResponse::Forbidden().json("Not Supported"))
    }
}

/// ListUsers
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserRoles",
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
#[get("/{org_id}/users/roles")]
pub async fn list_roles(_org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let roles = UserRole::iter()
        .filter_map(|role| {
            if role.eq(&UserRole::Root) || role.eq(&UserRole::Member) {
                None
            } else {
                Some(RolesResponse {
                    label: role.get_label(),
                    value: role.to_string(),
                })
            }
        })
        .collect::<Vec<RolesResponse>>();

    Ok(HttpResponse::Ok().json(roles))
}

fn unauthorized_error(mut resp: SignInResponse) -> Result<HttpResponse, Error> {
    resp.status = false;
    resp.message = "Invalid credentials".to_string();
    Ok(HttpResponse::Unauthorized().json(resp))
}
