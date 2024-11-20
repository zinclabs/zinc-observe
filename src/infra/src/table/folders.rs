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

use config::meta::folder::Folder;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, Set,
};

use super::entity::folders::{ActiveModel, Column, Entity, Model};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

/// Indicates the type of data that the folder can contain.
#[derive(Debug, Clone, Copy)]
enum FolderType {
    Dashboards,
}

impl From<FolderType> for i16 {
    fn from(value: FolderType) -> Self {
        match value {
            FolderType::Dashboards => 0,
        }
    }
}

impl From<Model> for Folder {
    fn from(value: Model) -> Self {
        Self {
            folder_id: value.id.to_string(),
            name: value.name,
            description: value.description.unwrap_or_default(),
        }
    }
}

/// Gets a folder by its ID.
pub async fn get(org_id: &str, folder_id: &str) -> Result<Option<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folder_id = parse_folder_id(folder_id)?;
    let folder = get_model(client, org_id, folder_id)
        .await
        .map(|f| f.map(Folder::from))?;
    Ok(folder)
}

/// Lists all dashboard folders.
pub async fn list_dashboard_folders(org_id: &str) -> Result<Vec<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folders = list_models(client, org_id, FolderType::Dashboards)
        .await?
        .into_iter()
        .map(Folder::from)
        .collect();
    Ok(folders)
}

/// Creates a new folder or updates an existing folder in the database. Returns
/// the new or updated folder.
pub async fn put(org_id: &str, folder: Folder) -> Result<Folder, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // We should probably generate folder_id here for new folders, rather than
    // depending on caller code to generate it.
    let folder_id = parse_folder_id(&folder.folder_id)?;
    let name = folder.name;
    let description = if folder.description.is_empty() {
        None
    } else {
        Some(folder.description)
    };

    match get_model(client, org_id, folder_id).await? {
        // If a folder with the given folder_id already exists then update it.
        Some(model) => {
            let mut active: ActiveModel = model.into();
            active.name = Set(name);
            active.description = Set(description);
            let model = active.update(client).await?;
            Ok(model.into())
        }
        // In no folder with the given folder_id already exists, create a new
        // folder.
        None => {
            let active = ActiveModel {
                id: Set(folder_id),
                org: Set(org_id.to_owned()),
                // Currently we only create dashboard folders. If we want to support
                // creating different type of folders then we need to change the API
                // for folders, either by adding the type field to the folder model
                // or by creating specialized routes for creating folders of
                // different types.
                r#type: Set(FolderType::Dashboards.into()),
                name: Set(name),
                description: Set(description),
                ..Default::default()
            };
            let model = active.insert(client).await?;
            Ok(model.into())
        }
    }
}

/// Deletes a folder with the given `folder_id` surrogate key.
pub async fn delete(org_id: &str, folder_id: &str) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folder_id = parse_folder_id(folder_id)?;
    let model = get_model(client, org_id, folder_id).await?;

    if let Some(model) = model {
        let _ = model.delete(client).await?;
    }

    Ok(())
}

/// Parses the "snowflake" folder ID as an [i64].
fn parse_folder_id(folder_id: &str) -> Result<i64, errors::Error> {
    folder_id
        .parse::<i64>()
        .map_err(|_| errors::Error::Message("folder_id is not a 64-bit signed integer".to_owned()))
}

/// Gets a folder ORM entity by its `folder_id` surrogate key.
async fn get_model(
    db: &DatabaseConnection,
    org_id: &str,
    folder_id: i64,
) -> Result<Option<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::Id.eq(folder_id))
        .one(db)
        .await
}

/// Lists all folder ORM models with the specified type.
async fn list_models(
    db: &DatabaseConnection,
    org_id: &str,
    folder_type: FolderType,
) -> Result<Vec<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::Type.eq::<i16>(folder_type.into()))
        .order_by(Column::Id, sea_orm::Order::Asc)
        .all(db)
        .await
}
