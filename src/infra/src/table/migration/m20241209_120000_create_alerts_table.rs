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

const ALERTS_FOLDERS_FK: &str = "alerts_folders_fk";
const ALERTS_ORG_STREAM_TYPE_STREAM_NAME_NAME_IDX: &str =
    "alerts_org_stream_type_stream_name_name_idx";
const ALERTS_FOLDER_ID_IDX: &str = "alerts_folder_id_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_alerts_table_statement())
            .await?;
        manager
            .create_index(create_alerts_org_stream_type_stream_name_name_idx_stmnt())
            .await?;
        manager
            .create_index(create_alerts_folder_id_idx_stmnt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(ALERTS_FOLDER_ID_IDX)
                    .table(Alerts::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(ALERTS_ORG_STREAM_TYPE_STREAM_NAME_NAME_IDX)
                    .table(Alerts::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Alerts::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the alerts table.
fn create_alerts_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Alerts::Table)
        .if_not_exists()
        // The ID is 27-character human readable KSUID.
        .col(
            ColumnDef::new(Alerts::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        // The org field is redundant and should always match the org of the
        // associated folder. However we need the org column on this table in
        // order to enforce a uniqueness constraint that includeds org but not
        // folder_id.
        .col(ColumnDef::new(Alerts::Org).string_len(100).not_null())
        // Foreign key to the folders table.
        .col(ColumnDef::new(Alerts::FolderId).big_integer().not_null())
        .col(ColumnDef::new(Alerts::Name).string_len(256).not_null())
        .col(ColumnDef::new(Alerts::StreamType).string_len(50).not_null())
        .col(ColumnDef::new(Alerts::StreamName).string_len(256).not_null())
        .col(ColumnDef::new(Alerts::IsRealTime).boolean().not_null())
        .col(ColumnDef::new(Alerts::Destinations).json().not_null())
        .col(ColumnDef::new(Alerts::ContextAttributes).json().null())
        .col(ColumnDef::new(Alerts::RowTemplate).text().null())
        .col(ColumnDef::new(Alerts::Description).text().null())
        .col(ColumnDef::new(Alerts::Enabled).boolean().not_null())
        .col(ColumnDef::new(Alerts::TzOffset).integer().not_null())
        .col(ColumnDef::new(Alerts::LastTriggeredAt).big_integer().null())
        .col(ColumnDef::new(Alerts::LastSatisfiedAt).big_integer().null())
        // Query condition
        .col(
            ColumnDef::new(Alerts::QueryType).small_integer().not_null(),
        )
        .col(ColumnDef::new(Alerts::QueryConditions).json().null())
        .col(ColumnDef::new(Alerts::QuerySql).text().null())
        .col(ColumnDef::new(Alerts::QueryPromql).text().null())
        .col(
            ColumnDef::new(Alerts::QueryPromqlCondition)
                .json()
                .null(),
        )
        .col(ColumnDef::new(Alerts::QueryAggregation).json().null())
        .col(ColumnDef::new(Alerts::QueryVrlFunction).text().null())
        .col(
            ColumnDef::new(Alerts::QuerySearchEventType).small_integer().null(),
        )
        .col(
            ColumnDef::new(Alerts::QueryMultiTimeRange)
                .json()
                .null(),
        )
        // Trigger condition
        .col(
            ColumnDef::new(Alerts::TriggerThresholdOperator)
                .string_len(50)
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerPeriodSeconds)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerThresholdCount)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerFrequencyType).small_integer().not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerFrequencySeconds)
                .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(Alerts::TriggerFrequencyCron).text().null())
        .col(
            ColumnDef::new(Alerts::TriggerFrequencyCronTimezone)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerSilenceSeconds)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerToleranceSeconds)
                .big_integer()
                .null(),
        )
        // Ownership and update information.
        .col(ColumnDef::new(Alerts::Owner).string_len(256).null())
        .col(ColumnDef::new(Alerts::LastEditedBy).string_len(256).null())
        .col(ColumnDef::new(Alerts::UpdatedAt).big_integer().null())
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(ALERTS_FOLDERS_FK)
                    .from(Alerts::Table, Alerts::FolderId)
                    .to(Folders::Table, Folders::Id)
        )
        .to_owned()
}

/// Statement to create unique index on the org, stream_type, stream_name, and
/// name columns of the alerts table.
fn create_alerts_org_stream_type_stream_name_name_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(ALERTS_ORG_STREAM_TYPE_STREAM_NAME_NAME_IDX)
        .table(Alerts::Table)
        .col(Alerts::Org)
        .col(Alerts::StreamType)
        .col(Alerts::StreamName)
        .col(Alerts::Name)
        .unique()
        .to_owned()
}

/// Statement to create index on the folder_id column of the alerts table.
fn create_alerts_folder_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(ALERTS_FOLDER_ID_IDX)
        .table(Alerts::Table)
        .col(Alerts::FolderId)
        .to_owned()
}

/// Identifiers used in queries on the alerts table.
#[derive(DeriveIden)]
enum Alerts {
    Table,
    Id,
    Org,
    FolderId,
    Name,
    StreamType,
    StreamName,
    IsRealTime,
    Destinations,
    ContextAttributes,
    RowTemplate,
    Description,
    Enabled,
    TzOffset,
    LastTriggeredAt,
    LastSatisfiedAt,
    // Query condition
    QueryType,
    QueryConditions,
    QuerySql,
    QueryPromql,
    QueryPromqlCondition,
    QueryAggregation,
    QueryVrlFunction,
    QuerySearchEventType,
    QueryMultiTimeRange,
    // Trigger condition
    TriggerThresholdOperator,
    TriggerThresholdCount,
    TriggerFrequencyType,
    TriggerFrequencySeconds,
    TriggerFrequencyCron,
    TriggerFrequencyCronTimezone,
    TriggerPeriodSeconds,
    TriggerSilenceSeconds,
    TriggerToleranceSeconds,
    Owner,
    UpdatedAt,
    LastEditedBy,
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
            &create_alerts_table_statement().to_string(PostgresQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "alerts" ( "id" char(27) NOT NULL PRIMARY KEY, "org" varchar(100) NOT NULL, "folder_id" bigint NOT NULL, "name" varchar(256) NOT NULL, "stream_type" varchar(50) NOT NULL, "stream_name" varchar(256) NOT NULL, "is_real_time" bool NOT NULL, "destinations" json NOT NULL, "context_attributes" json NULL, "row_template" text NULL, "description" text NULL, "enabled" bool NOT NULL, "tz_offset" integer NOT NULL, "last_triggered_at" bigint NULL, "last_satisfied_at" bigint NULL, "query_type" smallint NOT NULL, "query_conditions" json NULL, "query_sql" text NULL, "query_promql" text NULL, "query_promql_condition" json NULL, "query_aggregation" json NULL, "query_vrl_function" text NULL, "query_search_event_type" smallint NULL, "query_multi_time_range" json NULL, "trigger_threshold_operator" varchar(50) NOT NULL, "trigger_period_seconds" bigint NOT NULL, "trigger_threshold_count" bigint NOT NULL, "trigger_frequency_type" smallint NOT NULL, "trigger_frequency_seconds" bigint NOT NULL, "trigger_frequency_cron" text NULL, "trigger_frequency_cron_timezone" varchar(256) NULL, "trigger_silence_seconds" bigint NOT NULL, "trigger_tolerance_seconds" bigint NULL, "owner" varchar(256) NULL, "last_edited_by" varchar(256) NULL, "updated_at" bigint NULL, CONSTRAINT "alerts_folders_fk" FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") )"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_alerts_table_statement().to_string(MysqlQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS `alerts` ( `id` char(27) NOT NULL PRIMARY KEY, `org` varchar(100) NOT NULL, `folder_id` bigint NOT NULL, `name` varchar(256) NOT NULL, `stream_type` varchar(50) NOT NULL, `stream_name` varchar(256) NOT NULL, `is_real_time` bool NOT NULL, `destinations` json NOT NULL, `context_attributes` json NULL, `row_template` text NULL, `description` text NULL, `enabled` bool NOT NULL, `tz_offset` int NOT NULL, `last_triggered_at` bigint NULL, `last_satisfied_at` bigint NULL, `query_type` smallint NOT NULL, `query_conditions` json NULL, `query_sql` text NULL, `query_promql` text NULL, `query_promql_condition` json NULL, `query_aggregation` json NULL, `query_vrl_function` text NULL, `query_search_event_type` smallint NULL, `query_multi_time_range` json NULL, `trigger_threshold_operator` varchar(50) NOT NULL, `trigger_period_seconds` bigint NOT NULL, `trigger_threshold_count` bigint NOT NULL, `trigger_frequency_type` smallint NOT NULL, `trigger_frequency_seconds` bigint NOT NULL, `trigger_frequency_cron` text NULL, `trigger_frequency_cron_timezone` varchar(256) NULL, `trigger_silence_seconds` bigint NOT NULL, `trigger_tolerance_seconds` bigint NULL, `owner` varchar(256) NULL, `last_edited_by` varchar(256) NULL, `updated_at` bigint NULL, CONSTRAINT `alerts_folders_fk` FOREIGN KEY (`folder_id`) REFERENCES `folders` (`id`) )"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_alerts_table_statement().to_string(SqliteQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "alerts" ( "id" char(27) NOT NULL PRIMARY KEY, "org" varchar(100) NOT NULL, "folder_id" bigint NOT NULL, "name" varchar(256) NOT NULL, "stream_type" varchar(50) NOT NULL, "stream_name" varchar(256) NOT NULL, "is_real_time" boolean NOT NULL, "destinations" json_text NOT NULL, "context_attributes" json_text NULL, "row_template" text NULL, "description" text NULL, "enabled" boolean NOT NULL, "tz_offset" integer NOT NULL, "last_triggered_at" bigint NULL, "last_satisfied_at" bigint NULL, "query_type" smallint NOT NULL, "query_conditions" json_text NULL, "query_sql" text NULL, "query_promql" text NULL, "query_promql_condition" json_text NULL, "query_aggregation" json_text NULL, "query_vrl_function" text NULL, "query_search_event_type" smallint NULL, "query_multi_time_range" json_text NULL, "trigger_threshold_operator" varchar(50) NOT NULL, "trigger_period_seconds" bigint NOT NULL, "trigger_threshold_count" bigint NOT NULL, "trigger_frequency_type" smallint NOT NULL, "trigger_frequency_seconds" bigint NOT NULL, "trigger_frequency_cron" text NULL, "trigger_frequency_cron_timezone" varchar(256) NULL, "trigger_silence_seconds" bigint NOT NULL, "trigger_tolerance_seconds" bigint NULL, "owner" varchar(256) NULL, "last_edited_by" varchar(256) NULL, "updated_at" bigint NULL, FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") )"#
        );
    }
}
