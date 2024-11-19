use config::meta::folder::Folder;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, Set, TryIntoModel,
};

use super::entity::folders::{ActiveModel, Column, Entity, Model};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors,
};

// Flags that indicate the type of data the folder can contain.
const DASHBOARDS_FOLDER_TYPE: i16 = 0;

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

    // We should probably generate folder_id here for new folders, rather than
    // depending on caller code to generate it.
    let folder_id = parse_folder_id(&folder.folder_id)?;
    let mut active: ActiveModel = match get_model(client, org_id, folder_id).await? {
        // If a folder with the given folder_id already exists, get that folder
        // model and use it as the active model so that Sea ORM will update the
        // corresponding DB record when the active model is saved.
        Some(model) => model.into(),
        // In no folder with the given folder_id already exists, create a new
        // active record so that Sea ORM will create a new DB record when the
        // active model is saved.
        None => ActiveModel {
            org: Set(org_id.to_owned()),
            id: Set(folder_id),
            // Currently we only create dashboard folders. If we want to support
            // creating different type of folders then we need to change the API
            // for folders, either by adding the type field to the folder model
            // or by creating specialized routes for creating folders of
            // different types.
            r#type: Set(DASHBOARDS_FOLDER_TYPE),
            ..Default::default()
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
    folder_type: i16,
) -> Result<Vec<Model>, sea_orm::DbErr> {
    Entity::find()
        .filter(Column::Org.eq(org_id))
        .filter(Column::Type.eq(folder_type))
        .order_by(Column::Id, sea_orm::Order::Asc)
        .all(db)
        .await
}
