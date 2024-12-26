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

use sea_orm::{
    prelude::Expr,
    sea_query::{LockType, SimpleExpr},
    ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use super::{
    super::{entity::search_job_partitions::*, get_lock},
    common::{OperatorType, Value},
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors, orm_err,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PartitionJobOperator {
    Submit {
        job_id: String,
        jobs: Box<Vec<Model>>,
    },
    Set(SetOperator),
    Delete {
        job_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MetaColumn {
    JobId,
    PartitionId,
    StartTime,
    EndTime,
    CreatedAt,
    StartedAt,
    EndedAt,
    Cluster,
    Status,
    ResultPath,
    ErrorMessage,
}

impl From<MetaColumn> for Column {
    fn from(column: MetaColumn) -> Self {
        match column {
            MetaColumn::JobId => Column::JobId,
            MetaColumn::PartitionId => Column::PartitionId,
            MetaColumn::StartTime => Column::StartTime,
            MetaColumn::EndTime => Column::EndTime,
            MetaColumn::CreatedAt => Column::CreatedAt,
            MetaColumn::StartedAt => Column::StartedAt,
            MetaColumn::EndedAt => Column::EndedAt,
            MetaColumn::Cluster => Column::Cluster,
            MetaColumn::Status => Column::Status,
            MetaColumn::ResultPath => Column::ResultPath,
            MetaColumn::ErrorMessage => Column::ErrorMessage,
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
                _ => unreachable!("search_job_partition table only need equal"),
            },
            Value::I64(i) => match filter.operator {
                OperatorType::Equal => left.eq(i),
                _ => unreachable!("search_job_partition table only need equal"),
            },
        }
    }
}

// used for unify the set operation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetOperator {
    pub filter: Vec<Filter>,
    pub update: Vec<(MetaColumn, Value)>,
}

// in search_jobs table
// status 0: pending
// status 1: running
// status 2: finish
// status 3: cancel
// status 4: delete

pub async fn submit_partitions(job_id: &str, jobs: Vec<Model>) -> Result<(), errors::Error> {
    if jobs.is_empty() {
        return orm_err!("partitions array cannot be empty");
    }

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let tx = match client.begin().await {
        Ok(tx) => tx,
        Err(e) => return orm_err!(format!("submit partition job start transaction error: {e}")),
    };

    // sql: select * from search_job_partitions where job_id = job_id limit 1
    let status = Entity::find()
        .filter(Column::JobId.eq(job_id))
        .lock(LockType::Update)
        .one(client)
        .await;

    match status {
        Ok(Some(_)) => {
            // this mean we have already created partition jobs
            if let Err(e) = tx.commit().await {
                return orm_err!(format!("submit partition job commit error: {e}"));
            }
            return Ok(());
        }
        Err(e) => {
            if let Err(tx_err) = tx.rollback().await {
                return orm_err!(format!("submit partition job rollback error: {tx_err}"));
            }
            return orm_err!(format!("submit partition job check job_id error: {e}"));
        }
        Ok(None) => {}
    };

    let jobs: Vec<ActiveModel> = jobs.into_iter().map(|job| job.into()).collect::<Vec<_>>();
    let res = Entity::insert_many(jobs).exec(&tx).await;

    if let Err(e) = res {
        log::error!("submit partition job insert error: {}", e);
        if let Err(e) = tx.rollback().await {
            return orm_err!(format!("submit partition job rollback error: {e}"));
        }
        return orm_err!(format!("submit partition job insert error: {e}"));
    }

    if let Err(e) = tx.commit().await {
        return orm_err!(format!("submit partition job commit error: {e}"));
    }

    Ok(())
}

pub async fn get_partition_jobs(job_id: &str) -> Result<Vec<Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // sql: select * from search_job_partitions where job_id = job_id
    let res = Entity::find()
        .filter(Column::JobId.eq(job_id))
        .order_by(Column::PartitionId, Order::Asc)
        .all(client)
        .await;

    match res {
        Ok(jobs) => Ok(jobs),
        Err(e) => orm_err!(format!("get_partition_jobs_by_job_id failed: {}", e)),
    }
}

pub async fn set(operator: SetOperator) -> Result<(), errors::Error> {
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
        Ok(_) => Ok(()),
        Err(e) => orm_err!(format!(
            "set operator: {operator:?} in search job partition table error: {e}"
        )),
    }
}

pub async fn clean_deleted_partition_job(job_id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let res = Entity::delete_many()
        .filter(Column::JobId.eq(job_id))
        .exec(client)
        .await;

    if let Err(e) = res {
        return orm_err!(format!("clean_deleted_partition_jobs failed: {}", e));
    }

    Ok(())
}
