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

//! Populates the `reports` table by transforming unstructured alert records from the `meta` table.

use std::str::FromStr;

use itertools::{self, Itertools};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, FromQueryResult, Paginator,
    PaginatorTrait, QueryFilter, SelectModel, Set, Statement, TransactionTrait,
};
use sea_orm_migration::prelude::*;
use svix_ksuid::{Ksuid, KsuidLike};

/// The value of the flag used to indicate the alerts folder type in the folders table.
const REPORTS_FOLDER_TYPE: i16 = 2;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;

        // Select all orgs that have an alert. Create a default alert folder for each org. Migrate
        // pages of 100 records at a time to avoid loading too many records into memory.
        let mut org_pages = OrgWithReport::paginate(&txn, 100);
        while let Some(orgs) = org_pages.fetch_and_next().await? {
            let folders = orgs
                .into_iter()
                .map(|o| folders_table::ActiveModel {
                    id: NotSet, // todo
                    org: Set(o.org_name().to_owned()),
                    folder_id: Set("default".to_owned()),
                    name: Set("default".to_owned()),
                    description: Set(Some("default".to_owned())),
                    r#type: Set(REPORTS_FOLDER_TYPE),
                })
                .collect_vec();
            folders_table::Entity::insert_many(folders)
                .exec(&txn)
                .await?;
        }

        // Select each report from the meta table joined on its associated default folder. For each
        // report from the meta table insert a corresponding record in the new reports table.
        // Migrate pages of 100 records at a time to avoid loading too many records into
        // memory.
        let mut meta_pages = MetaReportWithFolder::paginate(&txn, 100);
        while let Some(metas) = meta_pages.fetch_and_next().await? {
            for m in metas {
                // Use the report JSON from the meta table and the associated folder ID to create a
                // new record in the new reports table.
                let folder_id = m.folder_id().map_err(DbErr::Migration)?;
                let meta_report: meta_table_reports::Report =
                    (&m).try_into().map_err(DbErr::Migration)?;
                let report_active_model: reports_table::ActiveModel = (folder_id, &meta_report)
                    .try_into()
                    .map_err(DbErr::Migration)?;
                let report_model = report_active_model.insert(&txn).await?;
                let report_id = Ksuid::from_str(&report_model.id).map_err(|_| {
                    DbErr::Migration(
                        "Generated report ID that cannot be parsed as a KSUID".to_string(),
                    )
                })?;

                // Look up each of the associated dashboards and use each to create a new record in
                // the new report_dashboards table.
                let org_id = &meta_report.org_id;
                for meta_rd in meta_report.dashboards.iter() {
                    let folder_snowflake_id = &meta_rd.folder;
                    let dashboard_snowflake_id = &meta_rd.dashboard;
                    let dashboard_and_folder = dashboards_table::Entity::find()
                        .filter(dashboards_table::Column::DashboardId.eq(dashboard_snowflake_id))
                        .find_also_related(folders_table::Entity)
                        .filter(folders_table::Column::Org.eq(org_id))
                        .filter(folders_table::Column::FolderId.eq(folder_snowflake_id))
                        .one(&txn)
                        .await?
                        .and_then(|(d, maybe_f)| maybe_f.map(|f| (d, f)));

                    // Sometimes the data from the meta table can reference dashboards that have
                    // been deleted. If we can't find the dashboard then we skip adding a
                    // corresponding record to the report_dashboards table.
                    if let Some((dashboard, _)) = dashboard_and_folder {
                        let dashbaord_id = Ksuid::from_str(&dashboard.id).map_err(|_| {
                            DbErr::Migration(
                                "Generated report ID that cannot be parsed as a KSUID".to_string(),
                            )
                        })?;

                        let report_dashboard_am: report_dashboards_table::ActiveModel =
                            (report_id.clone(), dashbaord_id, meta_rd)
                                .try_into()
                                .map_err(DbErr::Migration)?;
                        report_dashboard_am.insert(&txn).await?;
                    }
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        report_dashboards_table::Entity::delete_many()
            .exec(db)
            .await?;
        reports_table::Entity::delete_many().exec(db).await?;
        folders_table::Entity::delete_many()
            .filter(folders_table::Column::Type.eq(REPORTS_FOLDER_TYPE))
            .exec(db)
            .await?;
        Ok(())
    }
}

/// Identifiers used in queries on the meta table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Id,
    Org,
    FolderId,
    Type,
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Meta {
    Table,
    Module,
    Key1,
    Value,
}

/// A result from querying for unique names of orgs with reports from the meta table.
#[derive(Debug, FromQueryResult)]
pub struct OrgWithReport {
    /// The name of the org.
    key1: String,
}

impl OrgWithReport {
    pub fn org_name(&self) -> &str {
        &self.key1
    }
}

impl OrgWithReport {
    /// Returns the select statment to select unique names of orgs with reports from the meta table.
    fn statement() -> SelectStatement {
        Query::select()
            .column(Meta::Key1)
            .from(Meta::Table)
            .and_where(Expr::col(Meta::Module).eq("reports"))
            .group_by_col(Meta::Key1)
            .order_by(Meta::Key1, Order::Asc)
            .to_owned()
    }

    /// Paginate through the results of querying for unique names of orgs with reports from the meta
    /// table.
    fn paginate<C>(db: &C, page_size: u64) -> Paginator<'_, C, SelectModel<OrgWithReport>>
    where
        C: ConnectionTrait,
    {
        let select_statement = Self::statement();
        let backend = db.get_database_backend();
        let (sql, values) = match backend {
            sea_orm::DatabaseBackend::MySql => select_statement.build(MysqlQueryBuilder),
            sea_orm::DatabaseBackend::Postgres => select_statement.build(PostgresQueryBuilder),
            sea_orm::DatabaseBackend::Sqlite => select_statement.build(SqliteQueryBuilder),
        };
        let statement = Statement::from_sql_and_values(backend, sql, values);
        Self::find_by_statement(statement).paginate(db, page_size)
    }
}

/// A result from querying for reports from the meta table and joining on the folders table.
#[derive(Debug, FromQueryResult)]
pub struct MetaReportWithFolder {
    /// The JSON representation of the report.
    value: String,

    /// The ID of the folder that the report belongs to.
    id: Option<String>,
}

impl MetaReportWithFolder {
    /// Returns the JSON represenation of the report that was stored in the meta table's value
    /// column.
    pub fn report_json(&self) -> &str {
        &self.value
    }

    /// Returns the KSUID of the folder that the report belongs to.
    pub fn folder_id(&self) -> Result<Ksuid, String> {
        if let Some(id) = &self.id {
            Ksuid::from_str(id).map_err(|e| e.to_string())
        } else {
            return Err("Could not find folder for the report".to_owned());
        }
    }
}

impl MetaReportWithFolder {
    /// Returns the select statment to select alerts from the meta table joined on the folders
    /// table.
    fn statement() -> SelectStatement {
        Query::select()
            .column((Meta::Table, Meta::Value))
            .column((Folders::Table, Folders::Id))
            .from(Meta::Table)
            .left_join(
                Folders::Table,
                Expr::col((Folders::Table, Folders::Org)).equals((Meta::Table, Meta::Key1)),
            )
            .and_where(Expr::col((Meta::Table, Meta::Module)).eq("reports"))
            .and_where(Expr::col((Folders::Table, Folders::Type)).eq(REPORTS_FOLDER_TYPE))
            .and_where(Expr::col((Folders::Table, Folders::FolderId)).eq("default"))
            .to_owned()
    }

    /// Paginate through the results of querying for reports from the meta table and joining on the
    /// folders table.
    fn paginate<C>(db: &C, page_size: u64) -> Paginator<'_, C, SelectModel<MetaReportWithFolder>>
    where
        C: ConnectionTrait,
    {
        let select_statement = Self::statement();
        let backend = db.get_database_backend();
        let (sql, values) = match backend {
            sea_orm::DatabaseBackend::MySql => select_statement.build(MysqlQueryBuilder),
            sea_orm::DatabaseBackend::Postgres => select_statement.build(PostgresQueryBuilder),
            sea_orm::DatabaseBackend::Sqlite => select_statement.build(SqliteQueryBuilder),
        };
        let statement = Statement::from_sql_and_values(backend, sql, values);
        Self::find_by_statement(statement).paginate(db, page_size)
    }
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

/// Representation of the folders table at the time this migration executes.
mod folders_table {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "folders")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub org: String,
        pub folder_id: String,
        pub name: String,
        pub description: Option<String>,
        pub r#type: i16,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::dashboards_table::Entity")]
        Dashboards,
    }

    impl Related<super::dashboards_table::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Dashboards.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the reports table at the time this migration executes.
mod reports_table {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "reports")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub folder_id: String,
        pub name: String,
        pub title: String,
        pub description: Option<String>,
        pub enabled: bool,
        pub frequency: Json,
        pub destinations: Json,
        pub message: Option<String>,
        pub chrome_driver_login_email: Option<String>,
        pub chrome_driver_login_password: Option<String>,
        pub timezone: String,
        pub tz_offset: i32,
        pub owner: Option<String>,
        pub last_edited_by: Option<String>,
        pub created_at: i64,
        pub updated_at: Option<i64>,
        pub start_at: i64,
        pub last_triggered_at: Option<i64>,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the report_dashboards table at the time this migration executes.
mod report_dashboards_table {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "report_dashboards")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub report_id: String,
        pub dashboard_id: String,
        pub tab_names: Json,
        pub variables: Json,
        pub timerange: Json,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the dashboards table at the time this migration executes.
mod dashboards_table {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "dashboards")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub dashboard_id: String,
        pub folder_id: String,
        pub owner: String,
        pub role: Option<String>,
        pub title: String,
        pub description: Option<String>,
        pub data: Json,
        pub version: i32,
        pub created_at: i64,
    }

    // There are other relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::folders_table::Entity",
            from = "Column::FolderId",
            to = "super::folders_table::Column::Id",
            on_update = "NoAction",
            on_delete = "NoAction"
        )]
        Folders,
    }

    impl Related<super::folders_table::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Folders.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Defines the string and JSON serialization schemas used to store various child structures in the
/// new reports table at the time this migration executes.
mod reports_table_ser {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct ReportDestinations(pub Vec<ReportDestination>);

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ReportDestination {
        Email(String),
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ReportFrequency {
        Once,
        HourlyInterval(u32),
        DailyInterval(u32),
        WeeklyInterval(u32),
        MonthlyInterval(u32),
        Cron(String),
    }
}

/// Defines the string and JSON serialization schemas used to store various child structures in the
/// new report_dashboards table at the time this migration executes.
mod report_dashboards_table_ser {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct TabNames(pub Vec<String>);

    #[derive(Serialize)]
    pub struct ReportDashboardVariables(pub Vec<ReportDashboardVariable>);

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct ReportDashboardVariable {
        pub key: String,
        pub value: String,
        pub id: Option<String>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum ReportTimerange {
        Relative { period: String },
        Absolute { from: i64, to: i64 },
    }
}

/// Defines the JSON schema used to store reports and their child structures in the meta table at
/// the time this migration executes.
mod meta_table_reports {

    use chrono::{DateTime, FixedOffset, Utc};
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Report {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub title: String,
        pub org_id: String,
        /// Frequency of report generation. E.g. - Weekly.
        #[serde(default)]
        pub frequency: ReportFrequency,
        /// Start time of report generation in UNIX microseconds.
        #[serde(default)]
        pub start: i64,
        pub dashboards: Vec<ReportDashboard>,
        pub destinations: Vec<ReportDestination>,
        #[serde(default)]
        pub description: String,
        /// Message to include in the email
        #[serde(default)]
        pub message: String,
        #[serde(default)]
        pub enabled: bool,
        // The report JSON in the meta table stores the media_type field but it is not used in this
        // migration. Since it can only by PDF all records are assumed to be PDF and the reports
        // table does not contain a media_type column. If additional media types are added in the
        // future a media_type column can be added then.
        #[allow(unused)]
        #[serde(default)]
        pub media_type: ReportMediaType,
        /// User email for chromedriver login
        #[serde(default)]
        pub user: String,
        /// User password for chromedriver login
        #[serde(default)]
        pub password: String,
        #[serde(default)]
        pub timezone: String,
        /// Fixed timezone offset in minutes
        #[serde(default)]
        #[serde(rename = "timezoneOffset")]
        pub tz_offset: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_triggered_at: Option<i64>,
        #[serde(default = "datetime_now")]
        pub created_at: DateTime<FixedOffset>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub updated_at: Option<DateTime<FixedOffset>>,
        pub owner: String,
        pub last_edited_by: String,
    }

    impl Default for Report {
        fn default() -> Self {
            Self {
                name: "".to_string(),
                title: "".to_string(),
                org_id: "".to_string(),
                frequency: ReportFrequency::default(),
                start: Utc::now().timestamp_micros(), // Now
                destinations: vec![],
                dashboards: vec![],
                description: "".to_string(),
                message: "".to_string(),
                enabled: false,
                media_type: ReportMediaType::default(),
                user: "".to_string(),
                password: "".to_string(),
                timezone: "".to_string(),
                tz_offset: 0, // UTC
                last_triggered_at: None,
                created_at: datetime_now(),
                updated_at: None,
                owner: "".to_string(),
                last_edited_by: "".to_string(),
            }
        }
    }

    impl Default for ReportFrequency {
        fn default() -> Self {
            Self {
                interval: 1,
                cron: "".to_string(),
                frequency_type: Default::default(),
            }
        }
    }

    #[derive(Default, Deserialize, PartialEq)]
    pub enum ReportFrequencyType {
        #[serde(rename = "once")]
        Once,
        #[serde(rename = "hours")]
        Hours,
        #[serde(rename = "days")]
        Days,
        #[serde(rename = "weeks")]
        #[default]
        Weeks,
        #[serde(rename = "months")]
        Months,
        #[serde(rename = "cron")]
        Cron,
    }

    #[derive(Deserialize)]
    pub enum ReportDestination {
        #[serde(rename = "email")]
        Email(String), // Supports email only
    }

    #[derive(Default, Deserialize)]
    pub enum ReportMediaType {
        #[default]
        #[serde(rename = "pdf")]
        Pdf, // Supports Pdf only
    }

    #[derive(Deserialize)]
    pub struct ReportDashboard {
        pub dashboard: String,
        pub folder: String,
        pub tabs: Vec<String>,
        #[serde(default)]
        pub variables: Vec<ReportDashboardVariable>,
        /// The timerange of dashboard data.
        #[serde(default)]
        pub timerange: ReportTimerange,
    }

    #[derive(Default, Deserialize)]
    pub struct ReportDashboardVariable {
        pub key: String,
        pub value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct ReportTimerange {
        #[serde(rename = "type")]
        pub range_type: ReportTimerangeType,
        pub period: String, // 15m, 4M etc. For relative.
        pub from: i64,      // For absolute, in microseconds
        pub to: i64,        // For absolute, in microseconds
    }

    impl Default for ReportTimerange {
        fn default() -> Self {
            Self {
                range_type: ReportTimerangeType::default(),
                period: "1w".to_string(),
                from: 0,
                to: 0,
            }
        }
    }

    #[derive(Default, Deserialize)]
    pub enum ReportTimerangeType {
        #[default]
        #[serde(rename = "relative")]
        Relative,
        #[serde(rename = "absolute")]
        Absolute,
    }

    #[derive(Deserialize)]
    pub struct ReportFrequency {
        /// Frequency interval in the `frequency_type` unit
        #[serde(default)]
        pub interval: i64,
        /// Cron expression
        #[serde(default)]
        pub cron: String,
        #[serde(rename = "type")]
        #[serde(default)]
        pub frequency_type: ReportFrequencyType,
    }

    pub fn datetime_now() -> DateTime<FixedOffset> {
        Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()) // This can't fail.
    }
}

// Implementations of conversion traits for converting from the JSON representation of data
// structures in the old meta table to the JSON representation of data structures in the new tables.
// Some data structures are identical from the old meta table to the new tables but others differ so
// we explicitly specify the conversion logic for each here.

impl TryFrom<&MetaReportWithFolder> for meta_table_reports::Report {
    type Error = String;

    fn try_from(m: &MetaReportWithFolder) -> Result<Self, Self::Error> {
        let report: meta_table_reports::Report = serde_json::from_str(m.report_json())
            .map_err(|_| "Report in meta table could not be deserialized")?;
        Ok(report)
    }
}

impl TryFrom<(Ksuid, &meta_table_reports::Report)> for reports_table::ActiveModel {
    type Error = String;

    fn try_from(
        (folder_id, report_json): (Ksuid, &meta_table_reports::Report),
    ) -> Result<Self, Self::Error> {
        let id = report_ksuid_from_hash(report_json).to_string();
        let folder_id = folder_id.to_string();

        let frequency: reports_table_ser::ReportFrequency = (&report_json.frequency).try_into()?;
        let frequency_json = serde_json::to_value(&frequency).map_err(|e| e.to_string())?;

        let destinations: reports_table_ser::ReportDestinations =
            report_json.destinations.as_slice().into();
        let destionations_json = serde_json::to_value(&destinations).map_err(|e| e.to_string())?;

        Ok(reports_table::ActiveModel {
            id: Set(id),
            folder_id: Set(folder_id),
            name: Set(report_json.name.clone()),
            title: Set(report_json.title.clone()),
            description: Set(Some(report_json.description.clone()).filter(|s| !s.is_empty())),
            enabled: Set(report_json.enabled),
            frequency: Set(frequency_json),
            destinations: Set(destionations_json),
            message: Set(Some(report_json.message.clone()).filter(|s| !s.is_empty())),
            chrome_driver_login_email: Set(Some(report_json.user.clone()).filter(|s| !s.is_empty())),
            chrome_driver_login_password: Set(
                Some(report_json.password.clone()).filter(|s| !s.is_empty())
            ),
            timezone: Set(report_json.timezone.clone()),
            tz_offset: Set(report_json.tz_offset),
            owner: Set(Some(report_json.owner.clone()).filter(|s| !s.is_empty())),
            last_edited_by: Set(Some(report_json.last_edited_by.clone()).filter(|s| !s.is_empty())),
            created_at: Set(report_json.created_at.timestamp_micros()),
            updated_at: Set(report_json.updated_at.map(|t| t.timestamp_micros())),
            start_at: Set(report_json.start),
            last_triggered_at: Set(report_json.last_triggered_at),
        })
    }
}

impl TryFrom<&meta_table_reports::ReportFrequency> for reports_table_ser::ReportFrequency {
    type Error = String;

    fn try_from(value: &meta_table_reports::ReportFrequency) -> Result<Self, Self::Error> {
        match value.frequency_type {
            meta_table_reports::ReportFrequencyType::Once => Ok(Self::Once),
            meta_table_reports::ReportFrequencyType::Hours => {
                let interval = value
                    .interval
                    .try_into()
                    .or(Err("interval is negative".to_owned()))?;
                Ok(Self::HourlyInterval(interval))
            }
            meta_table_reports::ReportFrequencyType::Days => {
                let interval = value
                    .interval
                    .try_into()
                    .or(Err("interval is negative".to_owned()))?;
                Ok(Self::DailyInterval(interval))
            }
            meta_table_reports::ReportFrequencyType::Weeks => {
                let interval = value
                    .interval
                    .try_into()
                    .or(Err("interval is negative".to_owned()))?;
                Ok(Self::WeeklyInterval(interval))
            }
            meta_table_reports::ReportFrequencyType::Months => {
                let interval = value
                    .interval
                    .try_into()
                    .or(Err("interval is negative".to_owned()))?;
                Ok(Self::MonthlyInterval(interval))
            }
            meta_table_reports::ReportFrequencyType::Cron => Ok(Self::Cron(value.cron.clone())),
        }
    }
}

impl From<&[meta_table_reports::ReportDestination]> for reports_table_ser::ReportDestinations {
    fn from(value: &[meta_table_reports::ReportDestination]) -> Self {
        Self(value.iter().map(|d| d.into()).collect())
    }
}

impl From<&meta_table_reports::ReportDestination> for reports_table_ser::ReportDestination {
    fn from(value: &meta_table_reports::ReportDestination) -> Self {
        match value {
            meta_table_reports::ReportDestination::Email(email) => Self::Email(email.clone()),
        }
    }
}

impl TryFrom<(Ksuid, Ksuid, &meta_table_reports::ReportDashboard)>
    for report_dashboards_table::ActiveModel
{
    type Error = String;

    fn try_from(
        (report_id, dashboard_id, report_dashboard): (
            Ksuid,
            Ksuid,
            &meta_table_reports::ReportDashboard,
        ),
    ) -> Result<Self, Self::Error> {
        let id = report_dashboard_ksuid_from_hash(report_id, dashboard_id).to_string();
        let report_id = report_id.to_string();
        let dashboard_id = dashboard_id.to_string();

        let tab_names = report_dashboards_table_ser::TabNames(report_dashboard.tabs.clone());
        let tab_names_json = serde_json::to_value(&tab_names).map_err(|e| e.to_string())?;

        let variables: report_dashboards_table_ser::ReportDashboardVariables =
            report_dashboard.variables.as_slice().into();
        let variables_json = serde_json::to_value(&variables).map_err(|e| e.to_string())?;

        let timerange: report_dashboards_table_ser::ReportTimerange =
            (&report_dashboard.timerange).into();
        let timerange_json = serde_json::to_value(&timerange).map_err(|e| e.to_string())?;

        Ok(report_dashboards_table::ActiveModel {
            id: Set(id),
            report_id: Set(report_id),
            dashboard_id: Set(dashboard_id),
            tab_names: Set(tab_names_json),
            variables: Set(variables_json),
            timerange: Set(timerange_json),
        })
    }
}

impl From<&[meta_table_reports::ReportDashboardVariable]>
    for report_dashboards_table_ser::ReportDashboardVariables
{
    fn from(value: &[meta_table_reports::ReportDashboardVariable]) -> Self {
        Self(value.iter().map(|v| v.into()).collect())
    }
}

impl From<&meta_table_reports::ReportDashboardVariable>
    for report_dashboards_table_ser::ReportDashboardVariable
{
    fn from(value: &meta_table_reports::ReportDashboardVariable) -> Self {
        Self {
            key: value.key.clone(),
            value: value.value.clone(),
            id: value.id.clone(),
        }
    }
}

impl From<&meta_table_reports::ReportTimerange> for report_dashboards_table_ser::ReportTimerange {
    fn from(value: &meta_table_reports::ReportTimerange) -> Self {
        match value.range_type {
            meta_table_reports::ReportTimerangeType::Relative => {
                report_dashboards_table_ser::ReportTimerange::Relative {
                    period: value.period.clone(),
                }
            }
            meta_table_reports::ReportTimerangeType::Absolute => {
                report_dashboards_table_ser::ReportTimerange::Absolute {
                    from: value.from,
                    to: value.to,
                }
            }
        }
    }
}

/// Generates a KSUID from a hash of the report's `org_id` and `name`.
///
/// To generate a KSUID this function generates the 160-bit SHA-1 hash of the report's associated
/// organization ID and `name` and interprets that 160-bit hash as a 160-bit KSUID. Therefore two
/// KSUIDs generated in this manner will always be equal if the reports have the same associated
/// organization ID and `name`
///
/// It is important to note that KSUIDs generated in this manner will have
/// timestamp bits which are effectively random, meaning that the timestamp
/// in any KSUID generated with this function will be random.
fn report_ksuid_from_hash(report: &meta_table_reports::Report) -> svix_ksuid::Ksuid {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(report.org_id.clone());
    hasher.update(report.name.clone());
    let hash = hasher.finalize();
    svix_ksuid::Ksuid::from_bytes(hash.into())
}

/// Generates a KSUID from a hash of the report-to-dashboard relationship from the relationship's
/// `report_id` and `dashboard_id`.
///
/// To generate a KSUID this function generates the 160-bit SHA-1 hash of the relationship's
/// `report_id` and `dashboard_id` and interprets that 160-bit hash as a 160-bit KSUID. Therefore
/// two KSUIDs generated in this manner will always be equal if the reports have the same
/// `report_id` and `dashboard_id`.
///
/// It is important to note that KSUIDs generated in this manner will have
/// timestamp bits which are effectively random, meaning that the timestamp
/// in any KSUID generated with this function will be random.
fn report_dashboard_ksuid_from_hash(report_id: Ksuid, dashboard_id: Ksuid) -> svix_ksuid::Ksuid {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(report_id.to_string());
    hasher.update(dashboard_id.to_string());
    let hash = hasher.finalize();
    svix_ksuid::Ksuid::from_bytes(hash.into())
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn org_with_alert_postgres() {
        let stmnt = OrgWithReport::statement();
        assert_eq!(
            &stmnt.to_string(PostgresQueryBuilder),
            r#"SELECT "key1" FROM "meta" WHERE "module" = 'alerts' GROUP BY "key1""#
        );
    }

    #[test]
    fn org_with_alert_mysql() {
        let stmnt = OrgWithReport::statement();
        assert_eq!(
            &stmnt.to_string(MysqlQueryBuilder),
            r#"SELECT `key1` FROM `meta` WHERE `module` = 'alerts' GROUP BY `key1`"#
        );
    }

    #[test]
    fn org_with_alert_sqlite() {
        let stmnt = OrgWithReport::statement();
        assert_eq!(
            &stmnt.to_string(SqliteQueryBuilder),
            r#"SELECT "key1" FROM "meta" WHERE "module" = 'alerts' GROUP BY "key1""#
        );
    }

    #[test]
    fn meta_with_folder_postgres() {
        let stmnt = MetaReportWithFolder::statement();
        collapsed_eq!(
            &stmnt.to_string(PostgresQueryBuilder),
            r#"
                SELECT 
                "meta"."value", 
                "folders"."id" 
                FROM "meta" 
                LEFT JOIN "folders" 
                ON "folders"."org" = "meta"."key1" 
                WHERE "meta"."module" = 'alerts' 
                AND "folders"."type" = 1 
                AND "folders"."folder_id" = 'default'
            "#
        );
    }

    #[test]
    fn meta_with_folder_mysql() {
        let stmnt = MetaReportWithFolder::statement();
        collapsed_eq!(
            &stmnt.to_string(MysqlQueryBuilder),
            r#"
                SELECT 
                `meta`.`value`, 
                `folders`.`id` 
                FROM `meta` 
                LEFT JOIN `folders` 
                ON `folders`.`org` = `meta`.`key1` 
                WHERE `meta`.`module` = 'alerts' 
                AND `folders`.`type` = 1 
                AND `folders`.`folder_id` = 'default'
            "#
        );
    }

    #[test]
    fn meta_with_folder_sqlite() {
        let stmnt = MetaReportWithFolder::statement();
        collapsed_eq!(
            &stmnt.to_string(SqliteQueryBuilder),
            r#"
                SELECT
                "meta"."value", 
                "folders"."id" 
                FROM "meta" 
                LEFT JOIN "folders" 
                ON "folders"."org" = "meta"."key1" 
                WHERE "meta"."module" = 'alerts' 
                AND "folders"."type" = 1 
                AND "folders"."folder_id" = 'default'
            "#
        );
    }
}
