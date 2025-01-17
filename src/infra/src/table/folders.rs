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

use config::meta::folder::{Folder, FolderType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, ModelTrait, QueryFilter, QueryOrder, Set, TryIntoModel,
};
use svix_ksuid::{Ksuid, KsuidLike};

use super::entity::folders::{ActiveModel, Column, Entity, Model};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors::{self, FromStrError},
};

impl From<Model> for Folder {
    fn from(value: Model) -> Self {
        Self {
            folder_id: value.folder_id.to_string(),
            name: value.name,
            description: value.description.unwrap_or_default(),
        }
    }
}

/// Converts the [FolderType] into its [i16] representation in the database.
pub(crate) fn folder_type_into_i16(folder_type: FolderType) -> i16 {
    match folder_type {
        FolderType::Dashboards => 0,
        FolderType::Alerts => 1,
    }
}

/// Gets a folder by its ID.
pub async fn get(
    org_id: &str,
    folder_id: &str,
    folder_type: FolderType,
) -> Result<Option<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folder = get_model(client, org_id, folder_id, folder_type)
        .await
        .map(|f| f.map(Folder::from))?;
    Ok(folder)
}

/// Gets a folder by its ID.
pub async fn get_by_name(
    org_id: &str,
    folder_name: &str,
    folder_type: FolderType,
) -> Result<Option<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folder = get_model_by_name(client, org_id, folder_name, folder_type)
        .await
        .map(|f| f.map(Folder::from))?;
    Ok(folder)
}

/// Checks if the folder with the given ID exists.
pub async fn exists(
    org_id: &str,
    folder_id: &str,
    folder_type: FolderType,
) -> Result<bool, errors::Error> {
    let exists = get(org_id, folder_id, folder_type).await?.is_some();
    Ok(exists)
}

/// Lists all dashboard folders of the specified type.
pub async fn list_folders(
    org_id: &str,
    folder_type: FolderType,
) -> Result<Vec<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folders = list_models(client, org_id, folder_type)
        .await?
        .into_iter()
        .map(Folder::from)
        .collect();
    Ok(folders)
}

/// Creates a new folder or updates an existing folder in the database. Returns
/// the new or updated folder.
pub async fn put(
    org_id: &str,
    id: Option<Ksuid>,
    folder: Folder,
    folder_type: FolderType,
) -> Result<(Ksuid, Folder), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let model = match get_model(client, org_id, &folder.folder_id, folder_type).await? {
        // If a folder with the given folder_id already exists, get that folder
        // model and use it as the active model so that Sea ORM will update the
        // corresponding DB record when the active model is saved.
        Some(model) => {
            let mut active = model.into_active_model();
            active.name = Set(folder.name);
            active.description = Set(Some(folder.description).filter(|d| !d.is_empty()));
            let model: Model = active.update(client).await?.try_into_model()?;
            model
        }
        // In no folder with the given folder_id already exists, create a new
        // active record so that Sea ORM will create a new DB record when the
        // active model is saved.
        None => {
            let ksuid = id.unwrap_or_else(|| svix_ksuid::Ksuid::new(None, None));
            let active = ActiveModel {
                id: Set(ksuid.to_string()),
                org: Set(org_id.to_owned()),
                // We should probably generate folder_id here for new folders,
                // rather than depending on caller code to generate it.
                folder_id: Set(folder.folder_id),
                r#type: Set::<i16>(folder_type_into_i16(folder_type)),
                name: Set(folder.name),
                description: Set(Some(folder.description).filter(|d| !d.is_empty())),
            };
            let model: Model = active.insert(client).await?.try_into_model()?;
            model
        }
    };

    let ksuid = Ksuid::from_base62(&model.id).map_err(|_| FromStrError {
        value: model.id.clone(),
        ty: "svix_ksuid::Ksuid".to_owned(),
    })?;
    Ok((ksuid, model.into()))
}

/// Deletes a folder with the given `folder_id` surrogate key.
pub async fn delete(
    org_id: &str,
    folder_id: &str,
    folder_type: FolderType,
) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let model = get_model(client, org_id, folder_id, folder_type).await?;

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
    folder_type: FolderType,
) -> Result<Option<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::FolderId.eq(folder_id))
        .filter(Column::Type.eq(folder_type_into_i16(folder_type)))
        .one(db)
        .await
}

/// Gets a folder ORM entity by its `folder_id`.
pub(crate) async fn get_model_by_name<C: ConnectionTrait>(
    db: &C,
    org_id: &str,
    folder_name: &str,
    folder_type: FolderType,
) -> Result<Option<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::Name.eq(folder_name))
        .filter(Column::Type.eq(folder_type_into_i16(folder_type)))
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
        .filter(Column::Type.eq(folder_type_into_i16(folder_type)))
        .order_by(Column::Id, sea_orm::Order::Asc)
        .all(db)
        .await
}
