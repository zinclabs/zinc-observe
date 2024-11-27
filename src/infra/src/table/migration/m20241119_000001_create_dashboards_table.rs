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

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const DASHBOARDS_FOLDERS_FK: &str = "dashboards_folders_fk";
const DASHBOARDS_FOLDER_ID_DASHBOARD_ID_IDX: &str = "dashboards_folder_id_dashboard_id_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_dashboards_table_statement())
            .await?;
        manager
            .create_index(create_dashboards_folder_id_dashboard_id_idx_stmnt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(DASHBOARDS_FOLDER_ID_DASHBOARD_ID_IDX)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Dashboards::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the dashboards table.
fn create_dashboards_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Dashboards::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Dashboards::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        // A user-facing ID for the folder. This value can be a 64-bit signed
        // integer "snowflake".
        .col(ColumnDef::new(Dashboards::DashboardId).string_len(256).not_null())
        // Foreign key to the folders table.
        .col(ColumnDef::new(Dashboards::FolderId).big_integer().not_null())
        // Identifier of the user that owns the dashboard.
        .col(ColumnDef::new(Dashboards::Owner).string_len(256).not_null())
        .col(ColumnDef::new(Dashboards::Role).string_len(256).null())
        .col(ColumnDef::new(Dashboards::Title).string_len(256).not_null())
        .col(ColumnDef::new(Dashboards::Description).text().null())
        .col(ColumnDef::new(Dashboards::Data).json().not_null())
        .col(ColumnDef::new(Dashboards::Version).integer().not_null())
        .col(
            ColumnDef::new(Dashboards::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(DASHBOARDS_FOLDERS_FK)
                    .from(Dashboards::Table, Dashboards::FolderId)
                    .to(Folders::Table, Folders::Id)
        )
        .to_owned()
}

/// Statement to create unique index on dashboard_id.
fn create_dashboards_folder_id_dashboard_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(DASHBOARDS_FOLDER_ID_DASHBOARD_ID_IDX)
        .table(Dashboards::Table)
        .col(Dashboards::FolderId)
        .col(Dashboards::DashboardId)
        .unique()
        .to_owned()
}

/// Identifiers used in queries on the dashboards table.
#[derive(DeriveIden)]
enum Dashboards {
    Table,
    Id,
    DashboardId,
    FolderId,
    Owner,
    Role,
    Title,
    Description,
    Data,
    Version,
    CreatedAt,
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Id,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &create_dashboards_table_statement().to_string(PostgresQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "dashboards" ( 
                "id" bigserial NOT NULL PRIMARY KEY, 
                "dashboard_id" varchar(256) NOT NULL, 
                "folder_id" bigint NOT NULL, 
                "owner" varchar(256) NOT NULL, 
                "role" varchar(256) NULL, 
                "title" varchar(256) NOT NULL, 
                "description" text NULL, 
                "data" json NOT NULL,
                "version" integer NOT NULL, 
                "created_at" bigint NOT NULL, 
                CONSTRAINT "dashboards_folders_fk" FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") 
            )"#
        );
        assert_eq!(
            &create_dashboards_folder_id_dashboard_id_idx_stmnt().to_string(PostgresQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "dashboards_folder_id_dashboard_id_idx" ON "dashboards" ("folder_id", "dashboard_id")"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_dashboards_table_statement().to_string(MysqlQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS `dashboards` ( 
                `id` bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `dashboard_id` varchar(256) NOT NULL, 
                `folder_id` bigint NOT NULL, 
                `owner` varchar(256) NOT NULL, 
                `role` varchar(256) NULL, 
                `title` varchar(256) NOT NULL, 
                `description` text NULL,
                `data` json NOT NULL,
                `version` int NOT NULL, 
                `created_at` bigint NOT NULL, 
                CONSTRAINT `dashboards_folders_fk` FOREIGN KEY (`folder_id`) REFERENCES `folders` (`id`) 
            )"#
        );
        assert_eq!(
            &create_dashboards_folder_id_dashboard_id_idx_stmnt().to_string(MysqlQueryBuilder),
            r#"CREATE UNIQUE INDEX `dashboards_folder_id_dashboard_id_idx` ON `dashboards` (`folder_id`, `dashboard_id`)"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_dashboards_table_statement().to_string(SqliteQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "dashboards" ( 
                "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
                "dashboard_id" varchar(256) NOT NULL, 
                "folder_id" bigint NOT NULL, 
                "owner" varchar(256) NOT NULL, 
                "role" varchar(256) NULL, 
                "title" varchar(256) NOT NULL, 
                "description" text NULL, 
                "data" json_text NOT NULL, 
                "version" integer NOT NULL, 
                "created_at" bigint NOT NULL, 
                FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") 
            )"#
        );
        assert_eq!(
            &create_dashboards_folder_id_dashboard_id_idx_stmnt().to_string(SqliteQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "dashboards_folder_id_dashboard_id_idx" ON "dashboards" ("folder_id", "dashboard_id")"#
        );
    }
}
