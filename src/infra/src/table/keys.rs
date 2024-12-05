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
    entity::prelude::*, ColumnTrait, ConnectionTrait, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Schema, Set,
};

use super::get_lock;
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(15))")]
pub enum KeyType {
    #[sea_orm(string_value = "akeyless")]
    Akeyless,
    #[sea_orm(string_value = "local")]
    Local,
}

// define the short_urls table
// The primary key is combination of org-name ; so each org can only have
// one key with given name
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "key_info")]
pub struct Model {
    pub created_ts: i64,
    #[sea_orm(primary_key, column_type = "Text")]
    pub org: String,
    #[sea_orm(column_type = "Text")]
    pub created_by: String,
    #[sea_orm(column_type = "Text")]
    pub key_type: KeyType,
    #[sea_orm(primary_key, column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub credentials: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub struct KeyInfo {
    pub created_ts: i64,
    pub created_by: String,
    pub key_type: KeyType,
    pub name: String,
    pub credentials: String,
}

pub async fn init() -> Result<(), errors::Error> {
    create_table().await?;
    Ok(())
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

pub async fn add(org: &str, key: KeyInfo) -> Result<(), errors::Error> {
    let record = ActiveModel {
        org: Set(org.to_string()),
        created_by: Set(key.created_by),
        key_type: Set(key.key_type),
        name: Set(key.name),
        credentials: Set(key.credentials),
        created_ts: Set(chrono::Utc::now().timestamp_micros()),
        ..Default::default()
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::insert(record).exec(client).await?;

    Ok(())
}

pub async fn remove(org: &str, name: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::Org.eq(org))
        .filter(Column::Name.eq(name))
        .exec(client)
        .await?;

    Ok(())
}

pub async fn get(org: &str, name: &str) -> Result<Option<Model>, errors::DbError> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match Entity::find()
        .filter(Column::Org.eq(org))
        .filter(Column::Name.eq(name))
        .into_model::<Model>()
        .one(client)
        .await
    {
        Ok(v) => Ok(v),
        Err(e) => Err(errors::DbError::SeaORMError(e.to_string())),
    }
}

pub async fn list(limit: Option<i64>) -> Result<Vec<Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find().order_by(Column::CreatedTs, Order::Desc);
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<Model>().all(client).await?;

    Ok(records)
}

pub async fn contains(org: &str, name: &str) -> Result<bool, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::Org.eq(org))
        .filter(Column::Name.eq(name))
        .into_model::<Model>()
        .one(client)
        .await?;

    Ok(record.is_some())
}

pub async fn len() -> usize {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let len = Entity::find().count(client).await;

    match len {
        Ok(len) => len as usize,
        Err(e) => {
            log::error!("short_urls len error: {}", e);
            0
        }
    }
}

pub async fn clear() -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many().exec(client).await?;

    Ok(())
}

pub async fn is_empty() -> bool {
    len().await == 0
}
