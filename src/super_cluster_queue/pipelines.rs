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

use config::{meta::pipeline::Pipeline, utils::json};
use infra::errors::{Error, Result};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::PipelinePut => {
            let bytes = msg
                .value
                .ok_or(Error::Message("Message missing value".to_string()))?;
            let pipeline: Pipeline = json::from_slice(&bytes).inspect_err(|e| {
                log::error!(
                    "[SUPER_CLUSTER:PIPELINE] Failed to deserialize message value to pipeline: {}",
                    e
                );
            })?;
            infra::pipeline::put(&pipeline).await?;
            infra::cluster_coordinator::pipelines::emit_put_event(&pipeline.id).await?;
        }
        MessageType::PipelineDelete => {
            let pipeline_id = parse_key(&msg.key)?;
            infra::cluster_coordinator::pipelines::emit_delete_event(&pipeline_id).await?;
            infra::pipeline::delete(&pipeline_id).await?;
        }
        _ => {
            log::error!(
                "[SUPER_CLUSTER:PIPELINE] Invalid message: type: {:?}, key: {}",
                msg.message_type,
                msg.key
            );
            return Err(Error::Message("Invalid message type".to_string()));
        }
    }
    Ok(())
}

fn parse_key(key: &str) -> Result<String> {
    let key_columns: Vec<&str> = key.split('/').collect();
    if key_columns.len() != 3 || key_columns[2].is_empty() {
        return Err(Error::Message("Invalid key".to_string()));
    }
    Ok(key_columns[2].into())
}
