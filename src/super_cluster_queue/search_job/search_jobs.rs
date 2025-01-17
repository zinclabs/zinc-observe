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
    errors::Result,
    table::search_job::search_jobs::{JobOperator, *},
};

pub(crate) async fn process(operator: JobOperator) -> Result<()> {
    match operator {
        JobOperator::Submit(job) => {
            if let Err(e) = submit((*job).clone().into()).await {
                log::error!(
                    "[SUPER_CLUSTER:DB] Failed to submit job: {}, error: {e}",
                    job.id,
                );
                return Err(e);
            }
        }
        JobOperator::Set(operator) => {
            if let Err(e) = set(operator.clone()).await {
                log::error!("[SUPER_CLUSTER:DB] Failed to set job: {operator:?}, error: {e}",);
                return Err(e);
            }
        }
        JobOperator::Cancel { job_id, updated_at } => {
            if let Err(e) = cancel_job(job_id.as_str(), updated_at).await {
                log::error!("[SUPER_CLUSTER:DB] Failed to cancel job: {job_id}, error: {e}",);
                return Err(e);
            }
        }
        JobOperator::GetJob {
            job_id,
            cluster,
            node,
            updated_at,
        } => {
            if let Err(e) =
                set_job_start(job_id.as_str(), cluster.as_str(), node.as_str(), updated_at).await
            {
                log::error!("[SUPER_CLUSTER:DB] Failed to set job start: {job_id}, error: {e}",);
                return Err(e);
            }
        }
        JobOperator::Delete { job_id } => {
            if let Err(e) = clean_deleted_job(job_id.as_str()).await {
                log::error!("[SUPER_CLUSTER:DB] Failed to clean deleted job: {job_id}, error: {e}",);
                return Err(e);
            }
        }
        JobOperator::Retry {
            job_id,
            new_trace_id,
            updated_at,
        } => {
            if let Err(e) =
                retry_search_job(job_id.as_str(), new_trace_id.as_str(), updated_at).await
            {
                log::error!("[SUPER_CLUSTER:DB] Failed to retry job: {job_id}, error: {e}",);
                return Err(e);
            }
        }
    }
    Ok(())
}
