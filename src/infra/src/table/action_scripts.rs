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

use std::str::FromStr;

use chrono::{DateTime, Utc};
use config::meta::actions::action::{Action, ActionStatus, ExecutionDetailsType};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Schema, Set, Unchanged,
};

use super::{
    entity::action_scripts::{ActiveModel, Model},
    get_lock,
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors::{self, DbError, Error},
    table::entity::action_scripts::{Column, Entity},
};

impl TryFrom<Model> for Action {
    type Error = errors::Error;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        Ok(Action {
            id: Some(
                svix_ksuid::Ksuid::from_str(&model.id)
                    .map_err(|e| Error::Message(e.to_string()))?,
            ),
            name: model.name,
            org_id: model.org_id,
            environment_variables: serde_json::from_value(model.env)?,
            created_by: model.created_by,
            execution_details: ExecutionDetailsType::try_from(model.execution_details.as_str())
                .unwrap(),
            zip_file_path: Some(model.file_path),
            created_at: DateTime::<Utc>::from_timestamp_micros(model.created_at).unwrap(),
            last_executed_at: model
                .last_executed_at
                .and_then(DateTime::<Utc>::from_timestamp_micros),
            description: model.description,
            cron_expr: model.cron_expr,
            status: ActionStatus::try_from(model.status.as_str()).unwrap(),
            zip_file_name: model.file_name,
            last_modified_at: DateTime::<Utc>::from_timestamp_micros(model.last_modified_at)
                .unwrap(),
            last_successful_at: model
                .last_successful_at
                .and_then(DateTime::<Utc>::from_timestamp_micros),
            origin_cluster_url: (model.origin_cluster_url),
            service_account: (model.service_account),
        })
    }
}
pub async fn create_table() -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let builder = client.get_database_backend();

    let schema = Schema::new(builder);
    let create_table_stmt = schema
        .create_table_from_entity(Entity)
        .if_not_exists()
        .take();

    client.execute(builder.build(&create_table_stmt)).await?;

    Ok(())
}

pub async fn add(action: &Action) -> Result<String, errors::Error> {
    let record = ActiveModel {
        id: Set(action.id.unwrap().to_string()),
        name: Set(action.name.clone()),
        org_id: Set(action.org_id.to_string()),
        file_path: Set(action
            .zip_file_path
            .clone()
            .ok_or(Error::Message("file path not set".to_string()))?),
        env: Set(serde_json::json!(action.environment_variables.clone())),
        execution_details: Set(action.execution_details.to_string()),
        cron_expr: Set(action.cron_expr.clone()),
        created_at: Set(action.created_at.timestamp_micros()),
        last_modified_at: Set(action.last_modified_at.timestamp_micros()),
        last_executed_at: Set(action.last_executed_at.map(|d| d.timestamp_micros())),
        last_successful_at: Set(action.last_successful_at.map(|d| d.timestamp_micros())),
        description: Set(action.description.clone()),
        file_name: Set(action.zip_file_name.clone()),
        created_by: Set(action.created_by.clone()),
        status: Set(action.status.to_string()),
        origin_cluster_url: Set(action.origin_cluster_url.clone()),
        service_account: Set(action.service_account.clone()),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let uuid = Entity::insert(record).exec(client).await?.last_insert_id;

    Ok(uuid)
}

pub async fn update(action: &Action) -> Result<String, errors::Error> {
    let id = action.id.ok_or(Error::Message("id not set".to_string()))?;
    let record = ActiveModel {
        id: Unchanged(id.to_string()),
        name: Set(action.name.clone()),
        org_id: Set(action.org_id.to_string()),
        file_path: Set(action
            .zip_file_path
            .clone()
            .ok_or(Error::Message("file path not set".to_string()))?),
        env: Set(serde_json::json!(action.environment_variables.clone())),
        execution_details: Set(action.execution_details.to_string()),
        cron_expr: Set(action.cron_expr.clone()),
        created_at: Set(action.created_at.timestamp_micros()),
        last_modified_at: Set(action.last_modified_at.timestamp_micros()),
        last_executed_at: Set(action.last_executed_at.map(|d| d.timestamp_micros())),
        last_successful_at: Set(action.last_successful_at.map(|d| d.timestamp_micros())),
        description: Set(action.description.clone()),
        file_name: Set(action.zip_file_name.clone()),
        created_by: Set(action.created_by.clone()),
        status: Set(action.status.to_string()),
        origin_cluster_url: Set(action.origin_cluster_url.clone()),
        service_account: Set(action.service_account.clone()),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let action = Entity::update(record).exec(client).await?;

    Ok(action.id)
}
pub async fn remove(org_id: &str, id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let _ = Entity::delete_many()
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Id.eq(id))
        .exec(client)
        .await?;

    Ok(())
}

pub async fn get(id: &str, org_id: &str) -> Result<Action, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find_by_id(id)
        .filter(Column::OrgId.eq(org_id))
        .one(client)
        .await?
        .ok_or_else(|| {
            Error::DbError(DbError::SeaORMError("Action Script not found".to_string()))
        })?;

    record.try_into()
}

pub async fn list(org_id: &str, limit: Option<i64>) -> Result<Vec<Action>, errors::Error> {
    let limit = limit.unwrap_or(100);
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let res = Entity::find()
        .filter(Column::OrgId.eq(org_id))
        .limit(limit as u64)
        .order_by(Column::Id, Order::Desc);

    let records = res
        .all(client)
        .await?
        .into_iter()
        .map(Action::try_from)
        .collect::<Result<_, errors::Error>>()?;

    Ok(records)
}

pub async fn contains(id: &str, org_id: &str) -> Result<bool, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::Id.eq(id))
        .filter(Column::OrgId.eq(org_id))
        .one(client)
        .await?;

    Ok(record.is_some())
}

pub async fn len() -> Result<usize, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let len = Entity::find().count(client).await?;
    Ok(len as usize)
}

pub async fn clear() -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many().exec(client).await?;

    Ok(())
}

pub async fn is_empty() -> Result<bool, errors::Error> {
    Ok(len().await? == 0)
}
