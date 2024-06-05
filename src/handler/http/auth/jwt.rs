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
#[cfg(feature = "enterprise")]
use std::str::FromStr;

#[cfg(feature = "enterprise")]
use config::get_config;
#[cfg(feature = "enterprise")]
use jsonwebtoken::TokenData;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::openfga::{
    authorizer::{
        authz::{
            get_org_creation_tuples, get_user_creation_tuples, get_user_org_tuple,
            get_user_role_creation_tuple, get_user_role_deletion_tuple, update_tuples,
        },
        roles::{
            check_and_get_crole_tuple_for_new_user, get_roles_for_user,
            get_user_crole_removal_tuples,
        },
    },
    meta::mapping::{NON_OWNING_ORG, OFGA_MODELS},
};
#[cfg(feature = "enterprise")]
use once_cell::sync::Lazy;
#[cfg(feature = "enterprise")]
use regex::Regex;
#[cfg(feature = "enterprise")]
use {
    crate::{
        common::meta::user::{DBUser, RoleOrg, TokenValidationResponse, UserOrg, UserRole},
        service::{db, users},
    },
    o2_enterprise::enterprise::common::infra::config::O2_CONFIG,
};
#[cfg(feature = "enterprise")]
use {serde_json::Value, std::collections::HashMap};

#[cfg(feature = "enterprise")]
static RE_ROLE_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9_]+").unwrap());

#[cfg(feature = "enterprise")]
pub async fn process_token(
    res: (
        TokenValidationResponse,
        Option<TokenData<HashMap<String, Value>>>,
    ),
) {
    let dec_token = res.1.unwrap();

    let groups = match dec_token.claims.get(&O2_CONFIG.dex.group_claim) {
        None => vec![],
        Some(groups) => {
            if !groups.is_array() {
                vec![]
            } else {
                groups.as_array().unwrap().to_vec()
            }
        }
    };
    log::debug!("Here is the groups array: {:#?}", groups);

    let user_email = res.0.user_email.to_owned();

    let name = match dec_token.claims.get("name") {
        None => res.0.user_email.to_owned(),
        Some(name) => name.as_str().unwrap().to_string(),
    };
    let mut source_orgs: Vec<UserOrg> = vec![];
    let mut custom_roles: Vec<String> = vec![];
    let mut tuples_to_add = HashMap::new();
    if groups.is_empty() {
        source_orgs.push(UserOrg {
            role: UserRole::from_str(&O2_CONFIG.dex.default_role).unwrap(),
            name: O2_CONFIG.dex.default_org.clone(),
            ..UserOrg::default()
        });
    } else {
        for group in groups {
            let role_org = parse_dn(group.as_str().unwrap()).unwrap();
            if O2_CONFIG.openfga.map_group_to_role {
                custom_roles.push(format_role_name(
                    &role_org.org,
                    role_org.custom_role.unwrap(),
                ));
            } else {
                source_orgs.push(UserOrg {
                    role: role_org.role,
                    name: role_org.org,
                    ..UserOrg::default()
                });
            }
        }
    }

    // Assign users custom roles in RBAC
    if O2_CONFIG.openfga.map_group_to_role {
        map_group_to_custom_role(&user_email, &name, custom_roles).await;
        return;
    }

    // Check if the user exists in the database
    let db_user = db::user::get_user_by_email(&user_email).await;

    if db_user.is_none() {
        log::info!("User does not exist in the database");

        if O2_CONFIG.openfga.enabled {
            for (index, org) in source_orgs.iter().enumerate() {
                let mut tuples = vec![];
                get_user_creation_tuples(
                    &org.name,
                    &user_email,
                    &org.role.to_string(),
                    &mut tuples,
                );
                get_org_creation_tuples(
                    &org.name,
                    &mut tuples,
                    OFGA_MODELS
                        .iter()
                        .map(|(_, fga_entity)| fga_entity.key)
                        .collect(),
                    NON_OWNING_ORG.to_vec(),
                )
                .await;

                if index == 0 {
                    // this is to allow user call organization api with org
                    tuples.push(get_user_org_tuple(&user_email, &user_email));
                }

                tuples_to_add.insert(org.name.to_owned(), tuples);
            }
        }
        let updated_db_user = DBUser {
            email: user_email.to_owned(),
            first_name: name.to_owned(),
            last_name: "".to_owned(),
            password: "".to_owned(),
            salt: "".to_owned(),
            organizations: source_orgs,
            is_external: true,
            password_ext: Some("".to_owned()),
        };

        match users::update_db_user(updated_db_user).await {
            Ok(_) => {
                log::info!("User added to the database");
                if O2_CONFIG.openfga.enabled {
                    for (_, tuples) in tuples_to_add {
                        match update_tuples(tuples, vec![]).await {
                            Ok(_) => {
                                log::info!("User updated to the openfga");
                            }
                            Err(e) => {
                                log::error!("Error updating user to the openfga: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Error adding user to the database: {}", e);
            }
        }
    } else {
        log::info!("User exists in the database perform check for role change");
        let existing_db_user = db_user.unwrap();
        let existing_orgs = existing_db_user.organizations;
        let mut orgs_removed = Vec::new();
        let mut orgs_role_changed = HashMap::new();
        let mut orgs_added = Vec::new();

        let mut write_tuples = Vec::new();
        let mut delete_tuples = Vec::new();

        // Check for newly added organizations
        for source_org in &source_orgs {
            if !existing_orgs
                .iter()
                .any(|existing_org| existing_org.name == source_org.name)
            {
                orgs_added.push(source_org);
            }
        }

        for existing_org in &existing_orgs {
            match source_orgs
                .iter()
                .find(|&src_org| src_org.name == existing_org.name)
            {
                Some(src_org) => {
                    if src_org.role != existing_org.role {
                        // The role has changed for this organization
                        orgs_role_changed.insert(existing_org.role.to_string(), src_org);
                    }
                }
                None => {
                    // The organization is not found in source_orgs, hence removed
                    orgs_removed.push(existing_org);
                }
            }
        }

        // Add the user to the newly added organizations
        for org in orgs_added {
            match users::add_user_to_org(
                &org.name,
                &user_email,
                org.role.clone(),
                &get_config().auth.root_user_email,
            )
            .await
            {
                Ok(_) => {
                    log::info!("User added to the organization {}", org.name);
                    write_tuples.push(get_user_role_creation_tuple(
                        &org.role.to_string(),
                        &user_email,
                        &org.name,
                    ));
                }
                Err(e) => {
                    log::error!("Error adding user to the organization {}: {}", org.name, e);
                }
            }
        }

        for org in orgs_removed {
            match users::remove_user_from_org(
                &org.name,
                &user_email,
                &get_config().auth.root_user_email,
            )
            .await
            {
                Ok(_) => {
                    log::info!("User removed from the organization {}", org.name);
                    delete_tuples.push(get_user_role_deletion_tuple(
                        &org.role.to_string(),
                        &user_email,
                        &org.name,
                    ));
                }
                Err(e) => {
                    log::error!(
                        "Error removing user to the organization {}: {}",
                        org.name,
                        e
                    );
                }
            }
        }
        for (existing_role, org) in orgs_role_changed {
            match users::update_user(
                &org.name,
                &user_email,
                false,
                &get_config().auth.root_user_email,
                crate::common::meta::user::UpdateUser {
                    role: Some(org.role.clone()),
                    ..Default::default()
                },
            )
            .await
            {
                Ok(_) => {
                    log::info!("User update for the organization {}", org.name);
                    delete_tuples.push(get_user_role_deletion_tuple(
                        &existing_role,
                        &user_email,
                        &org.name,
                    ));
                    write_tuples.push(get_user_role_creation_tuple(
                        &org.role.to_string(),
                        &user_email,
                        &org.name,
                    ));
                }
                Err(e) => {
                    log::error!(
                        "Error updating user to the organization {}: {}",
                        org.name,
                        e
                    );
                }
            }
        }
        if O2_CONFIG.openfga.enabled {
            if write_tuples.is_empty() && delete_tuples.is_empty() {
                log::info!("No changes to the user information tuples");
            } else {
                log::info!(
                    "openfga tuples: wt {:?} ,dt {:?} ",
                    write_tuples,
                    delete_tuples
                );

                match update_tuples(write_tuples, delete_tuples).await {
                    Ok(_) => {
                        log::info!("User updated to the openfga");
                    }
                    Err(_) => {
                        log::error!("Error updating user to the openfga");
                    }
                }
            }
        }
    };
}

#[cfg(feature = "enterprise")]
fn parse_dn(dn: &str) -> Option<RoleOrg> {
    let mut org = "";
    let mut role = "";
    let mut custom_role = None;
    log::debug!("parse_dn dn is: {dn}");

    if O2_CONFIG.openfga.map_group_to_role {
        custom_role = Some(dn.to_owned());
        org = &O2_CONFIG.dex.default_org;
    } else {
        for part in dn.split(',') {
            let parts: Vec<&str> = part.split('=').collect();
            if parts.len() == 2 {
                if parts[0].eq(&O2_CONFIG.dex.group_attribute) && org.is_empty() {
                    org = parts[1];
                }
                if parts[0].eq(&O2_CONFIG.dex.role_attribute) && role.is_empty() {
                    role = parts[1];
                }
            }
        }
    }
    let role = if role.is_empty() {
        UserRole::from_str(&O2_CONFIG.dex.default_role).unwrap()
    } else {
        UserRole::from_str(role).unwrap()
    };

    if org.is_empty() {
        org = &O2_CONFIG.dex.default_org;
    }
    Some(RoleOrg {
        role,
        org: org.to_owned(),
        custom_role,
    })
}

#[cfg(feature = "enterprise")]
async fn map_group_to_custom_role(user_email: &str, name: &str, custom_roles: Vec<String>) {
    // Check if the user exists in the database
    let db_user = db::user::get_user_by_email(user_email).await;
    log::debug!("map_group_to_custom_role custom roles: {:#?}", custom_roles);

    if db_user.is_none() {
        let mut tuples = vec![];
        log::info!("group_to_custom_role: User does not exist in the database");

        if O2_CONFIG.openfga.enabled {
            get_org_creation_tuples(
                &O2_CONFIG.dex.default_org,
                &mut tuples,
                OFGA_MODELS
                    .iter()
                    .map(|(_, fga_entity)| fga_entity.key)
                    .collect(),
                NON_OWNING_ORG.to_vec(),
            )
            .await;
            tuples.push(get_user_org_tuple(&O2_CONFIG.dex.default_org, user_email));
            tuples.push(get_user_org_tuple(user_email, user_email));
            let start = std::time::Instant::now();
            check_and_get_crole_tuple_for_new_user(
                user_email,
                &O2_CONFIG.dex.default_org,
                custom_roles,
                &mut tuples,
            )
            .await;
            log::info!(
                "group_to_custom_role: Time taken to get crole tuple: {:?}",
                start.elapsed()
            );
        }
        let updated_db_user = DBUser {
            email: user_email.to_owned(),
            first_name: name.to_owned(),
            last_name: "".to_owned(),
            password: "".to_owned(),
            salt: "".to_owned(),
            organizations: vec![UserOrg {
                role: UserRole::from_str(&O2_CONFIG.dex.default_role).unwrap(),
                name: O2_CONFIG.dex.default_org.clone(),
                ..UserOrg::default()
            }],
            is_external: true,
            password_ext: Some("".to_owned()),
        };

        match users::update_db_user(updated_db_user).await {
            Ok(_) => {
                log::info!("group_to_custom_role: User added to the database");
                if O2_CONFIG.openfga.enabled {
                    let start = std::time::Instant::now();
                    match update_tuples(tuples, vec![]).await {
                        Ok(_) => {
                            log::info!("group_to_custom_role: User updated to the openfga");
                        }
                        Err(e) => {
                            log::error!(
                                "group_to_custom_role: Error updating user to the openfga: {}",
                                e
                            );
                        }
                    }

                    log::info!(
                        "group_to_custom_role: Time taken to update roles to db for new user: {:?}",
                        start.elapsed()
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "group_to_custom_role: Error adding user to the database: {}",
                    e
                );
            }
        }
    } else {
        log::info!("group_to_custom_role: User exists in the database");
        let mut add_tuples = vec![];
        let mut remove_tuples = vec![];
        // user exists in the db with default org hence skip org creation tuples
        let existing_roles = get_roles_for_user(user_email).await;
        log::debug!("user exists existing roles: {:#?}", existing_roles);

        // Find roles to delete: present in existing_role but not in custom_role
        for existing_role in &existing_roles {
            if !custom_roles.contains(existing_role) {
                // delete role
                get_user_crole_removal_tuples(user_email, existing_role, &mut remove_tuples);
            }
        }

        // Find new roles to add: present in custom_role but not in existing_role
        let new_roles = custom_roles
            .iter()
            .filter(|&role| !existing_roles.contains(role))
            .cloned()
            .collect();
        log::debug!("new roles: {:#?}", new_roles);

        check_and_get_crole_tuple_for_new_user(
            user_email,
            &O2_CONFIG.dex.default_org,
            new_roles,
            &mut add_tuples,
        )
        .await;
        log::debug!(
            "add_tuples: {:#?}\nremove_tuples: {:#?}",
            add_tuples,
            remove_tuples
        );

        if O2_CONFIG.openfga.enabled {
            let start = std::time::Instant::now();
            match update_tuples(add_tuples, remove_tuples).await {
                Ok(_) => {
                    log::info!("group_to_custom_role: User updated to the openfga");
                }
                Err(e) => {
                    log::error!(
                        "group_to_custom_role: Error updating user to the openfga: {}",
                        e
                    );
                }
            }
            log::info!(
                "group_to_custom_role: Time taken to update roles to db for existing user: {:?}",
                start.elapsed()
            );
        }
    }
}

#[cfg(feature = "enterprise")]
fn format_role_name(org: &str, role: String) -> String {
    let role = RE_ROLE_NAME.replace_all(&role, "_").to_string();
    format!("{org}/{role}")
}
