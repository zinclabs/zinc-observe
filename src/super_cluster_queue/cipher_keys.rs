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

use infra::{
    errors::{Error, Result},
    table::cipher::EntryKind,
};
use o2_enterprise::enterprise::super_cluster::queue::{KeysMessage, Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::KeysTable => {
            let actual_message: KeysMessage = serde_json::from_slice(&msg.value.unwrap())?;
            match actual_message {
                KeysMessage::Delete { name, org } => {
                    log::info!("[SUPER_CLUSTER:DB] deleting key {}/{}", org, name);
                    infra::table::cipher::remove(&org, EntryKind::CipherKey, &name).await?;
                }
                KeysMessage::Put { entry } => {
                    log::info!("[SUPER_CLUSTER:DB] adding key {}/{}", entry.org, entry.name);
                    infra::table::cipher::add(entry).await?;
                }
                KeysMessage::Update { entry } => {
                    log::info!(
                        "[SUPER_CLUSTER:DB] updating key {}/{}",
                        entry.org,
                        entry.name
                    );
                    infra::table::cipher::update(entry).await?;
                }
            }
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
