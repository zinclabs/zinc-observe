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

use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Set};

use super::{entity::cipher_keys::*, get_lock};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

pub enum EntryKind {
    CipherKey,
}

impl ToString for EntryKind {
    fn to_string(&self) -> String {
        match self {
            Self::CipherKey => "cipher_key".to_owned(),
        }
    }
}

impl TryFrom<String> for EntryKind {
    type Error = errors::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "cipher_key" => Ok(Self::CipherKey),
            _ => Err(errors::Error::NotImplemented),
        }
    }
}

pub struct ListFilter {
    pub org: Option<String>,
    pub kind: Option<EntryKind>,
}

pub struct CipherEntry {
    pub org: String,
    pub created_at: i64,
    pub created_by: String,
    pub name: String,
    pub data: String,
    pub kind: EntryKind,
}

impl TryInto<CipherEntry> for Model {
    type Error = errors::Error;
    fn try_into(self) -> Result<CipherEntry, Self::Error> {
        Ok(CipherEntry {
            org: self.org,
            created_at: self.created_at,
            created_by: self.created_by,
            kind: self.kind.try_into().unwrap(), // we can be fairly certain that this will not fail
            name: self.name,
            data: self.data,
        })
    }
}

pub async fn add(entry: CipherEntry) -> Result<(), errors::Error> {
    let record = ActiveModel {
        org: Set(entry.org),
        created_by: Set(entry.created_by),
        created_at: Set(entry.created_at),
        name: Set(entry.name),
        kind: Set(entry.kind.to_string()),
        data: Set(entry.data), // TODO encrypt data
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::insert(record).exec(client).await?;

    Ok(())
}

pub async fn remove(org: &str, kind: EntryKind, name: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::Org.eq(org))
        .filter(Column::Kind.eq(&kind.to_string()))
        .filter(Column::Name.eq(name))
        .exec(client)
        .await?;

    Ok(())
}

pub async fn get_data(
    org: &str,
    kind: EntryKind,
    name: &str,
) -> Result<Option<String>, errors::Error> {
    let res = get(org, kind, name).await?;
    // todo decrypt the data
    Ok(res.map(|r| r.data))
}

async fn get(org: &str, kind: EntryKind, name: &str) -> Result<Option<Model>, errors::DbError> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::find()
        .filter(Column::Org.eq(org))
        .filter(Column::Name.eq(name))
        .filter(Column::Kind.eq(&kind.to_string()))
        .into_model::<Model>()
        .one(client)
        .await
        .map_err(|e| errors::DbError::SeaORMError(e.to_string()))
}

pub async fn list_all(limit: Option<i64>) -> Result<Vec<CipherEntry>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find().order_by(Column::CreatedAt, Order::Desc);
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<Model>().all(client).await?;

    records.into_iter().map(|r| r.try_into()).collect()
}

pub async fn list_filtered(
    filter: ListFilter,
    limit: Option<i64>,
) -> Result<Vec<CipherEntry>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find().order_by(Column::CreatedAt, Order::Desc);
    if let Some(ref org) = filter.org {
        res = res.filter(Column::Org.eq(org));
    }
    if let Some(ref kind) = filter.kind {
        res = res.filter(Column::Kind.eq(&kind.to_string()));
    }
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res.into_model::<Model>().all(client).await?;

    records.into_iter().map(|r| r.try_into()).collect()
}

pub async fn clear() -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many().exec(client).await?;

    Ok(())
}
