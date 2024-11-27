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

//! Populates the dashboards table by transforming unstructured dashboard
//! records from the meta table.

use chrono::{DateTime, FixedOffset};
use sea_orm::{
    ActiveValue::NotSet, EntityTrait, FromQueryResult, Paginator, PaginatorTrait, SelectModel, Set,
    Statement, TransactionTrait,
};
use sea_orm_migration::prelude::*;
use serde_json::Value as JsonValue;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;

        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        let mut meta_pages = MetaDashboard::paginate(&txn, 100);

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let dashboards_rslt: Result<Vec<_>, DbErr> = metas
                .into_iter()
                .map(|m| {
                    let m: dashboards::ActiveModel = m.try_into().map_err(DbErr::Migration)?;
                    Ok(m)
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
            sea_orm::DatabaseBackend::MySql => {
                r#"
                    SELECT 
                        f.id AS folder_id,
                        SUBSTRING_INDEX(m.key2,'/', -1) AS dashboard_id,
                        m.value AS value
                    FROM meta AS m
                    JOIN folders f ON
                        SUBSTRING_INDEX(m.key2,'/', 1) = f.folder_id AND
                        m.key1 = f.org
                    WHERE m.module = 'dashboard'
                    ORDER BY m.id ASC
                "#
            }
            sea_orm::DatabaseBackend::Postgres => {
                r#"
                    SELECT 
                        f.id AS folder_id,
                        SPLIT_PART(m.key2, '/', 2) AS dashboard_id,
                        m.value AS value
                    FROM meta AS m
                    JOIN folders f ON
                        SPLIT_PART(m.key2, '/', 1) = f.folder_id AND
                        m.key1 = f.org
                    WHERE m.module = 'dashboard'
                    ORDER BY m.id ASC
                "#
            }
            sea_orm::DatabaseBackend::Sqlite => {
                r#"
                    SELECT 
                        f.id AS folder_id,
                        SUBSTR(m.key2, INSTR(m.key2, '/') + 1) AS dashboard_id,
                        m.value AS value
                    FROM meta AS m
                    INNER JOIN folders f ON
                        SUBSTR(m.key2, 1, INSTR(m.key2, '/') - 1) = f.folder_id AND
                        m.key1 = f.org
                    WHERE m.module = 'dashboard'
                    ORDER BY m.id ASC
                "#
            }
        };

        Self::find_by_statement(Statement::from_sql_and_values(backend, sql, []))
            .paginate(db, page_size)
    }
}

impl TryFrom<MetaDashboard> for dashboards::ActiveModel {
    type Error = String;

    fn try_from(m: MetaDashboard) -> Result<Self, Self::Error> {
        let folder_id = m
            .folder_id
            .ok_or("Dashboard in meta table references folder that does not exist")?;
        let dashboard_id = if m.dashboard_id.is_empty() {
            Err("Dashboard in meta table is missing a dasbhoard ID")
        } else {
            Ok(m.dashboard_id)
        }?;

        let mut value: JsonValue = serde_json::from_str(&m.value)
            .map_err(|_| "Dashboard in meta table has \"value\" field that is not valid JSON")?;
        let obj = value
            .as_object_mut()
            .ok_or("Dashboard in meta table has \"value\" field that is not a JSON object")?;

        // Remove each of the following fields from the inner JSON since they
        // will now be stored in table columns and we don't want to keep
        // duplicate sources of the data that we would need to synchronize.
        obj.remove("dashboardId");
        let version: i32 = obj
            .remove("version")
            .and_then(|v| v.as_i64())
            .and_then(|v: i64| v.try_into().ok())
            .ok_or("Dashboard JSON does not have i32 \"version\" field")?;
        let owner = obj
            .remove("owner")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or("Dashboard JSON does not have string \"owner\" field")?;
        let title = obj
            .remove("title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or("Dashboard JSON does not have string \"title\" field")?;
        let role = obj
            .remove("role")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let description = obj
            .remove("description")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|s| if s.is_empty() { None } else { Some(s) });

        // Since the created field includes a timezone that cannot be
        // represented in all databases and since the UI expects to receieve a
        // timestamp with a timezone, we will leave the created field in the
        // JSON while also migrating its value into the created_at Unix timestamp column.
        let created_at_tz: DateTime<FixedOffset> = obj
            .get("created")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or("Dashboard JSON does not have string \"created\" field".to_string())
            .and_then(|s| DateTime::parse_from_rfc3339(&s).map_err(|e| e.to_string()))?;
        let created_at_unix = created_at_tz.timestamp();

        Ok(dashboards::ActiveModel {
            id: NotSet, // Set automatically by DB.
            dashboard_id: Set(dashboard_id),
            folder_id: Set(folder_id),
            owner: Set(owner),
            role: Set(role),
            title: Set(title),
            description: Set(description),
            data: Set(value),
            version: Set(version),
            created_at: Set(created_at_unix),
        })
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
        #[sea_orm(primary_key)]
        pub id: i64,
        pub dashboard_id: String,
        pub folder_id: i64,
        pub owner: String,
        pub role: Option<String>,
        pub title: String,
        #[sea_orm(column_type = "Text", nullable)]
        pub description: Option<String>,
        pub data: Json,
        pub version: i32,
        pub created_at: i64,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
