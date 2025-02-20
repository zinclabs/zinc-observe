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

use config::utils::json;
use infra::{
    db::{delete_from_db_coordinator, put_into_db_coordinator},
    errors::{Error, Result},
    table::organizations,
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Organization {
    #[serde(default)]
    pub identifier: String,
    pub name: String,
    #[serde(default)]
    pub org_type: String,
}

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::OrganizationAdd => {
            add(msg).await?;
        }
        MessageType::OrganizationRename => {
            rename(msg).await?;
        }
        MessageType::OrganizationDelete => {
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
    let org: Organization = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = organizations::add(
        &org.identifier,
        &org.name,
        org.org_type.as_str().parse().unwrap(),
    )
    .await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to add organization: {}, error: {}",
            org.name,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, json::to_vec(&org).unwrap().into(), true, None).await;
    Ok(())
}

async fn rename(msg: Message) -> Result<()> {
    let org: Organization = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = organizations::rename(&org.identifier, &org.name).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to rename organization: {}, error: {}",
            org.name,
            e
        );
        return Err(e);
    }
    let _ = put_into_db_coordinator(&msg.key, json::to_vec(&org).unwrap().into(), true, None).await;
    Ok(())
}

async fn delete(msg: Message) -> Result<()> {
    let keys = msg.key.split('/').collect::<Vec<_>>();
    if keys.len() != 3 {
        log::error!(
            "[SUPER_CLUSTER:sync] Invalid key: {}, expected 3 parts, got {}",
            msg.key,
            keys.len()
        );
        return Err(Error::Message("Invalid key".to_string()));
    }
    let org_id = keys[2];
    if let Err(e) = organizations::remove(org_id).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to delete scheduler: {}, error: {}",
            org_id,
            e
        );
        return Err(e);
    }
    let _ = delete_from_db_coordinator(&msg.key, false, msg.need_watch, None).await;
    Ok(())
}
