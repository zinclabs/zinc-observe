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

use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use super::super::{entity::search_job_results::*, get_lock};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors, orm_err,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResultOperator {
    Delete { job_id: String },
}

pub async fn get(job_id: &str) -> Result<Vec<Model>, errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let res = Entity::find()
        .filter(Column::JobId.eq(job_id))
        .order_by(Column::StartedAt, Order::Asc)
        .all(client)
        .await;

    match res {
        Ok(res) => Ok(res),
        Err(e) => orm_err!(format!("get_search_job_results failed: {}", e)),
    }
}

pub async fn clean_deleted_job_result(job_id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let res = Entity::delete_many()
        .filter(Column::JobId.eq(job_id))
        .exec(client)
        .await;

    if let Err(e) = res {
        return orm_err!(format!("delete_search_job_results failed: {}", e));
    }

    Ok(())
}
