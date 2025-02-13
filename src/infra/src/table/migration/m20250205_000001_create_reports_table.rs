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

const REPORTS_FOLDERS_FK: &str = "reports_folders_fk";
const REPORTS_FOLDER_ID_IDX: &str = "reports_folder_id_idx";

const REPORT_DASHBOARDS_REPORTS_FK: &str = "report_dashboards_reports_fk";
const REPORT_DASHBOARDS_DASHBOARDS_FK: &str = "report_dashboards_dashboards_fk";
const REPORT_DASHBOARDS_DASHBOARD_ID_IDX: &str = "report_dashboards_dashboard_id_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_reports_table_statement())
            .await?;
        manager
            .create_index(create_reports_folder_id_idx_stmnt())
            .await?;
        manager
            .create_table(create_report_dashboards_table_statement())
            .await?;
        manager
            .create_index(create_report_dashboards_dashboard_id_idx_stmnt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
        .drop_index(
            Index::drop()
                .name(REPORT_DASHBOARDS_DASHBOARD_ID_IDX)
                .table(ReportDashboards::Table)
                .to_owned(),
        )
        .await?;
        manager
            .drop_table(Table::drop().table(ReportDashboards::Table).to_owned())
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(REPORTS_FOLDER_ID_IDX)
                    .table(Reports::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Reports::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the reports table.
fn create_reports_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Reports::Table)
        .if_not_exists()
        // The ID is 27-character human readable KSUID.
        .col(
            ColumnDef::new(Reports::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        // Foreign key to the folders table. This is a 27-character human
        // readable KSUID.
        .col(ColumnDef::new(Reports::FolderId)
            .char_len(27)
            .not_null())
        .col(ColumnDef::new(Reports::Name).string_len(256).not_null())
        .col(ColumnDef::new(Reports::Title).string_len(256).not_null())
        .col(ColumnDef::new(Reports::Description).text().null())
        .col(ColumnDef::new(Reports::Enabled).boolean().not_null())
        .col(ColumnDef::new(Reports::Frequency).json().not_null())
        .col(ColumnDef::new(Reports::Destinations).json().not_null())        
        .col(ColumnDef::new(Reports::Message).text().null())
        .col(
            ColumnDef::new(Reports::ChromeDriverLoginEmail)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(Reports::ChromeDriverLoginPassword)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(Reports::Timezone)
                .string_len(256)
                .not_null(),
        )
        .col(ColumnDef::new(Reports::TzOffset).integer().not_null())
        .col(ColumnDef::new(Reports::Owner).string_len(256).null())
        .col(ColumnDef::new(Reports::LastEditedBy).string_len(256).null())
        .col(ColumnDef::new(Reports::CreatedAt).big_integer().not_null())
        .col(ColumnDef::new(Reports::UpdatedAt).big_integer().null())
        .col(ColumnDef::new(Reports::StartAt).big_integer().not_null())
        .col(ColumnDef::new(Reports::LastTriggeredAt).big_integer().null())
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(REPORTS_FOLDERS_FK)
                    .from(Reports::Table, Reports::FolderId)
                    .to(Folders::Table, Folders::Id)
        )
        .to_owned()
}

/// Statement to create index on the folder_id column of the reports table.
fn create_reports_folder_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(REPORTS_FOLDER_ID_IDX)
        .table(Reports::Table)
        .col(Reports::FolderId)
        .to_owned()
}

/// Statement to create the report_dashboards table.
fn create_report_dashboards_table_statement() -> TableCreateStatement {
    Table::create()
        .table(ReportDashboards::Table)
        .if_not_exists()
        // The ID is 27-character human readable KSUID.
        .col(
            ColumnDef::new(ReportDashboards::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        // Foreign key to the reports table. This is a 27-character human
        // readable KSUID.
        .col(ColumnDef::new(ReportDashboards::ReportId)
            .char_len(27)
            .not_null())
        // Foreign key to the dashboards table. This is a 27-character human
        // readable KSUID.
        .col(ColumnDef::new(ReportDashboards::DashboardId)
            .char_len(27)
            .not_null())
        .col(ColumnDef::new(ReportDashboards::TabNames).json().not_null())        
        .col(ColumnDef::new(ReportDashboards::Variables).json().not_null())        
        .col(ColumnDef::new(ReportDashboards::Timerange).json().not_null())        
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(REPORT_DASHBOARDS_REPORTS_FK)
                    .from(ReportDashboards::Table, ReportDashboards::ReportId)
                    .to(Reports::Table, Reports::Id)
                    .on_delete(ForeignKeyAction::Cascade)
        ) 
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(REPORT_DASHBOARDS_DASHBOARDS_FK)
                    .from(ReportDashboards::Table, ReportDashboards::DashboardId)
                    .to(Dashboards::Table, Dashboards::Id)
                    .on_delete(ForeignKeyAction::Cascade)
        )
        .to_owned()
}

/// Statement to create index on the report_id column of the report_dashboards
/// table.
fn create_report_dashboards_dashboard_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(REPORT_DASHBOARDS_DASHBOARD_ID_IDX)
        .table(ReportDashboards::Table)
        .col(ReportDashboards::DashboardId)
        .to_owned()
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Id,
}

/// Identifiers used in queries on the dashboards table.
#[derive(DeriveIden)]
enum Dashboards {
    Table,
    Id,
}

/// Identifiers used in queries on the reports table.
#[derive(DeriveIden)]
enum Reports {
    Table,
    Id,
    FolderId,
    Name,
    Title,
    Description,
    Enabled,
    Frequency,
    Destinations,
    Message,
    ChromeDriverLoginEmail,
    ChromeDriverLoginPassword,
    Timezone,
    TzOffset,
    Owner,
    LastEditedBy,
    CreatedAt,
    UpdatedAt,
    StartAt,
    LastTriggeredAt,
}

/// Identifiers used in queries on the report_dashboards table.
#[derive(DeriveIden)]
enum ReportDashboards {
    Table,
    Id,
    ReportId,
    DashboardId,
    TabNames,
    Variables,
    Timerange,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &create_reports_table_statement().to_string(PostgresQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "reports" ( 
                "id" char(27) NOT NULL PRIMARY KEY,
                "folder_id" char(27) NOT NULL,
                "name" varchar(256) NOT NULL,
                "title" varchar(256) NOT NULL,
                "destinations" json NOT NULL,
                "description" text NULL,
                "message" text NULL,
                "enabled" bool NOT NULL,
                "chrome_driver_login_email" varchar(256) NOT NULL,
                "chrome_driver_login_password" varchar(256) NOT NULL,
                "timezone" varchar(256) NOT NULL,
                "tz_offset" integer NOT NULL,
                "owner" varchar(256) NULL,
                "last_edited_by" varchar(256) NULL,
                "created_at" bigint,
                "updated_at" bigint NULL,
                "start_at" bigint,
                "last_triggered_at" bigint NULL,
                CONSTRAINT "reports_folders_fk" FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") 
            )"#
        );

        collapsed_eq!(
            &create_report_dashboards_table_statement().to_string(PostgresQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "report_dashboards" ( 
                "id" char(27) NOT NULL PRIMARY KEY,
                "report_id" char(27) NOT NULL,
                "dashboard_id" char(27) NOT NULL,
                "tabs" json NOT NULL,
                "variables" json NOT NULL,
                "timerange" json NOT NULL,
                CONSTRAINT "report_dashboards_reports_fk" FOREIGN KEY ("report_id") REFERENCES "reports" ("id"),
                CONSTRAINT "report_dashboards_dashboards_fk" FOREIGN KEY ("dashboard_id") REFERENCES "dashboards" ("id")
            )"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_reports_table_statement().to_string(MysqlQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS `reports` ( 
                `id` char(27) NOT NULL PRIMARY KEY,
                `folder_id` char(27) NOT NULL,
                `name` varchar(256) NOT NULL,
                `title` varchar(256) NOT NULL,
                `destinations` json NOT NULL,
                `description` text NULL,
                `message` text NULL,
                `enabled` bool NOT NULL,
                `chrome_driver_login_email` varchar(256) NOT NULL,
                `chrome_driver_login_password` varchar(256) NOT NULL,
                `timezone` varchar(256) NOT NULL,
                `tz_offset` int NOT NULL,
                `owner` varchar(256) NULL,
                `last_edited_by` varchar(256) NULL,
                `created_at` bigint,
                `updated_at` bigint NULL,
                `start_at` bigint,
                `last_triggered_at` bigint NULL,
                CONSTRAINT `reports_folders_fk` FOREIGN KEY (`folder_id`) REFERENCES `folders` (`id`)
            )"#
        );

        collapsed_eq!(
            &create_report_dashboards_table_statement().to_string(MysqlQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS `report_dashboards` ( 
                `id` char(27) NOT NULL PRIMARY KEY,
                `report_id` char(27) NOT NULL,
                `dashboard_id` char(27) NOT NULL,
                `tabs` json NOT NULL,
                `variables` json NOT NULL,
                `timerange` json NOT NULL,
                CONSTRAINT `report_dashboards_reports_fk` FOREIGN KEY (`report_id`) REFERENCES `reports` (`id`),
                CONSTRAINT `report_dashboards_dashboards_fk` FOREIGN KEY (`dashboard_id`) REFERENCES `dashboards` (`id`) 
            )"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_reports_table_statement().to_string(SqliteQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "reports" ( 
                "id" char(27) NOT NULL PRIMARY KEY,
                "folder_id" char(27) NOT NULL,
                "name" varchar(256) NOT NULL,
                "title" varchar(256) NOT NULL,
                "destinations" json_text NOT NULL,
                "description" text NULL,
                "message" text NULL,
                "enabled" boolean NOT NULL,
                "chrome_driver_login_email" varchar(256) NOT NULL,
                "chrome_driver_login_password" varchar(256) NOT NULL,
                "timezone" varchar(256) NOT NULL,
                "tz_offset" integer NOT NULL,
                "owner" varchar(256) NULL,
                "last_edited_by" varchar(256) NULL,
                "created_at" bigint,
                "updated_at" bigint NULL,
                "start_at" bigint,
                "last_triggered_at" bigint NULL,
                FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") 
            )"#
        );

        collapsed_eq!(
            &create_report_dashboards_table_statement().to_string(SqliteQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "report_dashboards" ( 
                "id" char(27) NOT NULL PRIMARY KEY,
                "report_id" char(27) NOT NULL,
                "dashboard_id" char(27) NOT NULL,
                "tabs" json_text NOT NULL,
                "variables" json_text NOT NULL,
                "timerange" json_text NOT NULL,
                FOREIGN KEY ("report_id") REFERENCES "reports" ("id"),
                FOREIGN KEY ("dashboard_id") REFERENCES "dashboards" ("id")
            )"#
        );
    }
}
