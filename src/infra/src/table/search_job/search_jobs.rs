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

use config::cluster::LOCAL_NODE;
use sea_orm::{
    prelude::Expr,
    sea_query::{Keyword, LockType, SimpleExpr},
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait, UpdateMany, UpdateResult,
};
use serde::{Deserialize, Serialize};

use super::{
    super::{
        entity::{
            search_job_partitions::{Column as PartitionJobColumn, Entity as PartitionJobEntity},
            search_job_results::{ActiveModel as JobResultModel, Entity as JobResultEntity},
            search_jobs::*,
        },
        get_lock,
    },
    common::{OperatorType, Value},
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors, orm_err,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum JobOperator {
    Submit(Box<Model>),
    Set(SetOperator),
    GetJob {
        job_id: String,
        cluster: String,
        node: String,
        updated_at: i64,
    },
    Cancel {
        job_id: String,
        updated_at: i64,
    },
    Delete {
        job_id: String,
    },
    Retry {
        job_id: String,
        new_trace_id: String,
        updated_at: i64,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MetaColumn {
    Id,
    TraceId,
    OrgId,
    UserId,
    StreamType,
    StreamNames,
    Payload,
    StartTime,
    EndTime,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
    Cluster,
    Node,
    Status,
    ResultPath,
    ErrorMessage,
    PartitionNum,
}

impl From<MetaColumn> for Column {
    fn from(column: MetaColumn) -> Self {
        match column {
            MetaColumn::Id => Column::Id,
            MetaColumn::TraceId => Column::TraceId,
            MetaColumn::OrgId => Column::OrgId,
            MetaColumn::UserId => Column::UserId,
            MetaColumn::StreamType => Column::StreamType,
            MetaColumn::StreamNames => Column::StreamNames,
            MetaColumn::Payload => Column::Payload,
            MetaColumn::StartTime => Column::StartTime,
            MetaColumn::EndTime => Column::EndTime,
            MetaColumn::CreatedAt => Column::CreatedAt,
            MetaColumn::UpdatedAt => Column::UpdatedAt,
            MetaColumn::StartedAt => Column::StartedAt,
            MetaColumn::EndedAt => Column::EndedAt,
            MetaColumn::Cluster => Column::Cluster,
            MetaColumn::Node => Column::Node,
            MetaColumn::Status => Column::Status,
            MetaColumn::ResultPath => Column::ResultPath,
            MetaColumn::ErrorMessage => Column::ErrorMessage,
            MetaColumn::PartitionNum => Column::PartitionNum,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Filter {
    pub left: MetaColumn,
    pub operator: OperatorType,
    pub right: Value,
}

impl Filter {
    pub fn new(left: MetaColumn, operator: OperatorType, right: Value) -> Self {
        Self {
            left,
            operator,
            right,
        }
    }
}

impl From<Filter> for SimpleExpr {
    fn from(filter: Filter) -> Self {
        let left: Column = filter.left.into();

        match filter.right {
            Value::String(s) => match filter.operator {
                OperatorType::Equal => left.eq(s),
                OperatorType::NotEqual => left.ne(s),
                OperatorType::GreaterThan => left.gt(s),
                OperatorType::LessThan => left.lt(s),
            },
            Value::I64(i) => match filter.operator {
                OperatorType::Equal => left.eq(i),
                OperatorType::NotEqual => left.ne(i),
                OperatorType::GreaterThan => left.gt(i),
                OperatorType::LessThan => left.lt(i),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetOperator {
    pub filter: Vec<Filter>,
    pub update: Vec<(MetaColumn, Value)>,
}

// TODO: use enum to represent the status
// in search_jobs table
// status 0: pending
// status 1: running
// status 2: finish
// status 3: cancel
// status 4: delete

#[allow(clippy::too_many_arguments)]
pub async fn submit(job: ActiveModel) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let _res = match Entity::insert(job).exec(client).await {
        Ok(res) => res,
        Err(e) => return orm_err!(format!("submit search job error: {e}")),
    };

    Ok(())
}

// get the job and update status
pub async fn get_job(updated_at: i64) -> Result<Option<Model>, errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let tx = match client.begin().await {
        Ok(tx) => tx,
        Err(e) => return orm_err!(format!("get job start transaction error: {e}")),
    };
    // sql: select * from search_jobs where status = 0 order by created_at limit 1 for update
    let res = Entity::find()
        .lock(LockType::Update)
        .filter(Column::Status.eq(0))
        .order_by_asc(Column::CreatedAt)
        .one(&tx)
        .await;

    let model = match res {
        Ok(model) => model,
        Err(e) => {
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("get job rollback error: {e}"));
            };
            return orm_err!(format!("get job error: {e}"));
        }
    };

    if let Some(model) = &model {
        if let Err(e) = Entity::update_many()
            .col_expr(Column::Status, Expr::value(1))
            .col_expr(Column::UpdatedAt, Expr::value(updated_at))
            .col_expr(Column::StartedAt, Expr::value(updated_at))
            .col_expr(Column::Cluster, Expr::value(config::get_cluster_name()))
            .col_expr(Column::Node, Expr::value(&LOCAL_NODE.uuid))
            .filter(Column::Id.eq(&model.id))
            .exec(&tx)
            .await
        {
            log::error!("get job update status error: {e}");
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("get job rollback update status error: {e}"));
            }
            return orm_err!(format!("get job update status error: {e}"));
        }
    }

    if let Err(e) = tx.commit().await {
        return orm_err!(format!("get job commit error: {e}"));
    }

    Ok(model)
}

pub async fn cancel_job(job_id: &str, update_at: i64) -> Result<i64, errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let tx = match client.begin().await {
        Ok(tx) => tx,
        Err(e) => return orm_err!(format!("cancel job start transaction error: {e}")),
    };

    let res = Entity::find()
        .filter(Column::Id.eq(job_id))
        .lock(LockType::Update)
        .one(&tx)
        .await;

    let res = match res {
        Ok(status) => status,
        Err(e) => {
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("cancel job rollback error: {e}"));
            };
            return orm_err!(format!("cancel job get status error: {e}"));
        }
    };

    if let Some(res) = &res {
        if res.status == 1 || res.status == 0 {
            if let Err(e) = Entity::update_many()
                .col_expr(Column::Status, Expr::value(3))
                .col_expr(Column::EndedAt, Expr::value(update_at))
                .filter(Column::Id.eq(job_id))
                .exec(&tx)
                .await
            {
                log::error!("cancel job update status error: {e}");
                if let Err(e) = tx.rollback().await {
                    return orm_err!(format!("cancel job rollback update status error: {e}"));
                }
                return orm_err!(format!("cancel job update status error: {e}"));
            }
        } else {
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("cancel job rollback error: {e}"));
            }
            return orm_err!(format!(
                "job_id: {job_id} status is not pending or running, can not cancel"
            ));
        }
    } else {
        if let Err(e) = tx.rollback().await {
            return orm_err!(format!("cancel job rollback error: {e}"));
        }
        return orm_err!(format!("job_id: {job_id} is not found"));
    }

    if let Err(e) = tx.commit().await {
        return orm_err!(format!("cancel job commit error: {e}"));
    }

    Ok(res.unwrap().status)
}

pub async fn set(operator: SetOperator) -> Result<UpdateResult, errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let mut query = Entity::update_many();

    for (column, value) in operator.update.clone() {
        let column: Column = column.into();
        match value {
            Value::String(s) => {
                query = query.col_expr(column, Expr::value(s));
            }
            Value::I64(i) => {
                query = query.col_expr(column, Expr::value(i));
            }
        }
    }

    for filter in operator.filter.clone() {
        let filter: SimpleExpr = filter.into();
        query = query.filter(filter);
    }

    let res = query.exec(client).await;

    match res {
        Ok(res) => Ok(res),
        Err(e) => orm_err!(format!(
            "set operator: {operator:?} in search job table error: {e}"
        )),
    }
}

pub async fn clean_deleted_job(job_id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let res = Entity::delete_many()
        .filter(Column::Id.eq(job_id))
        .exec(client)
        .await;

    if let Err(e) = res {
        return orm_err!(format!("clean deleted jobs error: {e}"));
    }

    Ok(())
}

// 1. start a transaction
// 2. move the status from job table -> job result table,
// 3. clean old status: trace_id updated_at started_at ended_at node status result_path
//    error_message partition_num
// 4. generate the new trace_id,
// 5. make the job as pending status
// 6. commit the transaction
// NOTE: this function can ensure,
// 1. for finished job, it reset all partition job's status, result_path, error_message
// 2. for failed job, it reset faild job and all pending job's status, result_path, error_message
pub async fn retry_search_job(
    job_id: &str,
    new_trace_id: &str,
    updated_at: i64,
) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let tx = match client.begin().await {
        Ok(tx) => tx,
        Err(e) => return orm_err!(format!("retry job start transaction error: {e}")),
    };

    let res = Entity::find()
        .filter(Column::Id.eq(job_id))
        .lock(LockType::Update)
        .one(&tx)
        .await;

    let res = match res {
        Ok(Some(res)) => res,
        Ok(None) => {
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("retry job rollback error: {e}"));
            }
            return orm_err!("job_id not found");
        }
        Err(e) => {
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("retry job rollback error: {e}"));
            }
            return orm_err!(format!("retry job get job error: {e}"));
        }
    };

    // move the status from job table -> job result table
    let record = JobResultModel {
        job_id: Set(res.id.clone()),
        trace_id: Set(res.trace_id.clone()),
        started_at: Set(res.started_at),
        ended_at: Set(res.ended_at),
        cluster: Set(res.cluster.clone()),
        result_path: Set(res.result_path.clone()),
        error_message: Set(res.error_message.clone()),
    };

    // insert into job result table
    if let Err(e) = JobResultEntity::insert(record).exec(&tx).await {
        if let Err(e) = tx.rollback().await {
            return orm_err!(format!("retry job rollback error: {e}"));
        }
        return orm_err!(format!("retry job insert job result error: {e}"));
    };

    // reset all error job's status, result_path, error_message
    let mut query = generate_reset_partition_job_query(job_id);
    query = query.filter(PartitionJobColumn::ErrorMessage.is_not_null());
    let result = query.exec(&tx).await;
    if let Err(e) = result {
        if let Err(e) = tx.rollback().await {
            return orm_err!(format!("retry job rollback error: {e}"));
        }
        return orm_err!(format!("retry job update partition job error: {e}"));
    }

    // reset all finish job's status, result_path, error_message in partition job table
    if res.result_path.is_some() {
        let query = generate_reset_partition_job_query(job_id);
        let result = query.exec(&tx).await;
        if let Err(e) = result {
            if let Err(e) = tx.rollback().await {
                return orm_err!(format!("retry job rollback error: {e}"));
            }
            return orm_err!(format!("retry job update partition job error: {e}"));
        }
    }

    let mut model: ActiveModel = res.into();
    model.trace_id = Set(new_trace_id.to_string());
    model.status = Set(0);
    model.updated_at = Set(updated_at);
    model.started_at = Set(None);
    model.ended_at = Set(None);
    model.node = Set(None);
    model.result_path = Set(None);
    model.error_message = Set(None);

    let res = model.update(&tx).await;

    if let Err(e) = res {
        if let Err(e) = tx.rollback().await {
            return orm_err!(format!("retry job rollback error: {e}"));
        }
        return orm_err!(format!("retry job update job error: {e}"));
    }

    if let Err(e) = tx.commit().await {
        return orm_err!(format!("retry job commit error: {e}"));
    }

    Ok(())
}

pub async fn get(job_id: &str, org_id: &str) -> Result<Model, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let res = Entity::find()
        .filter(Column::Id.eq(job_id))
        .filter(Column::OrgId.eq(org_id))
        .one(client)
        .await;

    match res {
        Ok(Some(res)) => Ok(res),
        Ok(None) => orm_err!(format!("job_id: {job_id} not found")),
        Err(e) => orm_err!(format!("get search job by job_id: {job_id} error: {e}")),
    }
}

pub async fn list_status_by_org_id(org_id: &str) -> Result<Vec<Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let res = Entity::find()
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Status.ne(4))
        .all(client)
        .await;

    let res = match res {
        Ok(res) => res,
        Err(e) => return orm_err!(format!("get search job by org_id error: {e}")),
    };

    Ok(res)
}

pub async fn get_deleted_jobs() -> Result<Vec<Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let res = Entity::find()
        .filter(Column::Status.eq(4))
        .all(client)
        .await;

    match res {
        Ok(res) => Ok(res),
        Err(e) => orm_err!(format!("get deleted jobs error: {e}")),
    }
}

fn generate_reset_partition_job_query(job_id: &str) -> UpdateMany<PartitionJobEntity> {
    PartitionJobEntity::update_many()
        .col_expr(
            PartitionJobColumn::StartedAt,
            SimpleExpr::Keyword(Keyword::Null),
        )
        .col_expr(
            PartitionJobColumn::EndedAt,
            SimpleExpr::Keyword(Keyword::Null),
        )
        .col_expr(PartitionJobColumn::Status, Expr::value(0))
        .col_expr(
            PartitionJobColumn::Cluster,
            SimpleExpr::Keyword(Keyword::Null),
        )
        .col_expr(
            PartitionJobColumn::ResultPath,
            SimpleExpr::Keyword(Keyword::Null),
        )
        .col_expr(
            PartitionJobColumn::ErrorMessage,
            SimpleExpr::Keyword(Keyword::Null),
        )
        .filter(PartitionJobColumn::JobId.eq(job_id))
}

// only used for super cluster sync
pub async fn set_job_start(
    job_id: &str,
    cluster: &str,
    node: &str,
    updated_at: i64,
) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let res = Entity::update_many()
        .col_expr(Column::Status, Expr::value(1))
        .col_expr(Column::StartedAt, Expr::value(updated_at))
        .col_expr(Column::UpdatedAt, Expr::value(updated_at))
        .col_expr(Column::Cluster, Expr::value(cluster))
        .col_expr(Column::Node, Expr::value(node))
        .filter(Column::Id.eq(job_id))
        .exec(client)
        .await;

    match res {
        Ok(_) => Ok(()),
        Err(e) => orm_err!(format!("set job start error: {e}")),
    }
}
