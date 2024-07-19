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

use std::{collections::HashSet, io::Error};

use actix_web::{http, HttpResponse};
use config::{get_config, ider, utils::rand::generate_random_string};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::O2_CONFIG;

use crate::{
    common::{
        infra::config::{ROOT_USER, USERS, USERS_RUM_TOKEN},
        meta::{
            http::HttpResponse as MetaHttpResponse,
            organization::DEFAULT_ORG,
            user::{
                DBUser, UpdateUser, User, UserList, UserOrg, UserRequest, UserResponse, UserRole,
            },
        },
        utils::auth::{get_hash, get_role, is_root_user},
    },
    service::db,
};

pub async fn post_user(
    org_id: &str,
    usr_req: UserRequest,
    initiator_email: &str,
) -> Result<HttpResponse, Error> {
    let initiator_user = db::user::get(Some(org_id), initiator_email).await;
    if is_root_user(initiator_email)
        || (initiator_user.is_ok()
            && initiator_user.as_ref().unwrap().is_some()
            && initiator_user.unwrap().unwrap().role.eq(&UserRole::Admin))
    {
        let cfg = get_config();
        let existing_user = if is_root_user(&usr_req.email) {
            db::user::get(None, &usr_req.email).await
        } else {
            db::user::get(Some(org_id), &usr_req.email).await
        };
        if existing_user.is_err() {
            let salt = ider::uuid();
            let username = generate_username(&usr_req.email).await;
            let password = get_hash(&usr_req.password, &salt);
            let password_ext = get_hash(&usr_req.password, &cfg.auth.ext_auth_salt);
            let token = generate_random_string(16);
            let rum_token = format!("rum{}", generate_random_string(16));
            let user = usr_req.to_new_dbuser(
                username,
                password,
                salt,
                org_id.replace(' ', "_"),
                token,
                rum_token,
                usr_req.is_external,
                password_ext,
            );
            db::user::set(&user).await.unwrap();
            // Update OFGA
            #[cfg(feature = "enterprise")]
            {
                use o2_enterprise::enterprise::openfga::{
                    authorizer::authz::{
                        get_org_creation_tuples, get_user_role_tuple, update_tuples,
                    },
                    meta::mapping::{NON_OWNING_ORG, OFGA_MODELS},
                };
                if O2_CONFIG.openfga.enabled {
                    let mut tuples = vec![];
                    get_user_role_tuple(
                        &usr_req.role.to_string(),
                        &usr_req.email,
                        &org_id.replace(' ', "_"),
                        &mut tuples,
                    );
                    get_org_creation_tuples(
                        org_id,
                        &mut tuples,
                        OFGA_MODELS
                            .iter()
                            .map(|(_, fga_entity)| fga_entity.key)
                            .collect(),
                        NON_OWNING_ORG.to_vec(),
                    )
                    .await;
                    match update_tuples(tuples, vec![]).await {
                        Ok(_) => {
                            log::info!("User saved successfully in openfga");
                        }
                        Err(e) => {
                            log::error!("Error creating user in openfga: {}", e);
                        }
                    }
                }
            }
            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "User saved successfully".to_string(),
            )))
        } else {
            Ok(HttpResponse::BadRequest().json(MetaHttpResponse::message(
                http::StatusCode::BAD_REQUEST.into(),
                "User already exists".to_string(),
            )))
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(MetaHttpResponse::error(
            http::StatusCode::UNAUTHORIZED.into(),
            "Not Allowed".to_string(),
        )))
    }
}

pub async fn update_db_user(mut db_user: DBUser) -> Result<(), anyhow::Error> {
    if db_user.password.is_empty() {
        let salt = ider::uuid();
        let generated_pass = generate_random_string(8);
        let password = get_hash(&generated_pass, &salt);
        db_user.password = password;
        db_user.salt = salt;
    }
    for org in &mut db_user.organizations {
        if org.token.is_empty() {
            let token = generate_random_string(16);
            org.token = token;
        };
        if org.rum_token.is_none() {
            let rum_token = format!("rum{}", generate_random_string(16));
            org.rum_token = Some(rum_token);
        };
    }
    db::user::set(&db_user).await
}

pub async fn update_user(
    org_id: &str,
    username: &str,
    self_update: bool,
    initiator_email: &str,
    user: UpdateUser,
) -> Result<HttpResponse, Error> {
    let mut allow_password_update = false;

    let Some(local_user) = db::user::get_db_user_by_username(username)
        .await
        .and_then(|db_user| db_user.get_user(org_id.to_string()))
    else {
        return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            "User not found".to_string(),
        )));
    };

    if local_user.is_external {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::message(
            http::StatusCode::BAD_REQUEST.into(),
            "Updates not allowed in OpenObserve, please update with source system".to_string(),
        )));
    }

    let mut old_role = None;
    let mut new_role = None;
    let conf = get_config();
    let password_ext_salt = conf.auth.ext_auth_salt.as_str();
    let mut is_updated = false;
    let mut is_org_updated = false;
    let mut message = "";

    let initiating_user = db::user::get(Some(org_id), initiator_email)
        .await
        .unwrap()
        .unwrap();

    if !self_update {
        // initiator is the root user OR
        // initiator is an Admin AND local_user is not the root user
        if is_root_user(initiator_email)
            || (!local_user.role.eq(&UserRole::Root) && initiating_user.role.eq(&UserRole::Admin))
        {
            allow_password_update = true
        }
    }

    let mut new_user = local_user.clone();
    // update password
    if self_update && user.old_password.is_some() && user.new_password.is_some() {
        if local_user.password.eq(&get_hash(
            &user.old_password.clone().unwrap(),
            &local_user.salt,
        )) {
            let new_pass = user.new_password.unwrap();

            new_user.password = get_hash(&new_pass, &local_user.salt);
            new_user.password_ext = Some(get_hash(&new_pass, password_ext_salt));
            log::info!("Password self updated for user: {}", username);
            is_updated = true;
        } else {
            message = "Existing/old password mismatch, please provide valid existing password";
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::message(
                http::StatusCode::BAD_REQUEST.into(),
                message.to_string(),
            )));
        }
    } else if self_update && user.old_password.is_none() {
        message = "Please provide existing password"
    } else if !self_update
        && allow_password_update
        && user.new_password.is_some()
        && !local_user.is_external
    {
        let new_pass = user.new_password.unwrap();

        new_user.password = get_hash(&new_pass, &local_user.salt);
        new_user.password_ext = Some(get_hash(&new_pass, password_ext_salt));
        log::info!(
            "User {username}'s password is updated by user: {}",
            initiating_user.username
        );

        is_updated = true;
    } else {
        message = "You are not authorized to change other users' password"
    }

    // update first or last name
    if user.first_name.is_some() && !local_user.is_external {
        new_user.first_name = user.first_name.unwrap();
        is_updated = true;
    }
    if user.last_name.is_some() && !local_user.is_external {
        new_user.last_name = user.last_name.unwrap();
        is_updated = true;
    }

    // update role
    if user.role.is_some()
        && !local_user.is_external
        && (!self_update
            || (local_user.role.eq(&UserRole::Admin) || local_user.role.eq(&UserRole::Root)))
        // if the User Role is Root, we do not change the Role
        // Admins Role can still be mutable.
        && !local_user.role.eq(&UserRole::Root)
    {
        old_role = Some(new_user.role);
        new_user.role = user.role.unwrap();
        new_role = Some(new_user.role.clone());
        is_org_updated = true;
    }

    // update token
    if user.token.is_some() {
        new_user.token = user.token.unwrap();
        is_org_updated = true;
    }

    // save the new user
    if is_updated || is_org_updated {
        match db::user::get_db_user(&new_user.email).await {
            Ok(mut db_user) => {
                db_user.password = new_user.password;
                db_user.password_ext = new_user.password_ext;
                db_user.first_name = new_user.first_name;
                db_user.last_name = new_user.last_name;
                if is_org_updated {
                    db_user.organizations.retain(|org| !org.name.eq(org_id));
                    db_user.organizations.push(UserOrg {
                        name: org_id.to_string(),
                        token: new_user.token,
                        rum_token: new_user.rum_token,
                        role: new_user.role,
                    });
                }
                db::user::set(&db_user).await.unwrap();
                #[cfg(feature = "enterprise")]
                {
                    use o2_enterprise::enterprise::openfga::authorizer::authz::update_user_role;

                    if O2_CONFIG.openfga.enabled && old_role.is_some() && new_role.is_some() {
                        let old = old_role.unwrap();
                        let new = new_role.unwrap();
                        if !old.eq(&new) {
                            let mut old_str = old.to_string();
                            let mut new_str = new.to_string();
                            if old.eq(&UserRole::User) || old.eq(&UserRole::ServiceAccount) {
                                old_str = "allowed_user".to_string();
                            }
                            if new.eq(&UserRole::User) || new.eq(&UserRole::ServiceAccount) {
                                new_str = "allowed_user".to_string();
                            }
                            if old_str != new_str {
                                log::debug!(
                                    "updating openfga role for {} from {old_str} to {new_str}",
                                    new_user.email
                                );
                                update_user_role(&old_str, &new_str, &new_user.email, org_id).await;
                            }
                        }
                    }
                }

                #[cfg(not(feature = "enterprise"))]
                log::debug!("Role changed from {:?} to {:?}", old_role, new_role);
                Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                    http::StatusCode::OK.into(),
                    "User updated successfully".to_string(),
                )))
            }
            Err(_) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                http::StatusCode::NOT_FOUND.into(),
                "User not found".to_string(),
            ))),
        }
    } else {
        if message.is_empty() {
            message = "Not allowed to update";
        }
        Ok(HttpResponse::BadRequest().json(MetaHttpResponse::message(
            http::StatusCode::BAD_REQUEST.into(),
            message.to_string(),
        )))
    }
}

pub async fn add_user_to_org(
    org_id: &str,
    username: &str,
    role: UserRole,
    initiator_email: &str,
) -> Result<HttpResponse, Error> {
    let Some(mut db_user) = db::user::get_db_user_by_username(username).await else {
        return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            "User not found".to_string(),
        )));
    };
    let root_user = ROOT_USER.clone();
    let local_org;
    let initiating_user = if is_root_user(initiator_email) {
        local_org = org_id.replace(' ', "_");
        root_user.get("root").unwrap().clone()
    } else {
        local_org = org_id.to_owned();
        db::user::get(Some(org_id), initiator_email)
            .await
            .unwrap()
            .unwrap()
    };
    if initiating_user.role.eq(&UserRole::Root) || initiating_user.role.eq(&UserRole::Admin) {
        let role = get_role(role);
        let token = generate_random_string(16);
        let rum_token = format!("rum{}", generate_random_string(16));
        if db_user.is_external && db_user.organizations.iter().any(|org| org.name.eq(org_id)) {
            // External user is already part of this org
            return Ok(HttpResponse::Forbidden().json(MetaHttpResponse::error(
                http::StatusCode::FORBIDDEN.into(),
                "Not Allowed".to_string(),
            )));
        }
        db_user.organizations.retain(|org| !org.name.eq(&local_org));
        db_user.organizations.push(UserOrg {
            name: local_org.to_string(),
            token,
            rum_token: Some(rum_token),
            role: role.clone(),
        });
        db::user::set(&db_user).await.unwrap();

        // TODO(taiming): pending
        // Update OFGA
        #[cfg(feature = "enterprise")]
        {
            use o2_enterprise::enterprise::openfga::{
                authorizer::authz::{get_org_creation_tuples, get_user_role_tuple, update_tuples},
                meta::mapping::{NON_OWNING_ORG, OFGA_MODELS},
            };
            if O2_CONFIG.openfga.enabled {
                let mut tuples = vec![];
                get_user_role_tuple(&role.to_string(), &db_user.email, org_id, &mut tuples);
                get_org_creation_tuples(
                    org_id,
                    &mut tuples,
                    OFGA_MODELS
                        .iter()
                        .map(|(_, fga_entity)| fga_entity.key)
                        .collect(),
                    NON_OWNING_ORG.to_vec(),
                )
                .await;
                match update_tuples(tuples, vec![]).await {
                    Ok(_) => {
                        log::info!("User added to org successfully in openfga");
                    }
                    Err(e) => {
                        log::error!("Error adding user to the org in openfga: {}", e);
                    }
                }
            }
        }
        Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "User added to org successfully".to_string(),
        )))
    } else {
        Ok(HttpResponse::Unauthorized().json(MetaHttpResponse::error(
            http::StatusCode::UNAUTHORIZED.into(),
            "Not Allowed".to_string(),
        )))
    }
}

pub async fn get_user(org_id: Option<&str>, email_id: &str) -> Option<User> {
    let key = match org_id {
        Some(local_org) => format!("{local_org}/{email_id}"),
        None => format!("{DEFAULT_ORG}/{email_id}"),
    };
    let user = USERS.get(&key);
    match user {
        Some(loc_user) => Some(loc_user.value().clone()),
        None => db::user::get(org_id, email_id).await.ok().flatten(),
    }
}

// TODO(taiming): this can be refactored, since the db::user::get_by_token is basically doing the
// same thing
pub async fn get_user_by_token(org_id: &str, token: &str) -> Option<User> {
    let rum_tokens = USERS_RUM_TOKEN.clone();
    let key = format!("{DEFAULT_ORG}/{token}");
    if let Some(user) = rum_tokens.get(&key) {
        return Some(user.value().clone());
    }

    let key = format!("{org_id}/{token}");
    if let Some(user) = rum_tokens.get(&key) {
        return Some(user.value().clone());
    }

    // need to drop the reference to rum_tokens to avoid deadlock of dashmap
    drop(rum_tokens);

    log::info!("get_user_by_token: User not found in cache, fetching from db");
    if let Some(user_from_db) = db::user::get_by_token(Some(org_id), token)
        .await
        .ok()
        .flatten()
    {
        log::info!("get_user_by_token: User found updating cache");
        if is_root_user(&user_from_db.email) {
            USERS_RUM_TOKEN
                .clone()
                .insert(format!("{DEFAULT_ORG}/{token}"), user_from_db.clone());
        }
        USERS_RUM_TOKEN.clone().insert(key, user_from_db.clone());
        Some(user_from_db)
    } else {
        log::info!(
            "get_user_by_token: User not found even in db {} {}",
            org_id,
            token
        );
        None
    }
}

// TODO(taiming): updated the response and check with
pub async fn list_users(org_id: &str) -> Result<HttpResponse, Error> {
    let mut user_list: Vec<UserResponse> = vec![];
    for user in USERS.iter() {
        if user.key().starts_with(&format!("{org_id}/")) {
            user_list.push(UserResponse {
                username: user.value().username.clone(),
                role: user.value().role.clone(),
                first_name: user.value().first_name.clone(),
                last_name: user.value().last_name.clone(),
                is_external: user.value().is_external,
            })
        }
    }

    #[cfg(feature = "enterprise")]
    {
        if !org_id.eq(DEFAULT_ORG) {
            let root = ROOT_USER.get("root").unwrap();
            let root_user = root.value();
            user_list.push(UserResponse {
                username: root_user.username.clone(),
                role: root_user.role.clone(),
                first_name: root_user.first_name.clone(),
                last_name: root_user.last_name.clone(),
                is_external: root_user.is_external,
            })
        }
    }

    Ok(HttpResponse::Ok().json(UserList { data: user_list }))
}

pub async fn remove_user_from_org(
    org_id: &str,
    username: &str,
    initiator_email: &str,
) -> Result<HttpResponse, Error> {
    let initiating_user = if is_root_user(initiator_email) {
        ROOT_USER.get("root").unwrap().clone()
    } else {
        db::user::get(Some(org_id), initiator_email)
            .await
            .unwrap()
            .unwrap()
    };
    if initiating_user.role.eq(&UserRole::Root) || initiating_user.role.eq(&UserRole::Admin) {
        let ret_user = db::user::get_db_user_by_username(username).await;
        match ret_user {
            Some(mut user) => {
                if !user.organizations.is_empty() {
                    if user.organizations.len() == 1 {
                        let _ = db::user::delete(&user.email).await;
                        #[cfg(feature = "enterprise")]
                        {
                            use o2_enterprise::enterprise::openfga::authorizer::authz::delete_user_from_org;
                            let user_role = &user.organizations[0].role;
                            let user_fga_role = if user_role.eq(&UserRole::ServiceAccount)
                                || user_role.eq(&UserRole::User)
                            {
                                "allowed_user".to_string()
                            } else {
                                user_role.to_string()
                            };
                            if O2_CONFIG.openfga.enabled {
                                log::debug!("delete user single org, role: {}", &user_fga_role);
                                delete_user_from_org(org_id, &user.email, &user_fga_role).await;
                            }
                        }
                    } else {
                        let mut _user_fga_role: Option<String> = None;
                        #[cfg(feature = "enterprise")]
                        for org in user.organizations.iter() {
                            if org.name.eq(org_id) {
                                let user_role = &org.role;
                                _user_fga_role = if user_role.eq(&UserRole::ServiceAccount)
                                    || user_role.eq(&UserRole::User)
                                {
                                    Some("allowed_user".to_string())
                                } else {
                                    Some(user_role.to_string())
                                };
                            }
                        }
                        user.organizations.retain(|org| !org.name.eq(org_id));
                        let resp = db::user::set(&user).await;
                        // special case as we cache flattened user struct
                        if resp.is_ok() {
                            USERS.remove(&format!("{org_id}/{}", user.email));
                            #[cfg(feature = "enterprise")]
                            {
                                use o2_enterprise::enterprise::openfga::authorizer::authz::delete_user_from_org;
                                log::debug!(
                                    "user_fga_role, multi org: {}",
                                    _user_fga_role.as_ref().unwrap()
                                );
                                if O2_CONFIG.openfga.enabled && _user_fga_role.is_some() {
                                    delete_user_from_org(
                                        org_id,
                                        &user.email,
                                        _user_fga_role.unwrap().as_str(),
                                    )
                                    .await;
                                }
                            }
                        }
                    }
                    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                        http::StatusCode::OK.into(),
                        "User removed from organization".to_string(),
                    )))
                } else {
                    Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                        http::StatusCode::NOT_FOUND.into(),
                        format!("User not found with in organization {org_id}"),
                    )))
                }
            }
            None => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                http::StatusCode::NOT_FOUND.into(),
                "User associated with the given username not found".to_string(),
            ))),
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(MetaHttpResponse::error(
            http::StatusCode::UNAUTHORIZED.into(),
            "Not Allowed".to_string(),
        )))
    }
}

pub async fn delete_user(email_id: &str) -> Result<HttpResponse, Error> {
    let result = db::user::delete(email_id).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "User deleted".to_string(),
        ))),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}

pub async fn root_user_exists() -> bool {
    let local_users = ROOT_USER.clone();
    if !local_users.is_empty() {
        true
    } else {
        db::user::root_user_exists().await
    }
}

pub fn is_user_from_org(orgs: Vec<UserOrg>, org_id: &str) -> (bool, UserOrg) {
    if orgs.is_empty() {
        (false, UserOrg::default())
    } else {
        let mut local_orgs = orgs;
        local_orgs.retain(|org| !org.name.eq(org_id));
        local_orgs
            .first()
            .map_or((false, UserOrg::default()), |user_org| {
                (true, user_org.clone())
            })
    }
}

pub(crate) async fn create_root_user(usr_req: UserRequest) -> Result<(), Error> {
    let cfg = get_config();
    let salt = ider::uuid();
    let username = generate_username(&usr_req.email).await;
    let password = get_hash(&usr_req.password, &salt);
    let password_ext = get_hash(&usr_req.password, &cfg.auth.ext_auth_salt);
    let token = generate_random_string(16);
    let rum_token = format!("rum{}", generate_random_string(16));
    let user = usr_req.to_new_dbuser(
        username,
        password,
        salt,
        DEFAULT_ORG.replace(' ', "_"),
        token,
        rum_token,
        usr_req.is_external,
        password_ext,
    );
    db::user::set(&user).await.unwrap();
    Ok(())
}

/// Based on the given `email_id`, generates a new username that is unique in the
/// entire system. To keep the generated username meaningful and memorable, first
/// attempts to take the prefix of the given email (already validated).
///
/// If the attempted username already exists, modifies the username by appending
/// a random string of length 2 until it is new.
/// Random string generated evenly through a-z, A-Z, and 0-9. 3844, 62 * 62,
/// possible strings in total, which fits the application's current scope.
pub(crate) async fn generate_username(email_id: &str) -> String {
    let existing_usernames: HashSet<String> = {
        let mut usernames = HashSet::new();
        if let Some(db_users) = db::user::list_db_users().await {
            usernames.extend(db_users.into_iter().map(|user| user.username));
        }
        // root_user is also part of [USERS]
        usernames.extend(USERS.iter().map(|entry| entry.username.clone()));
        usernames
    };

    let attempt = email_id.split_once('@').unwrap().0.to_string();
    let mut username = attempt.clone();
    while existing_usernames.contains(&username) {
        username = format!("{}{}", attempt, generate_random_string(2));
    }
    username
}

#[cfg(test)]
mod tests {
    use infra::db as infra_db;

    use super::*;

    async fn set_up() {
        USERS.insert(
            "dummy/admin@zo.dev".to_string(),
            User {
                email: "admin@zo.dev".to_string(),
                username: generate_username("admin@zo.dev").await,
                password: "pass#123".to_string(),
                role: crate::common::meta::user::UserRole::Admin,
                salt: String::new(),
                token: "token".to_string(),
                rum_token: Some("rum_token".to_string()),
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
                org: "dummy".to_string(),
                is_external: false,
                password_ext: Some("pass#123".to_string()),
            },
        );
    }

    #[tokio::test]
    async fn test_list_users() {
        set_up().await;
        assert!(list_users("dummy").await.is_ok())
    }

    #[tokio::test]
    async fn test_root_user_exists() {
        set_up().await;
        assert!(!root_user_exists().await);
    }

    #[tokio::test]
    async fn test_get_user() {
        set_up().await;
        assert!(get_user(Some("dummy"), "admin@zo.dev").await.is_some())
    }

    #[tokio::test]
    async fn test_post_user() {
        infra_db::create_table().await.unwrap();
        set_up().await;

        let resp = post_user(
            "dummy",
            UserRequest {
                email: "user@zo.dev".to_string(),
                password: "pass#123".to_string(),
                role: crate::common::meta::user::UserRole::Admin,
                first_name: "user".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
            "admin@zo.dev",
        )
        .await;
        assert!(resp.is_ok());
    }

    #[tokio::test]
    async fn test_user() {
        infra_db::create_table().await.unwrap();
        set_up().await;

        let resp = update_user(
            "dummy",
            "user2@example.com",
            true,
            "user2@example.com",
            UpdateUser {
                token: Some("new_token".to_string()),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                old_password: Some("pass".to_string()),
                new_password: Some("new_pass".to_string()),
                role: Some(crate::common::meta::user::UserRole::Member),
                change_password: false,
            },
        )
        .await;

        assert!(resp.is_ok());

        let resp = update_user(
            "dummy",
            "user2@example.com",
            false,
            "admin@zo.dev",
            UpdateUser {
                token: Some("new_token".to_string()),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                old_password: None,
                new_password: None,
                role: Some(crate::common::meta::user::UserRole::Admin),
                change_password: false,
            },
        )
        .await;

        assert!(resp.is_ok());

        let resp = add_user_to_org(
            "dummy",
            "user2@example.com",
            UserRole::Member,
            "admin@zo.dev",
        )
        .await;

        assert!(resp.is_ok());

        let resp = remove_user_from_org("dummy", "user2@example.com", "admin@zo.dev").await;

        assert!(resp.is_ok());
    }
}
