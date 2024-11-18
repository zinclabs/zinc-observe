//! Populates the folders table by transforming unstructured folder records from
//! the meta table.

use config::utils::json;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::prelude::*;
use serde::{self, Deserialize};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;

        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        // txn.execute()
        let mut meta_pages = meta::Entity::find()
            .filter(meta::Column::Module.eq("folders"))
            .order_by_asc(meta::Column::Id)
            .paginate(&txn, 100);

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let folders_rslt: Result<Vec<_>, DbErr> = metas
                .into_iter()
                .map(|m| {
                    // Transform unstructured JSON from the meta record.
                    let json: MetaFolder =
                        json::from_str(&m.value).map_err(|e| DbErr::Migration(e.to_string()))?;
                    let description = if json.description.is_empty() {
                        None
                    } else {
                        Some(json.description)
                    };
                    Ok(folder::ActiveModel {
                        folder_id: Set(m.key2),
                        org: Set(m.key1),
                        name: Set(json.name),
                        description: Set(description),
                        r#type: Set(0),
                        ..Default::default()
                    })
                })
                .collect();
            let folders = folders_rslt?;
            folder::Entity::insert_many(folders).exec(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        folder::Entity::delete_many().exec(db).await?;
        Ok(())
    }
}

/// Representation of a folder in the meta table.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaFolder {
    #[serde(default)]
    pub folder_id: String,
    pub name: String,
    pub description: String,
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

/// Representation of the meta table at the time this migration executes.
mod meta {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "meta")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub module: String,
        pub key1: String,
        pub key2: String,
        pub start_dt: i64,
        #[sea_orm(column_type = "Text")]
        pub value: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the folder table at the time this migration executes.
mod folder {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "folder")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(unique)]
        pub folder_id: String,
        pub org: String,
        pub name: String,
        #[sea_orm(column_type = "Text", nullable)]
        pub description: Option<String>,
        pub r#type: i16,
        pub created_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
