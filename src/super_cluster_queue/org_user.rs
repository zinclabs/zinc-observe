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
use config::utils::json;
use infra::{
    db::{delete_from_db_coordinator, put_into_db_coordinator},
    errors::{Error, Result},
    table::org_users::{self, OrgUserPut},
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::OrgUserAdd => {
            add(msg).await?;
        }
        MessageType::OrgUserDelete => {
            delete(msg).await?;
        }
        MessageType::OrgUserUpdate => {
            update(msg).await?;
        }
        MessageType::OrgUserUpdateToken => {
            update_token(msg).await?;
        }
        MessageType::OrgUserUpdateRumToken => {
            update_rum_token(msg).await?;
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
    let org_user: OrgUserPut = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = org_users::add(
        &org_user.org_id,
        &org_user.email,
        org_user.role,
        &org_user.token,
        org_user.rum_token,
    )
    .await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to add org user: {}/{}, error: {}",
            org_user.org_id,
            org_user.email,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, Bytes::new(), msg.need_watch, None).await;
    Ok(())
}

async fn update(msg: Message) -> Result<()> {
    let org_user: OrgUserPut = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = org_users::update(
        &org_user.org_id,
        &org_user.email,
        org_user.role,
        &org_user.token,
        org_user.rum_token,
    )
    .await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update org user: {}/{}, error: {}",
            org_user.org_id,
            org_user.email,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, Bytes::new(), msg.need_watch, None).await;
    Ok(())
}

async fn update_token(msg: Message) -> Result<()> {
    let token = match msg.value {
        Some(ref value) => String::from_utf8_lossy(value).to_string(),
        None => "".to_string(),
    };
    let keys = msg.key.split('/').collect::<Vec<&str>>();
    if keys.len() < 4 {
        return Err(Error::Message("Invalid key".to_string()));
    }
    let org_id = keys[2];
    let user_email = keys[3];
    if let Err(e) = org_users::update_token(org_id, user_email, &token).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update token for org user: {}/{}, error: {}",
            org_id,
            user_email,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, Bytes::new(), msg.need_watch, None).await;
    Ok(())
}

async fn update_rum_token(msg: Message) -> Result<()> {
    let rum_token = match msg.value {
        Some(ref value) => String::from_utf8_lossy(value).to_string(),
        None => "".to_string(),
    };
    let keys = msg.key.split('/').collect::<Vec<&str>>();
    if keys.len() < 4 {
        return Err(Error::Message("Invalid key".to_string()));
    }
    let org_id = keys[2];
    let user_email = keys[3];
    if let Err(e) = org_users::update_rum_token(org_id, user_email, &rum_token).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update RUM token for org user: {}/{}, error: {}",
            org_id,
            user_email,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, Bytes::new(), msg.need_watch, None).await;
    Ok(())
}

async fn delete(msg: Message) -> Result<()> {
    let keys = msg.key.split('/').collect::<Vec<&str>>();
    if keys.len() < 4 {
        return Err(Error::Message("Invalid key".to_string()));
    }
    let org_id = keys[2];
    let user_email = keys[3];
    if keys[1].eq("many") {
        if let Err(e) = org_users::remove_by_user(user_email).await {
            log::error!(
                "[SUPER_CLUSTER:sync] Failed to delete org user: {}, error: {}",
                user_email,
                e
            );
            return Err(e);
        }
    } else if let Err(e) = org_users::remove(org_id, user_email).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to delete org user: {}/{}, error: {}",
            org_id,
            user_email,
            e
        );
        return Err(e);
    }
    let _ = delete_from_db_coordinator(&msg.key, false, true, None).await;
    Ok(())
}
