//! Populates the dashboards table by transforming unstructured dashboard
//! records from the meta table.

use std::collections::VecDeque;

use config::utils::json;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use sea_orm_migration::prelude::*;
use serde::{self, Deserialize};

use crate::table::entity::folders;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;

        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        let mut meta_pages = meta::Entity::find()
            .filter(meta::Column::Module.eq("dashboard"))
            .order_by_asc(meta::Column::Id)
            .paginate(&txn, 100);

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let dashboards_rslt: Result<Vec<_>, DbErr> = metas
                .into_iter()
                .map(|m| {
                    let org_id = &m.key1;
                    let (folder_id, dashboard_id) = parse_key2(&m.key2)?;
                    let folder = get_dashboard_folder(&txn, org_id, &folder_id).await?;

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

/// Parses key2 from the meta table into a folder ID and dashboard ID.
fn parse_key2(key2: &str) -> Result<(String, String), DbErr> {
    let mut strs = key2.split("/").collect::<VecDeque<_>>();
    let folder_id = strs
        .pop_front()
        .ok_or_else(|| DbErr::Migration("Dashbord missing folder_id in key2".to_string()))?;
    let dashboard_id = strs
        .pop_front()
        .ok_or_else(|| DbErr::Migration("Dashbord missing dashboard_id in key2".to_string()))?;
    Ok((folder_id.to_owned(), dashboard_id.to_owned()))
}

/// Gets a dashboard folder by its `org` and `folder_id`.
async fn get_dashboard_folder<C>(
    db: &C,
    org_id: &str,
    folder_id: &str,
) -> Result<Option<folders::Model>, DbErr>
where
    C: ConnectionTrait,
{
    folders::Entity::find()
        .filter(folders::Column::Org.eq(org_id))
        .filter(folders::Column::FolderId.eq(folder_id))
        .filter(folders::Column::Type.eq(0)) // 0 indicates the dashboard folder type.
        .one(db)
        .await
}

/// Representation of a dashboard in the meta table at the time this migration
/// runs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaDashboard {
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
    #[sea_orm(table_name = "folders")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
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
