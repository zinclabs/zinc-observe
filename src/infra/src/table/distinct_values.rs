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
    entity::prelude::*, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    Schema, Set,
};

use super::get_lock;
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors::{self, DbError, Error},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum OriginType {
    #[sea_orm(string_value = "stream")]
    Stream,
    #[sea_orm(string_value = "dashboard")]
    Dashboard,
    #[sea_orm(string_value = "report")]
    Report,
}

/// Define the distinct fields table
/// Primary key for this is composite of all fields, i.e.
/// There will always be only one entry for specific origin-stream-field combination
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "distinct_value_fields")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "String(StringLen::N(16))")]
    pub origin: OriginType,
    #[sea_orm(primary_key, column_type = "String(StringLen::N(32))")]
    pub origin_id: String,
    #[sea_orm(primary_key, column_type = "String(StringLen::N(128))")]
    pub org_name: String,
    #[sea_orm(primary_key, column_type = "String(StringLen::N(256))")]
    pub stream_name: String,
    #[sea_orm(primary_key, column_type = "String(StringLen::N(16))")]
    pub stream_type: String,
    #[sea_orm(primary_key, column_type = "String(StringLen::N(256))")]
    pub field_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("No relations defined")
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(FromQueryResult, Debug)]
pub struct DistinctFieldRecord {
    pub origin: OriginType,
    pub origin_id: String,
    pub org_name: String,
    pub stream_name: String,
    pub stream_type: String,
    pub field_name: String,
}

impl DistinctFieldRecord {
    pub fn new(
        origin: OriginType,
        origin_id: &str,
        org: &str,
        stream: &str,
        stream_type: String,
        field: &str,
    ) -> Self {
        Self {
            origin,
            origin_id: origin_id.to_owned(),
            org_name: org.to_owned(),
            stream_name: stream.to_owned(),
            stream_type,
            field_name: field.to_owned(),
        }
    }
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

    client
        .execute(builder.build(&create_table_stmt))
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn add(record: DistinctFieldRecord) -> Result<(), errors::Error> {
    let record = ActiveModel {
        origin: Set(record.origin),
        origin_id: Set(record.origin_id),
        org_name: Set(record.org_name),
        stream_name: Set(record.stream_name),
        stream_type: Set(record.stream_type),
        field_name: Set(record.field_name),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let res = Entity::insert(record).exec(client).await;

    match res {
        Ok(_) => Ok(()),
        Err(DbErr::Exec(RuntimeErr::SqlxError(SqlxError::Database(e)))) => {
            // unique violation will occur when we try to re-insert the same combination
            // which is ok, because what we want is already there.
            if e.is_unique_violation() {
                Ok(())
            } else {
                Err(Error::DbError(DbError::SeaORMError(e.to_string())))
            }
        }
        Err(e) => Err(Error::DbError(DbError::SeaORMError(e.to_string()))),
    }
}

pub async fn remove(record: DistinctFieldRecord) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let record = ActiveModel {
        origin: Set(record.origin),
        origin_id: Set(record.origin_id),
        org_name: Set(record.org_name),
        stream_name: Set(record.stream_name),
        stream_type: Set(record.stream_type),
        field_name: Set(record.field_name),
    };

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete(record)
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn check_field_use(
    org_name: &str,
    stream_name: &str,
    stream_type: &str,
    field_name: &str,
) -> Result<Vec<DistinctFieldRecord>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let records = Entity::find()
        .filter(Column::OrgName.eq(org_name))
        .filter(Column::StreamName.eq(stream_name))
        .filter(Column::StreamType.eq(stream_type))
        .filter(Column::FieldName.eq(field_name))
        .into_model::<DistinctFieldRecord>()
        .all(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;
    Ok(records)
}

/// This is specifically for the case when a dashboard is deleted, we can bulk remove
/// the dependencies, without having to go through one by one
pub async fn batch_remove(origin: OriginType, origin_id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::Origin.eq(origin))
        .filter(Column::OriginId.eq(origin_id))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn len() -> Result<u64, errors::Error> {
    let _lock = get_lock().await;
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::find()
        .count(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))
}
