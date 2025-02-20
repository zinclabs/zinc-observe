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

use bytes::Bytes;
use config::{meta::user::DBUser, utils::json};
use infra::{
    db::put_into_db_coordinator,
    errors::{Error, Result},
    table::{
        org_users,
        users::{self, UserRecord, UserUpdate},
    },
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::UserAdd => {
            add(msg).await?;
        }
        MessageType::UserUpdate => {
            update(msg).await?;
        }
        MessageType::UserDelete => {
            delete(msg).await?;
        }
        _ => {
            log::error!(
                "[SUPER_CLUSTER:DB] Invalid message: type: {:?}, key: {}",
                msg.message_type,
                msg.key
            );
            return Err(Error::Message("Invalid message type".to_string()));
        }
    }
    Ok(())
}

async fn add(msg: Message) -> Result<()> {
    let db_user: DBUser = json::from_slice(&msg.value.unwrap())?;
    let user = UserRecord::from(&db_user);
    if let Err(e) = users::add(user.clone()).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to add user: {}, error: {}",
            user.email,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, Bytes::new(), true, None).await;

    // Add user to orgs
    for org in &db_user.organizations {
        org_users::add(
            &org.name,
            &user.email,
            org.role.clone(),
            &org.token,
            org.rum_token.clone(),
        )
        .await?;
        let key = format!("/org_users/single/{}/{}", org.name, user.email);
        let _ = put_into_db_coordinator(&key, Bytes::new(), true, None).await;
    }
    Ok(())
}

async fn update(msg: Message) -> Result<()> {
    let user: UserUpdate = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = users::update(
        &user.email,
        &user.first_name,
        &user.last_name,
        &user.password,
        user.password_ext,
    )
    .await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update user: {}, error: {}",
            user.email,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, Bytes::new(), true, None).await;
    Ok(())
}

async fn delete(msg: Message) -> Result<()> {
    let keys = msg.key.split('/').collect::<Vec<_>>();
    if keys.len() != 2 {
        log::error!(
            "[SUPER_CLUSTER:sync] Invalid key: {}, expected 3 parts, got {}",
            msg.key,
            keys.len()
        );
        return Err(Error::Message("Invalid key".to_string()));
    }
    let email = keys[1];
    if let Err(e) = org_users::remove_by_user(email).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to delete user: {}, error: {}",
            email,
            e
        );
        return Err(e);
    }
    if let Err(e) = users::remove(email).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to delete user: {}, error: {}",
            email,
            e
        );
        return Err(e);
    }

    Ok(())
}
