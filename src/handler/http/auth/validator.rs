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

use actix_web::{
    dev::ServiceRequest,
    error::{ErrorForbidden, ErrorUnauthorized},
    http::{header, Method},
    web, Error,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use config::{get_config, utils::base64};

use crate::{
    common::{
        meta::{
            ingestion::INGESTION_EP,
            user::{AuthTokensExt, DBUser, TokenValidationResponse, UserRole},
        },
        utils::auth::{get_hash, is_root_user, AuthExtractor},
    },
    service::{db, users},
};

pub const PKCE_STATE_ORG: &str = "o2_pkce_state";
pub const ACCESS_TOKEN: &str = "access_token";
pub const REFRESH_TOKEN: &str = "refresh_token";
pub const ID_TOKEN_HEADER: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

pub async fn validator(
    req: ServiceRequest,
    user_id: &str,
    password: &str,
    auth_info: AuthExtractor,
    path_prefix: &str,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let cfg = get_config();
    let path = match req
        .request()
        .path()
        .strip_prefix(format!("{}{}", cfg.common.base_uri, path_prefix).as_str())
    {
        Some(path) => path,
        None => req.request().path(),
    };
    match validate_credentials(user_id, password.trim(), path).await {
        Ok(res) => {
            if res.is_valid {
                // / Hack for prometheus, need support POST and check the header
                let mut req = req;
                if req.method().eq(&Method::POST) && !req.headers().contains_key("content-type") {
                    req.headers_mut().insert(
                        header::CONTENT_TYPE,
                        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                }
                req.headers_mut().insert(
                    header::HeaderName::from_static("user_id"),
                    header::HeaderValue::from_str(&res.user_email).unwrap(),
                );

                if auth_info.bypass_check
                    || check_permissions(user_id, auth_info, res.user_role).await
                {
                    Ok(req)
                } else {
                    Err((ErrorForbidden("Unauthorized Access"), req))
                }
            } else {
                Err((ErrorUnauthorized("Unauthorized Access"), req))
            }
        }
        Err(err) => Err((err, req)),
    }
}

/// `validate_token` validates the endpoints which are token only.
/// This includes endpoints like `rum` etc.
///
/// ### Args:
/// - token: The token to validate
///  
pub async fn validate_token(token: &str, org_id: &str) -> Result<bool, Error> {
    match users::get_user_by_token(org_id, token).await {
        Some(_user) => Ok(true),
        None => Err(ErrorForbidden("Not allowed")),
    }
}

pub async fn validate_credentials(
    user_id: &str,
    user_password: &str,
    path: &str,
) -> Result<TokenValidationResponse, Error> {
    let user;
    let mut path_columns = path.split('/').collect::<Vec<&str>>();
    if let Some(v) = path_columns.last() {
        if v.is_empty() {
            path_columns.pop();
        }
    }

    // this is only applicable for super admin user
    if is_root_user(user_id) {
        user = users::get_user(None, user_id).await;
        if user.is_none() {
            return Ok(TokenValidationResponse {
                is_valid: false,
                user_email: "".to_string(),
                is_internal_user: false,
                user_role: None,
                user_name: "".to_string(),
                family_name: "".to_string(),
                given_name: "".to_string(),
            });
        }
    } else if path_columns.last().unwrap_or(&"").eq(&"organizations") {
        let db_user = db::user::get_db_user(user_id).await;
        user = match db_user {
            Ok(user) => {
                let all_users = user.get_all_users();
                if all_users.is_empty() {
                    None
                } else {
                    all_users.first().cloned()
                }
            }
            Err(_) => None,
        }
    } else {
        user = match path.find('/') {
            Some(index) => {
                let org_id = &path[0..index];
                users::get_user(Some(org_id), user_id).await
            }
            None => users::get_user(None, user_id).await,
        }
    };

    if user.is_none() {
        return Ok(TokenValidationResponse {
            is_valid: false,
            user_email: "".to_string(),
            is_internal_user: false,
            user_role: None,
            user_name: "".to_string(),
            family_name: "".to_string(),
            given_name: "".to_string(),
        });
    }
    let user = user.unwrap();

    if (path_columns.len() == 1 || INGESTION_EP.iter().any(|s| path_columns.contains(s)))
        && user.token.eq(&user_password)
    {
        return Ok(TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: Some(user.role),
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        });
    }

    let in_pass = get_hash(user_password, &user.salt);
    if !user.password.eq(&in_pass)
        && !user
            .password_ext
            .unwrap_or("".to_string())
            .eq(&user_password)
    {
        return Ok(TokenValidationResponse {
            is_valid: false,
            user_email: "".to_string(),
            is_internal_user: false,
            user_role: None,
            user_name: "".to_string(),
            family_name: "".to_string(),
            given_name: "".to_string(),
        });
    }
    if !path.contains("/user")
        || (path.contains("/user")
            && (user.role.eq(&UserRole::Admin)
                || user.role.eq(&UserRole::Root)
                || user.email.eq(user_id)))
    {
        Ok(TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: Some(user.role),
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        })
    } else {
        Err(ErrorForbidden("Not allowed"))
    }
}

async fn validate_user_from_db(
    db_user: Result<DBUser, anyhow::Error>,
    user_password: &str,
    req_time: Option<&String>,
    exp_in: i64,
    password_ext_salt: &str,
) -> Result<TokenValidationResponse, Error> {
    // let db_user = db::user::get_db_user(user_id).await;
    match db_user {
        Ok(mut user) => {
            let in_pass = get_hash(user_password, &user.salt);
            if req_time.is_none() && user.password.eq(&in_pass) {
                if user.password_ext.is_none() {
                    let password_ext = get_hash(user_password, password_ext_salt);
                    user.password_ext = Some(password_ext);
                    let _ = db::user::set(&user).await;
                }
                Ok(TokenValidationResponse {
                    is_valid: true,
                    user_email: user.email,
                    is_internal_user: !user.is_external,
                    user_role: None,
                    user_name: user.first_name.to_owned(),
                    family_name: user.last_name,
                    given_name: user.first_name,
                })
            } else if user.password_ext.is_some() && req_time.is_some() {
                let hashed_pass = get_hash(
                    &format!(
                        "{}{}",
                        get_hash(
                            &format!("{}{}", user.password_ext.unwrap(), req_time.unwrap()),
                            password_ext_salt
                        ),
                        exp_in
                    ),
                    password_ext_salt,
                );
                if hashed_pass.eq(&user_password) {
                    return Ok(TokenValidationResponse {
                        is_valid: true,
                        user_email: user.email,
                        is_internal_user: !user.is_external,
                        user_role: None,
                        user_name: user.first_name.to_owned(),
                        family_name: user.last_name,
                        given_name: user.first_name,
                    });
                } else {
                    Err(ErrorForbidden("Not allowed"))
                }
            } else {
                Err(ErrorForbidden("Not allowed"))
            }
        }
        Err(_) => Err(ErrorForbidden("Not allowed")),
    }
}

pub async fn validate_user(
    user_id: &str,
    user_password: &str,
) -> Result<TokenValidationResponse, Error> {
    let db_user = db::user::get_db_user(user_id).await;
    validate_user_from_db(db_user, user_password, None, 0, &get_salt()).await
}

pub async fn validate_user_for_query_params(
    user_id: &str,
    user_password: &str,
    req_time: Option<&String>,
    exp_in: i64,
) -> Result<TokenValidationResponse, Error> {
    let db_user = db::user::get_db_user(user_id).await;

    validate_user_from_db(db_user, user_password, req_time, exp_in, &get_salt()).await
}

pub async fn validator_aws(
    req: ServiceRequest,
    _credentials: Option<BasicAuth>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let cfg = get_config();
    let path = req
        .request()
        .path()
        .strip_prefix(format!("{}/aws/", cfg.common.base_uri).as_str())
        .unwrap_or(req.request().path());

    match req.headers().get("X-Amz-Firehose-Access-Key") {
        Some(val) => match val.to_str() {
            Ok(val) => {
                let amz_creds = match base64::decode(val) {
                    Ok(val) => val,
                    Err(_) => return Err((ErrorUnauthorized("Unauthorized Access"), req)),
                };
                let creds = amz_creds
                    .split(':')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                match validate_credentials(&creds[0], &creds[1], path).await {
                    Ok(res) => {
                        if res.is_valid {
                            let mut req = req;
                            req.headers_mut().insert(
                                header::HeaderName::from_static("user_id"),
                                header::HeaderValue::from_str(&res.user_email).unwrap(),
                            );
                            Ok(req)
                        } else {
                            Err((ErrorUnauthorized("Unauthorized Access"), req))
                        }
                    }
                    Err(err) => Err((err, req)),
                }
            }
            Err(_) => Err((ErrorUnauthorized("Unauthorized Access"), req)),
        },
        None => Err((ErrorUnauthorized("Unauthorized Access"), req)),
    }
}

pub async fn validator_gcp(
    req: ServiceRequest,
    _credentials: Option<BasicAuth>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let cfg = get_config();
    let path = req
        .request()
        .path()
        .strip_prefix(format!("{}/gcp/", cfg.common.base_uri).as_str())
        .unwrap_or(req.request().path());

    let query =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .unwrap();
    match query.get("API-Key") {
        Some(val) => {
            let gcp_creds = base64::decode(val).unwrap();
            let creds = gcp_creds
                .split(':')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            match validate_credentials(&creds[0], &creds[1], path).await {
                Ok(res) => {
                    if res.is_valid {
                        let mut req = req;
                        req.headers_mut().insert(
                            header::HeaderName::from_static("user_id"),
                            header::HeaderValue::from_str(&res.user_email).unwrap(),
                        );
                        Ok(req)
                    } else {
                        Err((ErrorUnauthorized("Unauthorized Access"), req))
                    }
                }
                Err(err) => Err((err, req)),
            }
        }
        None => Err((ErrorUnauthorized("Unauthorized Access"), req)),
    }
}

pub async fn validator_rum(
    req: ServiceRequest,
    _credentials: Option<BasicAuth>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let path = req
        .request()
        .path()
        .strip_prefix(format!("{}/rum/v1/", get_config().common.base_uri).as_str())
        .unwrap_or(req.request().path());

    // After this previous path clean we should get only the
    // remaining `org_id/rum` or `org_id/replay` or `org_id/logs`
    let org_id_end_point: Vec<&str> = path.split('/').collect();
    if org_id_end_point.len() != 2 {
        return Err((
            ErrorUnauthorized("Unauthorized Access. Please pass a valid org_id."),
            req,
        ));
    }

    let query =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .unwrap();

    let token = query.get("oo-api-key").or_else(|| query.get("o2-api-key"));
    match token {
        Some(token) => match validate_token(token, org_id_end_point[0]).await {
            Ok(res) => {
                if res {
                    Ok(req)
                } else {
                    Err((ErrorUnauthorized("Unauthorized Access"), req))
                }
            }
            Err(err) => Err((err, req)),
        },
        None => Err((ErrorUnauthorized("Unauthorized Access"), req)),
    }
}

async fn oo_validator_internal(
    req: ServiceRequest,
    auth_info: AuthExtractor,
    path_prefix: &str,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    if auth_info.auth.starts_with("Basic") {
        let decoded = base64::decode(auth_info.auth.strip_prefix("Basic").unwrap().trim())
            .expect("Failed to decode base64 string");

        let credentials = String::from_utf8(decoded.into())
            .map_err(|_| ())
            .expect("Failed to decode base64 string");
        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err((ErrorUnauthorized("Unauthorized Access"), req));
        }
        let (username, password) = (parts[0], parts[1]);
        let username = username.to_owned();
        let password = password.to_owned();
        validator(req, &username, &password, auth_info, path_prefix).await
    } else if auth_info.auth.starts_with("Bearer") {
        super::token::token_validator(req, auth_info).await
    } else if auth_info.auth.starts_with("auth_ext") {
        let auth_tokens: AuthTokensExt =
            config::utils::json::from_str(&auth_info.auth).unwrap_or_default();
        if chrono::Utc::now().timestamp() - auth_tokens.request_time > auth_tokens.expires_in {
            Err((ErrorUnauthorized("Unauthorized Access"), req))
        } else {
            Err((ErrorUnauthorized("Unauthorized Access"), req))
        }
    } else {
        Err((ErrorUnauthorized("Unauthorized Access"), req))
    }
}

/// Validates the authentication information in the incoming request and returns the request if
/// valid, or an error if invalid.
///
/// This function is responsible for validating the authentication information in the incoming
/// request. It supports both Basic and Bearer authentication (in enterprise).
/// Works exclusively on `/api` prefix
///
/// For Basic authentication, it decodes the base64-encoded credentials, splits them into username
/// and password, and calls the `validator` function to validate the credentials.
///
/// For Bearer authentication, it calls the `token_validator` function to validate the token.
///
/// If the authentication is invalid, it returns an `ErrorUnauthorized` error.
pub async fn oo_validator(
    req: ServiceRequest,
    auth_info: AuthExtractor,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let path_prefix = "/api/";
    oo_validator_internal(req, auth_info, path_prefix).await
}

/// Validates the authentication information in the request and returns the request if valid, or an
/// error if invalid.
///
/// This function is a proxy for the `oo_validator_internal` function, setting the `path_prefix` to
/// "/proxy/".
///
/// # Arguments
/// * `req` - The `ServiceRequest` to validate.
/// * `auth_info` - The authentication information extracted from the request.
///
/// # Returns
/// * `Result<ServiceRequest, (Error, ServiceRequest)>` - The validated request, or an error if the
///   authentication is invalid.
pub async fn validator_proxy_url(
    req: ServiceRequest,
    auth_info: AuthExtractor,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let path_prefix = "/proxy/";
    oo_validator_internal(req, auth_info, path_prefix).await
}

#[cfg(feature = "enterprise")]
pub(crate) async fn check_permissions(
    user_id: &str,
    auth_info: AuthExtractor,
    role: Option<UserRole>,
) -> bool {
    use o2_enterprise::enterprise::common::infra::config::O2_CONFIG;

    if !O2_CONFIG.openfga.enabled {
        return true;
    }

    let object_str = auth_info.o2_type;
    let obj_str = if object_str.contains("##user_id##") {
        object_str.replace("##user_id##", user_id)
    } else {
        object_str
    };
    let role = match role {
        Some(role) => {
            if role.eq(&UserRole::Root) {
                // root user should have access to everything , bypass check in openfga
                return true;
            } else {
                format!("{role}")
            }
        }
        None => "".to_string(),
    };
    let org_id = if auth_info.org_id.eq("organizations") {
        user_id
    } else {
        &auth_info.org_id
    };

    o2_enterprise::enterprise::openfga::authorizer::authz::is_allowed(
        org_id,
        user_id,
        &auth_info.method,
        &obj_str,
        &auth_info.parent_id,
        &role,
    )
    .await
}

#[cfg(not(feature = "enterprise"))]
pub(crate) async fn check_permissions(
    _user_id: &str,
    _auth_info: AuthExtractor,
    _role: Option<UserRole>,
) -> bool {
    true
}

#[cfg(feature = "enterprise")]
async fn list_objects(
    user_id: &str,
    permission: &str,
    object_type: &str,
) -> Result<Vec<String>, anyhow::Error> {
    o2_enterprise::enterprise::openfga::authorizer::authz::list_objects(
        user_id,
        permission,
        object_type,
    )
    .await
}

#[cfg(feature = "enterprise")]
pub(crate) async fn list_objects_for_user(
    _org_id: &str,
    user_id: &str,
    permission: &str,
    object_type: &str,
) -> Result<Option<Vec<String>>, Error> {
    use o2_enterprise::enterprise::common::infra::config::O2_CONFIG;

    if !is_root_user(user_id) && O2_CONFIG.openfga.enabled && O2_CONFIG.openfga.list_only_permitted
    {
        match crate::handler::http::auth::validator::list_objects(user_id, permission, object_type)
            .await
        {
            Ok(resp) => Ok(Some(resp)),
            Err(_) => Err(ErrorForbidden("Unauthorized Access")),
        }
    } else {
        Ok(None)
    }
}

fn get_salt() -> String {
    #[cfg(feature = "enterprise")]
    {
        o2_enterprise::enterprise::common::infra::config::O2_CONFIG
            .common
            .ext_auth_salt
            .clone()
    }
    #[cfg(not(feature = "enterprise"))]
    "openobserve".to_string()
}
#[cfg(test)]
mod tests {
    use infra::db as infra_db;

    use super::*;
    use crate::common::meta::user::UserRequest;

    #[tokio::test]
    async fn test_validate() {
        let org_id = "default";
        let user_id = "user1@example.com";
        let init_user = "root@example.com";
        let pwd = "Complexpass#123";

        infra_db::create_table().await.unwrap();
        users::create_root_user(
            org_id,
            UserRequest {
                email: init_user.to_string(),
                password: pwd.to_string(),
                role: crate::common::meta::user::UserRole::Root,
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
        )
        .await
        .unwrap();
        users::post_user(
            org_id,
            UserRequest {
                email: user_id.to_string(),
                password: pwd.to_string(),
                role: crate::common::meta::user::UserRole::Member,
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: true,
            },
            init_user,
        )
        .await
        .unwrap();

        assert!(
            validate_credentials(init_user, pwd, "default/_bulk")
                .await
                .unwrap()
                .is_valid
        );
        assert!(
            !validate_credentials("", pwd, "default/_bulk")
                .await
                .unwrap()
                .is_valid
        );
        assert!(!validate_credentials("", pwd, "/").await.unwrap().is_valid);
        assert!(
            !validate_credentials(user_id, pwd, "/")
                .await
                .unwrap()
                .is_valid
        );
        assert!(
            validate_credentials(user_id, pwd, "default/user")
                .await
                .unwrap()
                .is_valid
        );
        assert!(
            !validate_credentials(user_id, "x", "default/user")
                .await
                .unwrap()
                .is_valid
        );
        assert!(validate_user(init_user, pwd).await.unwrap().is_valid);
    }
}
