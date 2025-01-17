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

pub mod search_job_partitions;
pub mod search_job_results;
pub mod search_jobs;

use config::utils::json;
use infra::{
    errors::{Error, Result},
    table::search_job::{
        search_job_partitions::PartitionJobOperator, search_job_results::JobResultOperator,
        search_jobs::JobOperator,
    },
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::SearchJob => {
            let operator: JobOperator = json::from_slice(&msg.value.unwrap())?;
            search_jobs::process(operator).await?;
        }
        MessageType::SearchJobPartition => {
            let operator: PartitionJobOperator = json::from_slice(&msg.value.unwrap())?;
            search_job_partitions::process(operator).await?;
        }
        MessageType::SearchJobResult => {
            let operator: JobResultOperator = json::from_slice(&msg.value.unwrap())?;
            search_job_results::process(operator).await?;
        }
        _ => {
            log::error!(
                "[SUPER_CLUSTER:DB] Invalid message for search job: type: {:?}, key: {}",
                msg.message_type,
                msg.key
            );
            return Err(Error::Message("Invalid message type".to_string()));
        }
    }
    Ok(())
}
