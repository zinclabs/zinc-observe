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
    table::distinct_values::{BatchDeleteMessage, DistinctFieldRecord},
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::DistinctValuesPut => {
            let record: DistinctFieldRecord = serde_json::from_slice(&msg.value.unwrap())?;
            log::info!(
                "[SUPER_CLUSTER:DB] adding distinct value {}/{}/{}",
                record.org_name,
                record.stream_name,
                record.field_name
            );
            infra::table::distinct_values::add(record).await?;
        }
        MessageType::DistinctValuesDelete(true) => {
            let val: BatchDeleteMessage = serde_json::from_slice(&msg.value.unwrap())?;
            log::info!(
                "[SUPER_CLUSTER:DB] batch removing {:?}:{}",
                val.origin_type,
                val.id
            );
            infra::table::distinct_values::batch_remove(val.origin_type, &val.id).await?;
        }
        MessageType::DistinctValuesDelete(false) => {
            let record: DistinctFieldRecord = serde_json::from_slice(&msg.value.unwrap())?;
            log::info!(
                "[SUPER_CLUSTER:DB] removing distinct value {}/{}/{}",
                record.org_name,
                record.stream_name,
                record.field_name
            );
            infra::table::distinct_values::remove(record).await?;
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
