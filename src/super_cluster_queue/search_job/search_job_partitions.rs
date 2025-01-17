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

use infra::{errors::Result, table::search_job::search_job_partitions::*};

pub(crate) async fn process(operator: PartitionJobOperator) -> Result<()> {
    match operator {
        PartitionJobOperator::Submit { job_id, jobs } => {
            if let Err(e) = submit_partitions(job_id.as_str(), *jobs).await {
                log::error!(
                    "[SUPER_CLUSTER:DB] Failed to submit partition job: {job_id}, error: {e}",
                );
                return Err(e);
            }
        }
        PartitionJobOperator::Set(operator) => {
            if let Err(e) = set(operator.clone()).await {
                log::error!(
                    "[SUPER_CLUSTER:DB] Failed to set partition job: {operator:?}, error: {e}",
                );
                return Err(e);
            }
        }
        PartitionJobOperator::Delete { job_id } => {
            if let Err(e) = clean_deleted_partition_job(job_id.as_str()).await {
                log::error!(
                    "[SUPER_CLUSTER:DB] Failed to clean deleted partition job: {job_id}, error: {e}",
                );
                return Err(e);
            }
        }
    }
    Ok(())
}
