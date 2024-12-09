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
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, ModelTrait, QueryFilter, QueryOrder, Set, TryIntoModel,
};

use super::entity::folders::{ActiveModel, Column, Entity, Model};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

/// Indicates the type of data that the folder can contain.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FolderType {
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
            folder_id: value.folder_id.to_string(),
            name: value.name,
            description: value.description.unwrap_or_default(),
        }
    }
}

/// Gets a folder by its ID.
pub async fn get(org_id: &str, folder_id: &str) -> Result<Option<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folder = get_model(client, org_id, folder_id)
        .await
        .map(|f| f.map(Folder::from))?;
    Ok(folder)
}

/// Checks if the folder with the given ID exists.
pub async fn exists(org_id: &str, folder_id: &str) -> Result<bool, errors::Error> {
    let exists = get(org_id, folder_id).await?.is_some();
    Ok(exists)
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

    let mut active: ActiveModel = match get_model(client, org_id, &folder.folder_id).await? {
        // If a folder with the given folder_id already exists, get that folder
        // model and use it as the active model so that Sea ORM will update the
        // corresponding DB record when the active model is saved.
        Some(model) => model.into(),
        // In no folder with the given folder_id already exists, create a new
        // active record so that Sea ORM will create a new DB record when the
        // active model is saved.
        None => ActiveModel {
            id: NotSet,          // Set by DB.
            name: NotSet,        // Can be updated so this is set below.
            description: NotSet, // Can be updated so this is set below.
            org: Set(org_id.to_owned()),
            // We should probably generate folder_id here for new folders,
            // rather than depending on caller code to generate it.
            folder_id: Set(folder.folder_id),
            // Currently we only create dashboard folders. If we want to support
            // creating different types of folders then we can allow the caller
            // to pass the folder type as a field on the Folder struct.
            r#type: Set::<i16>(FolderType::Dashboards.into()),
        },
    };

    active.name = Set(folder.name);
    active.description = Set(if folder.description.is_empty() {
        None
    } else {
        Some(folder.description)
    });
    let model: Model = active.save(client).await?.try_into_model()?;
    Ok(model.into())
}

/// Deletes a folder with the given `folder_id` surrogate key.
pub async fn delete(org_id: &str, folder_id: &str) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let model = get_model(client, org_id, folder_id).await?;

    if let Some(model) = model {
        let _ = model.delete(client).await?;
    }

    Ok(())
}

/// Gets a folder ORM entity by its `folder_id`.
pub(crate) async fn get_model<C: ConnectionTrait>(
    db: &C,
    org_id: &str,
    folder_id: &str,
) -> Result<Option<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::FolderId.eq(folder_id))
        .one(db)
        .await
}

/// Lists all folder ORM models with the specified type.
async fn list_models<C: ConnectionTrait>(
    db: &C,
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
