//! Populates the dashboards table by transforming unstructured dashboard
//! records from the meta table.

use std::collections::VecDeque;

use config::utils::json;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, FromQueryResult, Paginator,
    PaginatorTrait, QueryFilter, QueryOrder, SelectModel, Set, Statement, TransactionTrait,
};
use sea_orm_migration::prelude::*;
use serde::{self, Deserialize};
use serde_json::Value as JsonValue;

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
        let mut meta_pages = MetaDashboard::paginate(&txn, 100);

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let dashboards_rslt: Result<Vec<_>, DbErr> = metas
                .into_iter()
                .map(|m| {
                    let folder_id = m.folder_id.ok_or_else(|| {
                        DbErr::Migration(
                            "Dashboard in meta table references folder that does not exist"
                                .to_string(),
                        )
                    })?;
                    let mut value: JsonValue = serde_json::from_str(&m.value).map_err(|_| {
                        DbErr::Migration(
                            "Dashboard in meta table has value field that is not valid JSON"
                                .to_string(),
                        )
                    })?;
                    let mut obj = value.as_object_mut().ok_or_else(|| {
                        DbErr::Migration(
                            "Dashboard in meta table has value field that is not a JSON object"
                                .to_string(),
                        )
                    })?;

                    Ok(dashboards::ActiveModel {
                        folder_id: Set(folder_id),
                        owner: todo!(),
                        role: todo!(),
                        title: todo!(),
                        description: todo!(),
                        data: todo!(),
                        version: todo!(),
                        ..Default::default()
                    })
                })
                .collect();
            let dashboards = dashboards_rslt?;
            dashboards::Entity::insert_many(dashboards)
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        dashboards::Entity::delete_many().exec(db).await?;
        Ok(())
    }
}

/// A result from querying for dashboards from the meta table and joining on the
/// folders table.
#[derive(Debug, FromQueryResult)]
pub struct MetaDashboard {
    org: String,
    folder_id: Option<i64>,
    dashboard_id: String,
    value: String,
}

impl MetaDashboard {
    /// Paginate through the results of querying for dashboards from the meta
    /// table and joining on the folders table.
    fn paginate<C>(db: &C, page_size: u64) -> Paginator<'_, C, SelectModel<MetaDashboard>>
    where
        C: ConnectionTrait,
    {
        let backend = db.get_database_backend();
        let sql = match backend {
            sea_orm::DatabaseBackend::MySql => todo!(),
            sea_orm::DatabaseBackend::Postgres => {
                r#"
                SELECT 
                    m.key1 AS org,
                    f.id AS folder_id,
                    split_part(m.key2, '/', 2) AS deashboard_id,
                    m.value AS value
                FROM meta AS m
                JOIN folders f ON
                    split_part(m.key2, '/', 1) = f.folder_id AND
                    m.key1 = f.org
                WHERE m.module = 'dashboard'
                ORDER BY m.id
            "#
            }
            sea_orm::DatabaseBackend::Sqlite => todo!(),
        };

        Self::find_by_statement(Statement::from_sql_and_values(backend, sql, []))
            .paginate(db, page_size)
    }
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

/// Representation of the dashboards table at the time this migration executes.
mod dashboards {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "dashboards")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: i64,
        pub folder_id: i64,
        pub dashboard_id: String,
        pub owner: String,
        pub role: Option<String>,
        pub title: String,
        #[sea_orm(column_type = "Text", nullable)]
        pub description: Option<String>,
        pub data: Json,
        pub version: i32,
        pub created_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::folders::Entity",
            from = "Column::FolderId",
            to = "super::folders::Column::Id",
            on_update = "NoAction",
            on_delete = "NoAction"
        )]
        Folders,
    }

    impl Related<super::folders::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Folders.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
