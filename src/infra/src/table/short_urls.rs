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
    entity::prelude::*, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait,
    FromQueryResult, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Schema, Set,
};

use super::get_lock;
use crate::{
    db::{connect_to_orm, mysql, postgres, sqlite, IndexStatement, ORM_CLIENT},
    errors::{self, DbError, Error},
};

// define the short_urls table
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "short_urls")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm(column_type = "String(StringLen::N(32))")]
    pub short_id: String,
    #[sea_orm(column_type = "Text")]
    pub original_url: String,
    pub created_ts: i64,
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
pub struct ShortUrlRecord {
    pub short_id: String,
    pub original_url: String,
}

impl ShortUrlRecord {
    pub fn new(short_id: &str, original_url: &str) -> Self {
        Self {
            short_id: short_id.to_string(),
            original_url: original_url.to_string(),
        }
    }
}

#[derive(FromQueryResult, Debug)]
pub struct ShortId {
    pub short_id: String,
}

pub async fn init() -> Result<(), errors::Error> {
    create_table().await?;
    create_table_index().await?;
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

pub async fn create_table_index() -> Result<(), errors::Error> {
    let index1 = IndexStatement::new("short_urls_short_id_idx", "short_urls", true, &["short_id"]);
    let index2 = IndexStatement::new(
        "short_urls_created_ts_idx",
        "short_urls",
        false,
        &["created_ts"],
    );

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match client.get_database_backend() {
        DatabaseBackend::MySql => {
            mysql::create_index(index1).await?;
            mysql::create_index(index2).await?;
        }
        DatabaseBackend::Postgres => {
            postgres::create_index(index1).await?;
            postgres::create_index(index2).await?;
        }
        _ => {
            sqlite::create_index(index1).await?;
            sqlite::create_index(index2).await?;
        }
    }
    Ok(())
}

pub async fn add(short_id: &str, original_url: &str) -> Result<(), errors::Error> {
    let record = ActiveModel {
        short_id: Set(short_id.to_string()),
        original_url: Set(original_url.to_string()),
        created_ts: Set(chrono::Utc::now().timestamp_micros()),
        ..Default::default()
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::insert(record).exec(client).await?;

    Ok(())
}

pub async fn remove(short_id: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::ShortId.eq(short_id))
        .exec(client)
        .await?;

    Ok(())
}

pub async fn get(short_id: &str) -> Result<ShortUrlRecord, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .select_only()
        .column(Column::ShortId)
        .column(Column::OriginalUrl)
        .filter(Column::ShortId.eq(short_id))
        .into_model::<ShortUrlRecord>()
        .one(client)
        .await?
        .ok_or_else(|| Error::DbError(DbError::SeaORMError("Short URL not found".to_string())))?;

    Ok(record)
}

pub async fn list(limit: Option<i64>) -> Result<Vec<ShortUrlRecord>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find()
        .select_only()
        .column(Column::ShortId)
        .column(Column::OriginalUrl)
        .order_by(Column::CreatedTs, Order::Desc);
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<ShortUrlRecord>().all(client).await?;

    Ok(records)
}

pub async fn contains(short_id: &str) -> Result<bool, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::ShortId.eq(short_id))
        .into_model::<ShortUrlRecord>()
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

pub async fn get_expired(
    expired_before: i64,
    limit: Option<i64>,
) -> Result<Vec<String>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find()
        .select_only()
        .column(Column::ShortId)
        .filter(Column::CreatedTs.lt(expired_before));
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<ShortId>().all(client).await?;
    Ok(records.iter().map(|r| r.short_id.clone()).collect())
}

pub async fn batch_remove(short_ids: Vec<String>) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::ShortId.is_in(short_ids))
        .exec(client)
        .await?;

    Ok(())
}
