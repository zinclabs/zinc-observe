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

use std::sync::Arc;

use anyhow::bail;
use config::utils::json;

use crate::{
    common::{
        infra::config::{ROOT_USER, USERS, USERS_RUM_TOKEN},
        meta::user::{DBUser, User, UserOrg, UserRole},
    },
    service::{db, users::generate_username},
};

pub async fn get(org_id: Option<&str>, email_id: &str) -> Result<Option<User>, anyhow::Error> {
    let user = match org_id {
        None => ROOT_USER.get("root"),
        Some(org_id) => USERS.get(&format!("{org_id}/{email_id}")),
    };

    if let Some(user) = user {
        return Ok(Some(user.clone()));
    }

    let org_id = org_id.expect("Missing org_id");

    let key = format!("/user/{email_id}");
    let val = db::get(&key).await?;
    let db_user: DBUser = json::from_slice(&val).unwrap();
    Ok(db_user.get_user(org_id.to_string()))
}

/// Lists all users from db
pub async fn list_db_users() -> Option<Vec<DBUser>> {
    let key = "/user/";
    if let Ok(ret) = db::list_values(key).await {
        let mut db_users = Vec::with_capacity(ret.len());
        for bytes in ret {
            let db_user: DBUser = json::from_slice(&bytes).ok()?;
            db_users.push(db_user);
        }
        return Some(db_users);
    }
    None
}

/// Retrieve the user object given token and the requested org
pub async fn get_by_token(
    org_id: Option<&str>,
    token: &str,
) -> Result<Option<User>, anyhow::Error> {
    let user = match org_id {
        None => ROOT_USER.get("root").map(|v| v.value().clone()),
        Some(org_id) => USERS_RUM_TOKEN
            .clone()
            .get(&format!("{org_id}/{token}"))
            .map(|v| v.value().clone()),
    };

    if let Some(user) = user {
        return Ok(Some(user.clone()));
    }

    let org_id = org_id.expect("Missing org_id");

    let key = "/user/";
    let ret = db::list_values(key).await.unwrap();

    let normal_valid_user = |org: &UserOrg| {
        org.name == org_id
            && org
                .rum_token
                .as_ref()
                .is_some_and(|rum_token| rum_token == token)
    };

    let users: Vec<DBUser> = ret
        .iter()
        .map(|item| json::from_slice::<DBUser>(item).unwrap())
        .filter(|user| user.organizations.iter().any(normal_valid_user))
        .collect();

    if users.len() != 1 {
        bail!("Found invalid token for the given org");
    }

    Ok(users[0].get_user(org_id.to_string()))
}

pub async fn get_db_user(email_id: &str) -> Result<DBUser, anyhow::Error> {
    let key = format!("/user/{email_id}");
    let val = db::get(&key).await?;
    Ok(json::from_slice::<DBUser>(&val).unwrap())
}

pub async fn get_db_user_by_username(username: &str) -> Option<DBUser> {
    list_db_users().await.and_then(|db_users| {
        db_users
            .into_iter()
            .find(|db_user| db_user.username == username)
    })
}

pub async fn set(user: &DBUser) -> Result<(), anyhow::Error> {
    let key = format!("/user/{}", user.email);
    db::put(
        &key,
        json::to_vec(&user).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await?;

    // cache user
    for org in &user.organizations {
        let role = match org.role {
            UserRole::Root => org.role.clone(),
            _ => {
                #[cfg(feature = "enterprise")]
                {
                    org.role.clone()
                }
                #[cfg(not(feature = "enterprise"))]
                {
                    UserRole::Admin
                }
            }
        };
        let user = User {
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            username: user.username.clone(),
            password: user.password.clone(),
            role,
            org: org.name.clone(),
            token: org.token.clone(),
            rum_token: org.rum_token.clone(),
            salt: user.salt.clone(),
            is_external: user.is_external,
            password_ext: user.password_ext.clone(),
        };
        if let Some(rum_token) = &org.rum_token {
            USERS_RUM_TOKEN.insert(format!("{}/{}", org.name, rum_token), user.clone());
        }
        if user.role.eq(&UserRole::Root) {
            ROOT_USER.insert("root".to_string(), user.clone());
        }
        USERS.insert(format!("{}/{}", org.name, user.email), user);
    }
    Ok(())
}

pub async fn delete(email_id: &str) -> Result<(), anyhow::Error> {
    let key = format!("/user/{email_id}");
    match db::delete(&key, false, db::NEED_WATCH, None).await {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Error deleting user: {}", e);
            Err(anyhow::anyhow!("Error deleting user: {}", e))
        }
    }
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/user/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching user");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_users: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();

                let item_value: DBUser = if config::get_config().common.meta_store_external {
                    match db::get(&ev.key).await {
                        Ok(val) => match json::from_slice(&val) {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        },
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    }
                } else {
                    json::from_slice(&ev.value.unwrap()).unwrap()
                };

                let users = item_value.get_all_users();
                // Invalidate the entire RUM-TOKEN-CACHE
                for (_, user) in USERS.clone() {
                    if user.email.eq(item_key) {
                        USERS_RUM_TOKEN.clone().remove(&format!(
                            "{}/{}",
                            user.org,
                            user.rum_token.as_ref().unwrap()
                        ));
                    }
                }

                #[cfg(not(feature = "enterprise"))]
                for mut user in users {
                    if user.role.eq(&UserRole::Root) {
                        ROOT_USER.insert("root".to_string(), user.clone());
                    } else {
                        user.role = UserRole::Admin;
                    };
                    USERS.insert(format!("{}/{}", user.org, item_key), user.clone());
                    if let Some(rum_token) = &user.rum_token {
                        USERS_RUM_TOKEN
                            .clone()
                            .insert(format!("{}/{}", user.org, rum_token), user);
                    }
                }

                #[cfg(feature = "enterprise")]
                for user in users {
                    if user.role.eq(&UserRole::Root) {
                        ROOT_USER.insert("root".to_string(), user.clone());
                    }
                    USERS.insert(format!("{}/{}", user.org, item_key), user.clone());
                    if let Some(rum_token) = &user.rum_token {
                        USERS_RUM_TOKEN
                            .clone()
                            .insert(format!("{}/{}", user.org, rum_token), user);
                    }
                }
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                for (_, user) in USERS.clone() {
                    if user.email.eq(item_key) {
                        USERS.remove(&format!("{}/{}", user.org, user.email));
                        // Invalidate the entire RUM-TOKEN-CACHE
                        USERS_RUM_TOKEN.clone().remove(&format!(
                            "{}/{}",
                            user.org,
                            user.rum_token.as_ref().unwrap()
                        ));
                    }
                }
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/user/";
    let bytes = db::list_values(key).await?;
    for item in bytes {
        let mut db_user: DBUser = json::from_slice(&item).unwrap();
        // backfill empty usernames and save to db
        if db_user.username.is_empty() {
            db_user.username = generate_username(&db_user.email).await;
            if let Err(e) = set(&db_user).await {
                log::error!("Error saving updated user into database: {}", e);
                return Err(anyhow::anyhow!(
                    "Error saving updated user into database: {}",
                    e
                ));
            }
        }
        let users = db_user.get_all_users();
        for user in users {
            let user = {
                match user.role {
                    UserRole::Root => {
                        ROOT_USER.insert("root".to_string(), user.clone());
                        user
                    }
                    _ => {
                        #[cfg(not(feature = "enterprise"))]
                        {
                            let mut user = user;
                            user.role = UserRole::Admin;
                            user
                        }
                        #[cfg(feature = "enterprise")]
                        {
                            user
                        }
                    }
                }
            };

            if let Some(rum_token) = &user.rum_token {
                USERS_RUM_TOKEN
                    .clone()
                    .insert(format!("{}/{}", user.org, rum_token), user.clone());
            }
            USERS.insert(format!("{}/{}", user.org, user.email), user);
        }
    }
    log::info!("Users Cached");
    Ok(())
}

pub async fn root_user_exists() -> bool {
    let key = "/user/";
    let mut ret = db::list_values(key).await.unwrap_or_default();
    ret.retain(|item| {
        let user: DBUser = json::from_slice(item).unwrap();
        if user.organizations.is_empty() {
            return false;
        }
        user.organizations
            .first()
            .as_ref()
            .unwrap()
            .role
            .eq(&UserRole::Root)
    });
    !ret.is_empty()
}

pub async fn reset() -> Result<(), anyhow::Error> {
    let key = "/user/";
    db::delete(key, true, db::NO_NEED_WATCH, None).await?;
    Ok(())
}

pub async fn get_user_by_email(email: &str) -> Option<DBUser> {
    let key = "/user/";
    let ret = db::list_values(key).await.unwrap();
    ret.iter()
        .filter_map(|item| {
            let user: DBUser = serde_json::from_slice(item).unwrap();
            user.email.eq(email).then_some(user)
        })
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user() {
        let org_id = "dummy".to_string();
        let email = "user3@example.com";
        let resp = set(&DBUser {
            email: email.to_string(),
            username: "user3".to_string(),
            password: "pass".to_string(),
            salt: String::from("sdfjshdkfshdfkshdfkshdfkjh"),
            first_name: "admin".to_owned(),
            last_name: "".to_owned(),
            is_external: false,
            organizations: vec![UserOrg {
                role: crate::common::meta::user::UserRole::Admin,
                name: org_id.clone(),
                token: "Abcd".to_string(),
                rum_token: Some("rumAbcd".to_string()),
            }],
            password_ext: Some("pass".to_string()),
        })
        .await;
        assert!(resp.is_ok());
        let _ = cache().await;

        let resp = get(Some(&org_id), email).await;
        assert!(resp.unwrap().is_some());

        let resp = delete(email).await;
        assert!(resp.is_ok());
    }
}
