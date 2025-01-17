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

use arrow::datatypes::Schema;
use async_recursion::async_recursion;
use config::{meta::stream::StreamType, utils::json};
use infra::errors::{Error, Result};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    let db = infra::db::get_db().await;
    match msg.message_type {
        MessageType::Delete(with_prefix) => {
            log::debug!("[SUPER_CLUSTER:sync] Schema Delete: {}", msg.key);
            db.delete(&msg.key, with_prefix, msg.need_watch, msg.start_dt)
                .await?;
        }
        MessageType::Put => {
            log::debug!("[SUPER_CLUSTER:sync] Schema Put: {}", msg.key);
            merge(msg).await?;
        }
        MessageType::SchemaMerge => {
            log::debug!("[SUPER_CLUSTER:sync] Schema Merge: {}", msg.key);
            merge(msg).await?;
        }
        MessageType::SchemaSetting => {
            log::debug!("[SUPER_CLUSTER:sync] Schema Setting: {}", msg.key);
            setting(msg).await?;
        }
        MessageType::SchemaDeleteFields => {
            log::debug!("[SUPER_CLUSTER:sync] Schema DeleteFields: {}", msg.key);
            delete_fields(msg).await?;
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

#[async_recursion]
async fn merge(msg: Message) -> Result<()> {
    let (org_id, stream_type, stream_name) = parse_key(&msg.key)?;
    let schema: Schema = json::from_slice(msg.value.as_ref().unwrap())?;
    if let Err(e) =
        infra::schema::merge(&org_id, &stream_name, stream_type, &schema, msg.start_dt).await
    {
        if e.to_string().to_lowercase().contains("duplicate key")
            && e.to_string()
                .to_lowercase()
                .contains("meta_module_start_dt_idx")
        {
            log::warn!("[SUPER_CLUSTER:sync] Duplicate start_dt, retrying with new start_dt");
            let msg = Message {
                key: msg.key,
                value: msg.value,
                start_dt: Some(chrono::Utc::now().timestamp_micros()),
                need_watch: msg.need_watch,
                message_type: msg.message_type,
            };
            return merge(msg).await;
        }
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to merge schema for stream: {}/{}/{}, start_dt: {:?}, error: {}",
            org_id,
            stream_type,
            stream_name,
            msg.start_dt,
            e
        );
        return Err(e);
    }
    Ok(())
}

async fn setting(msg: Message) -> Result<()> {
    let (org_id, stream_type, stream_name) = parse_key(&msg.key)?;
    let metadata: std::collections::HashMap<String, String> =
        json::from_slice(&msg.value.unwrap())?;
    if let Err(e) =
        infra::schema::update_setting(&org_id, &stream_name, stream_type, metadata.clone()).await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update setting for stream: {}/{}/{}, metadata: {:?}, error: {}",
            org_id,
            stream_type,
            stream_name,
            metadata,
            e
        );
        return Err(e);
    }
    Ok(())
}

async fn delete_fields(msg: Message) -> Result<()> {
    let (org_id, stream_type, stream_name) = parse_key(&msg.key)?;
    let fields: Vec<String> = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) =
        infra::schema::delete_fields(&org_id, &stream_name, stream_type, fields.clone()).await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to delete fields for stream: {}/{}/{}, fields: {:?}, error: {}",
            org_id,
            stream_type,
            stream_name,
            fields,
            e
        );
        return Err(e);
    }
    Ok(())
}

fn parse_key(key: &str) -> Result<(String, StreamType, String)> {
    let key_columns = key.split('/').collect::<Vec<&str>>();
    if key_columns.len() < 5 {
        log::error!("[SUPER_CLUSTER:sync] Invalid key format: {}", key);
        return Err(Error::Message("Invalid key format".to_string()));
    }
    let org_id = key_columns[2].into();
    let stream_type = key_columns[3].into();
    let stream_name = key_columns[4].into();
    Ok((org_id, stream_type, stream_name))
}
