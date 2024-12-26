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

use config::ider;
use infra::{
    errors, orm_err,
    table::{
        entity::search_jobs::Model,
        search_job::{
            common::{OperatorType, Value},
            search_jobs::{Filter, MetaColumn, SetOperator},
        },
    },
};
#[cfg(feature = "enterprise")]
use {
    config::cluster::LOCAL_NODE, infra::table::search_job::search_jobs::JobOperator,
    o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config,
    o2_enterprise::enterprise::super_cluster,
};

#[allow(clippy::too_many_arguments)]
pub async fn submit(
    trace_id: &str,
    org_id: &str,
    user_id: &str,
    stream_type: &str,
    stream_names: &str,
    payload: &str,
    start_time: i64,
    end_time: i64,
) -> Result<String, errors::Error> {
    let job_id = ider::uuid();
    let created_at = chrono::Utc::now().timestamp_micros();
    let updated_at = created_at;
    let job = Model {
        id: job_id.to_string(),
        trace_id: trace_id.to_string(),
        org_id: org_id.to_string(),
        user_id: user_id.to_string(),
        stream_type: stream_type.to_string(),
        stream_names: stream_names.to_string(),
        payload: payload.to_string(),
        start_time,
        end_time,
        created_at,
        updated_at,
        status: 0,
        partition_num: None,
        started_at: None,
        ended_at: None,
        node: None,
        cluster: None,
        result_path: None,
        error_message: None,
    };

    infra::table::search_job::search_jobs::submit(job.clone().into()).await?;

    // super cluster, add a new job
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Submit(job.into()))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job submit error: {e}"))
            })?;
    }

    Ok(job_id)
}

// get a oldest job and lock it
pub async fn get_job() -> Result<Option<Model>, errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let res = infra::table::search_job::search_jobs::get_job(updated_at).await?;

    // TODO: need other call to set the get job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled && res.is_some() {
        let job_id = res.as_ref().unwrap().clone();
        super_cluster::queue::search_job_operator(JobOperator::GetJob {
            job_id: job_id.id.clone(),
            updated_at,
            cluster: config::get_cluster_name(),
            node: LOCAL_NODE.uuid.clone(),
        })
        .await
        .map_err(|e| errors::Error::Message(format!("super cluster search job get error: {e}")))?;
    }

    Ok(res)
}

pub async fn cancel_job_by_job_id(job_id: &str) -> Result<i64, errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let res = infra::table::search_job::search_jobs::cancel_job(job_id, updated_at).await?;

    // super cluster, cancel a job
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Cancel {
            job_id: job_id.to_string(),
            updated_at,
        })
        .await
        .map_err(|e| {
            errors::Error::Message(format!("super cluster search job cancel error: {e}"))
        })?;
    }
    Ok(res)
}

pub async fn set_job_error_message(job_id: &str, error_message: &str) -> Result<(), errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let operator = SetOperator {
        filter: vec![Filter::new(
            MetaColumn::Id,
            OperatorType::Equal,
            Value::string(job_id),
        )],
        update: vec![
            (MetaColumn::ErrorMessage, Value::string(error_message)),
            (MetaColumn::EndedAt, Value::i64(updated_at)),
            (MetaColumn::Status, Value::i64(2)),
        ],
    };
    infra::table::search_job::search_jobs::set(operator.clone()).await?;

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn set_job_finish(job_id: &str, path: &str) -> Result<(), errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let operator = SetOperator {
        filter: vec![
            Filter::new(MetaColumn::Id, OperatorType::Equal, Value::string(job_id)),
            Filter::new(MetaColumn::Status, OperatorType::Equal, Value::i64(1)),
        ],
        update: vec![
            (MetaColumn::Status, Value::i64(2)),
            (MetaColumn::ResultPath, Value::string(path)),
            (MetaColumn::UpdatedAt, Value::i64(updated_at)),
            (MetaColumn::EndedAt, Value::i64(updated_at)),
        ],
    };
    let res = infra::table::search_job::search_jobs::set(operator.clone()).await;

    match res {
        Ok(res) if res.rows_affected == 1 => {}
        Ok(_) => return orm_err!("job_id not found or status is not running"),
        Err(e) => return orm_err!(format!("set job finish error: {e}")),
    }

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn set_partition_num(job_id: &str, partition_num: i64) -> Result<(), errors::Error> {
    let operator = SetOperator {
        filter: vec![Filter::new(
            MetaColumn::Id,
            OperatorType::Equal,
            Value::string(job_id),
        )],
        update: vec![(MetaColumn::PartitionNum, Value::i64(partition_num))],
    };

    infra::table::search_job::search_jobs::set(operator.clone()).await?;

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn set_job_deleted(job_id: &str) -> Result<bool, errors::Error> {
    let operator = SetOperator {
        filter: vec![Filter::new(
            MetaColumn::Id,
            OperatorType::Equal,
            Value::string(job_id),
        )],
        update: vec![(MetaColumn::Status, Value::i64(4))],
    };

    let res = infra::table::search_job::search_jobs::set(operator.clone()).await;
    let res = match res {
        Ok(res) if res.rows_affected == 1 => true,
        Ok(_) => false,
        Err(e) => return orm_err!(format!("set job deleted error: {e}")),
    };

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job set error: {e}"))
            })?;
    }

    Ok(res)
}

pub async fn update_running_job(job_id: &str) -> Result<(), errors::Error> {
    let updated_at = chrono::Utc::now().timestamp_micros();
    let operator = SetOperator {
        filter: vec![
            Filter::new(MetaColumn::Id, OperatorType::Equal, Value::string(job_id)),
            Filter::new(MetaColumn::Status, OperatorType::Equal, Value::i64(1)),
        ],
        update: vec![(MetaColumn::UpdatedAt, Value::i64(updated_at))],
    };

    infra::table::search_job::search_jobs::set(operator.clone()).await?;

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn check_running_jobs(updated_at: i64) -> Result<(), errors::Error> {
    let operator = SetOperator {
        filter: vec![
            Filter::new(MetaColumn::Status, OperatorType::Equal, Value::i64(1)),
            Filter::new(
                MetaColumn::UpdatedAt,
                OperatorType::LessThan,
                Value::i64(updated_at),
            ),
        ],
        update: vec![(MetaColumn::Status, Value::i64(0))],
    };

    infra::table::search_job::search_jobs::set(operator.clone()).await?;

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Set(operator))
            .await
            .map_err(|e| {
                errors::Error::Message(format!("super cluster search job set error: {e}"))
            })?;
    }

    Ok(())
}

pub async fn clean_deleted_job(job_id: &str) -> Result<(), errors::Error> {
    infra::table::search_job::search_jobs::clean_deleted_job(job_id).await?;

    // super cluster, set the job's status
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Delete {
            job_id: job_id.to_string(),
        })
        .await
        .map_err(|e| {
            errors::Error::Message(format!("super cluster search job delete error: {e}"))
        })?;
    }

    Ok(())
}

pub async fn retry_search_job(job_id: &str) -> Result<(), errors::Error> {
    let trace_id = ider::uuid();
    let updated_at = chrono::Utc::now().timestamp_micros();
    infra::table::search_job::search_jobs::retry_search_job(job_id, &trace_id, updated_at).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::search_job_operator(JobOperator::Retry {
            job_id: job_id.to_string(),
            new_trace_id: trace_id.to_string(),
            updated_at,
        })
        .await
        .map_err(|e| {
            errors::Error::Message(format!("super cluster search job retry error: {e}"))
        })?;
    }

    Ok(())
}

pub async fn get(job_id: &str, org_id: &str) -> Result<Model, errors::Error> {
    infra::table::search_job::search_jobs::get(job_id, org_id).await
}

pub async fn list_status_by_org_id(org_id: &str) -> Result<Vec<Model>, errors::Error> {
    infra::table::search_job::search_jobs::list_status_by_org_id(org_id).await
}

pub async fn get_deleted_jobs() -> Result<Vec<Model>, errors::Error> {
    infra::table::search_job::search_jobs::get_deleted_jobs().await
}
