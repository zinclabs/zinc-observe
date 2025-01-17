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
    table::search_job::search_job_results::{JobResultOperator, *},
};

pub(crate) async fn process(operator: JobResultOperator) -> Result<()> {
    match operator {
        JobResultOperator::Delete { job_id } => {
            if let Err(e) = clean_deleted_job_result(job_id.as_str()).await {
                log::error!(
                    "[SUPER_CLUSTER:DB] Failed to clean deleted job result: {job_id}, error: {e}",
                );
                return Err(e);
            }
        }
    }
    Ok(())
}
