// Copyright 2024 OpenObserve Inc.
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
    errors,
    table::{
        entity::search_job_partitions::Model,
        search_job::{
            common::{OperatorType, Value},
            search_job_partitions::{Filter, MetaColumn, SetOperator},
        },
    },
};
#[cfg(feature = "enterprise")]
use {
    infra::table::search_job::search_job_partitions::PartitionJobOperator,
    o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config,
    o2_enterprise::enterprise::super_cluster,
};

pub async fn submit_partitions(job_id: &str, partitions: &[[i64; 2]]) -> Result<(), errors::Error> {
    let created_at = chrono::Utc::now().timestamp_micros();
    let mut jobs = Vec::with_capacity(partitions.len());
    for (idx, partition) in partitions.iter().enumerate() {
        jobs.push(Model {
            job_id: job_id.to_string(),
            partition_id: idx as i64,
            start_time: partition[0],
            end_time: partition[1],
            created_at,
            status: 0,
            started_at: None,
            ended_at: None,
            cluster: None,
            result_path: None,
            error_message: None,
        });
    }

    infra::table::search_job::search_job_partitions::submit_partitions(job_id, jobs.clone())
        .await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_partition_operator(PartitionJobOperator::Submit {
            job_id: job_id.to_string(),
            jobs: Box::new(jobs),
        })
        .await
        .map_err(|e| {
            errors::Error::Message(format!(
                "super cluster search job partition submit error: {e}"
            ))
        })?;
    }

    Ok(())
}

pub async fn get_partition_jobs(job_id: &str) -> Result<Vec<Model>, errors::Error> {
    infra::table::search_job::search_job_partitions::get_partition_jobs(job_id).await
}

pub async fn set_partition_job_start(job_id: &str, partition_id: i64) -> Result<(), errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();

    let operator = SetOperator {
        filter: vec![
            Filter::new(
                MetaColumn::JobId,
                OperatorType::Equal,
                Value::string(job_id),
            ),
            Filter::new(
                MetaColumn::PartitionId,
                OperatorType::Equal,
                Value::i64(partition_id),
            ),
        ],
        update: vec![
            (MetaColumn::Status, Value::i64(1)),
            (MetaColumn::StartedAt, Value::i64(updated_at)),
            (
                MetaColumn::Cluster,
                Value::string(config::get_cluster_name().as_str()),
            ),
        ],
    };

    infra::table::search_job::search_job_partitions::set(operator.clone()).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_partition_operator(PartitionJobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job partition set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn set_partition_job_finish(
    job_id: &str,
    partition_id: i64,
    path: &str,
) -> Result<(), errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let operator = SetOperator {
        filter: vec![
            Filter::new(
                MetaColumn::JobId,
                OperatorType::Equal,
                Value::string(job_id),
            ),
            Filter::new(
                MetaColumn::PartitionId,
                OperatorType::Equal,
                Value::i64(partition_id),
            ),
        ],
        update: vec![
            (MetaColumn::Status, Value::i64(2)),
            (MetaColumn::EndedAt, Value::i64(updated_at)),
            (MetaColumn::ResultPath, Value::string(path)),
        ],
    };

    infra::table::search_job::search_job_partitions::set(operator.clone()).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_partition_operator(PartitionJobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job partition set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn set_partition_job_error_message(
    job_id: &str,
    partition_id: i64,
    error_message: &str,
) -> Result<(), errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let operator = SetOperator {
        filter: vec![
            Filter::new(
                MetaColumn::JobId,
                OperatorType::Equal,
                Value::string(job_id),
            ),
            Filter::new(
                MetaColumn::PartitionId,
                OperatorType::Equal,
                Value::i64(partition_id),
            ),
        ],
        update: vec![
            (MetaColumn::Status, Value::i64(2)),
            (MetaColumn::EndedAt, Value::i64(updated_at)),
            (MetaColumn::ErrorMessage, Value::string(error_message)),
        ],
    };

    infra::table::search_job::search_job_partitions::set(operator.clone()).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_partition_operator(PartitionJobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job partition set error: {e}"))
            })?;
    }

    Ok(())
}

// query search_job_partitions table
pub async fn cancel_partition_job(job_id: &str) -> Result<(), errors::Error> {
    let operator = SetOperator {
        filter: vec![Filter::new(
            MetaColumn::JobId,
            OperatorType::Equal,
            Value::string(job_id),
        )],
        update: vec![(MetaColumn::Status, Value::i64(3))],
    };

    infra::table::search_job::search_job_partitions::set(operator.clone()).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_partition_operator(PartitionJobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job partition set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn clean_deleted_partition_job(job_id: &str) -> Result<(), errors::Error> {
    infra::table::search_job::search_job_partitions::clean_deleted_partition_job(job_id).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_partition_operator(PartitionJobOperator::Delete {
            job_id: job_id.to_string(),
        })
        .await
        .map_err(|e| {
            errors::Error::Message(format!(
                "super cluster search job partition delete error: {e}"
            ))
        })?;
    }

    Ok(())
}
