// Copyright 2025 OpenObserve Inc.
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

use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};
use infra::errors::Error;
use infra::table::ratelimit::RatelimitRule;


pub async fn process(msg: Message) -> Result<(), anyhow::Error> {
    log::debug!(
        "[SUPER_CLUSTER:RATELIMIT] LOCAL_NODE:{:?}, Processing message: {:?}",
        config::cluster::LOCAL_NODE,
        msg.key
    );

    let bytes = msg
        .value
        .ok_or(Error::Message("Message missing value".to_string()))?;
    let rule = RatelimitRule::try_from(&bytes)?;

    match msg.message_type {
        MessageType::RatelimitAdd => {
            infra::table::ratelimit::add(rule).await?;
        }
        MessageType::RatelimitUpdate => {
            infra::table::ratelimit::update(rule).await?;
        }
        MessageType::RatelimitDelete => {
            infra::table::ratelimit::delete(rule).await?;
        }
        _ => {
            log::error!(
                "[SUPER_CLUSTER:RATELIMIT] Invalid message: type: {:?}, key: {}",
                msg.message_type,
                msg.key
            );
            return Err(anyhow::anyhow!("Invalid message type".to_string()));
        }
    }
    Ok(())
}