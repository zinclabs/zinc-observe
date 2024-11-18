use config::meta::folder::Folder;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, EntityTrait,
    ModelTrait, QueryFilter, QueryOrder, Set,
};

use super::entity::folder::{ActiveModel, Column, Entity, Model};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

// Flags that indicate the type of data the folder can contain.
const DASHBOARDS_FOLDER_TYPE: i16 = 0;

impl From<Model> for Folder {
    fn from(value: Model) -> Self {
        Self {
            folder_id: value.folder_id,
            name: value.name,
            description: value.description.unwrap_or_default(),
        }
    }
}

/// Gets a folder by its `folder_id` surrogate key.
pub async fn get(org_id: &str, folder_id: &str) -> Result<Option<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folder = get_model(client, org_id, folder_id)
        .await
        .map(|f| f.map(Folder::from))?;
    Ok(folder)
}

/// Lists all dashboard folders.
pub async fn list_dashboard_folders(org_id: &str) -> Result<Vec<Folder>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let folders = list_models(client, org_id, DASHBOARDS_FOLDER_TYPE)
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
    let folder_id = folder.folder_id;
    let mut active: ActiveModel = match get_model(client, org_id, &folder_id).await? {
        Some(model) => model.into(),
        None => ActiveModel {
            id: NotSet,
            // We should probably set folder_id here rather than in caller code.
            ..Default::default()
        },
    };

    active.name = Set(folder.name);
    active.description = Set(if folder.description.is_empty() {
        None
    } else {
        Some(folder.description)
    });
    let model = active.update(client).await?;
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

/// Gets a folder ORM entity by its `folder_id` surrogate key.
async fn get_model(
    db: &DatabaseConnection,
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
async fn list_models(
    db: &DatabaseConnection,
    org_id: &str,
    folder_type: i16,
) -> Result<Vec<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::Type.eq(folder_type))
        .order_by(Column::Id, sea_orm::Order::Asc)
        .all(db)
        .await
}
