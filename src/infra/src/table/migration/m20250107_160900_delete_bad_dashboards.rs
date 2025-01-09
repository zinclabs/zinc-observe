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

//! Removes records from the dashboards table whose JSON data column cannot be
//! parsed into a dashboard.

use chrono::{DateTime, FixedOffset};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, PaginatorTrait, TransactionTrait};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;

        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        let mut pages = sea_orm_dashboards::Entity::find().paginate(&txn, 100);

        while let Some(dashboards) = pages.fetch_and_next().await? {
            for d in dashboards {
                if !can_parse_data(&d) {
                    let active_model = d.into_active_model();
                    active_model.delete(&txn).await?;
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // The deletion of the invalid dashboards cannot be reversed.
        Ok(())
    }
}

/// Returns true if the dashboard records JSON `data` field can be parsed into
/// v1, v2, v3, v4, or v5 dashboard.
pub fn can_parse_data(model: &sea_orm_dashboards::Model) -> bool {
    let mut data = model.data.clone();

    if let Some(obj) = data.as_object_mut() {
        // The domain model JSON deserialization logic for v1-v5 expects
        // some or all these fields to be present in the JSON even though we
        // store them in DB columns. Therefore we add these values back into
        // the JSON object so that deserializing the JSON can succeed.
        obj.insert("dashboardId".to_owned(), model.dashboard_id.clone().into());
        obj.insert("version".to_owned(), model.version.into());
        obj.insert("owner".to_owned(), model.owner.clone().into());
        obj.insert(
            "role".to_owned(),
            model.role.clone().unwrap_or_default().into(),
        );
        obj.insert("title".to_owned(), model.title.clone().into());
        obj.insert(
            "description".to_owned(),
            model.description.clone().unwrap_or_default().into(),
        );
    }

    match model.version {
        1 => {
            let parsed: Result<v1::Dashboard, _> = serde_json::from_value(data);
            parsed.is_ok()
        }
        2 => {
            let parsed: Result<v2::Dashboard, _> = serde_json::from_value(data);
            parsed.is_ok()
        }
        3 => {
            let parsed: Result<v3::Dashboard, _> = serde_json::from_value(data);
            parsed.is_ok()
        }
        4 => {
            let parsed: Result<v4::Dashboard, _> = serde_json::from_value(data);
            parsed.is_ok()
        }
        5 => {
            let parsed: Result<v5::Dashboard, _> = serde_json::from_value(data);
            parsed.is_ok()
        }
        _ => false,
    }
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

/// Representation of the dashboards table at the time this migration executes.
mod sea_orm_dashboards {
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
    #[derive(Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the v1 dashboard inner data schema at the time this
/// migration executes.
#[allow(dead_code)]
mod v1 {
    use chrono::{DateTime, FixedOffset};
    use serde::Deserialize;

    use super::{datetime_now, stream::StreamType};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dashboard {
        #[serde(default)]
        pub dashboard_id: String,
        pub title: String,
        pub description: String,
        #[serde(default)]
        pub role: String,
        #[serde(default)]
        pub owner: String,
        #[serde(default = "datetime_now")]
        pub created: DateTime<FixedOffset>,
        #[serde(default)]
        pub panels: Vec<Panel>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub layouts: Option<Vec<Layout>>,
        pub variables: Option<Variables>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Layout {
        pub x: i64,
        pub y: i64,
        pub w: i64,
        pub h: i64,
        pub i: i64,
        pub panel_id: String,
        #[serde(rename = "static")]
        pub is_static: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Panel {
        pub id: String,
        #[serde(rename = "type")]
        pub typ: String,
        pub fields: PanelFields,
        pub config: PanelConfig,
        pub query: String,
        #[serde(default)]
        pub query_type: String,
        pub custom_query: bool,
    }

    #[derive(Deserialize)]
    pub struct PanelFields {
        pub stream: String,
        pub stream_type: StreamType,
        pub x: Vec<AxisItem>,
        pub y: Vec<AxisItem>,
        pub filter: Vec<PanelFilter>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AxisItem {
        pub label: String,
        pub alias: String,
        pub column: String,
        pub color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aggregation_function: Option<AggregationFunc>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AggregationFunc {
        Count,
        #[serde(rename = "count-distinct")]
        CountDistinct,
        Histogram,
        Sum,
        Min,
        Max,
        Avg,
        Median,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelFilter {
        #[serde(rename = "type")]
        pub typ: String,
        pub values: Vec<String>,
        pub column: String,
        pub operator: Option<String>,
        pub value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct PanelConfig {
        title: String,
        description: String,
        show_legends: bool,
        legends_position: Option<String>,
        promql_legend: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_custom: Option<String>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Variables {
        pub list: Vec<VariableList>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariableList {
        #[serde(rename = "type")]
        pub type_field: String,
        pub name: String,
        pub label: String,
        #[serde(rename = "query_data")]
        pub query_data: Option<QueryData>,
        pub value: Option<String>,
        pub options: Option<Vec<CustomFieldsOption>>,
    }

    #[derive(Default, Deserialize)]
    pub struct QueryData {
        pub stream_type: StreamType,
        pub stream: String,
        pub field: String,
        pub max_record_size: Option<i64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CustomFieldsOption {
        pub label: String,
        pub value: String,
    }
}

/// Representation of the v2 dashboard inner data schema at the time this
/// migration executes.
#[allow(dead_code)]
mod v2 {
    use chrono::{DateTime, FixedOffset};
    use serde::Deserialize;

    use super::{datetime_now, stream::StreamType};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dashboard {
        version: i32,
        #[serde(default)]
        pub dashboard_id: String,
        pub title: String,
        pub description: String,
        #[serde(default)]
        pub role: String,
        #[serde(default)]
        pub owner: String,
        #[serde(default = "datetime_now")]
        pub created: DateTime<FixedOffset>,
        #[serde(default)]
        pub panels: Vec<Panel>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variables: Option<Variables>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Layout {
        pub x: i64,
        pub y: i64,
        pub w: i64,
        pub h: i64,
        pub i: i64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Panel {
        pub id: String,
        #[serde(rename = "type")]
        pub typ: String,
        pub title: String,
        pub description: String,
        pub config: PanelConfig,
        #[serde(default)]
        pub query_type: String,
        pub queries: Vec<Query>,
        pub layout: Layout,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Query {
        pub query: String,
        pub custom_query: bool,
        pub fields: PanelFields,
        pub config: QueryConfig,
    }

    #[derive(Deserialize)]
    pub struct PanelFields {
        pub stream: String,
        pub stream_type: StreamType,
        pub x: Vec<AxisItem>,
        pub y: Vec<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub z: Option<Vec<AxisItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub longitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub weight: Option<AxisItem>,
        pub filter: Vec<PanelFilter>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AxisItem {
        pub label: String,
        pub alias: String,
        pub column: String,
        pub color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aggregation_function: Option<AggregationFunc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sort_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub args: Option<Vec<AxisArg>>,
    }

    #[derive(Deserialize)]
    pub struct AxisArg {
        value: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AggregationFunc {
        Count,
        #[serde(rename = "count-distinct")]
        CountDistinct,
        Histogram,
        Sum,
        Min,
        Max,
        Avg,
        Median,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelFilter {
        #[serde(rename = "type")]
        pub typ: String,
        pub values: Vec<String>,
        pub column: String,
        pub operator: Option<String>,
        pub value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct PanelConfig {
        show_legends: bool,
        legends_position: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_custom: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        decimals: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_border_show: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        legend_width: Option<LegendWidth>,
        base_map: Option<BaseMap>,
        map_view: Option<MapView>,
    }

    #[derive(Deserialize)]
    pub struct QueryConfig {
        promql_legend: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        layer_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        weight_fixed: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Variables {
        pub list: Vec<VariableList>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub show_dynamic_filters: Option<bool>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariableList {
        #[serde(rename = "type")]
        pub type_field: String,
        pub name: String,
        pub label: String,
        #[serde(rename = "query_data")]
        pub query_data: Option<QueryData>,
        pub value: Option<String>,
        pub options: Option<Vec<CustomFieldsOption>>,
    }

    #[derive(Default, Deserialize)]
    pub struct QueryData {
        pub stream_type: StreamType,
        pub stream: String,
        pub field: String,
        pub max_record_size: Option<i64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CustomFieldsOption {
        pub label: String,
        pub value: String,
    }

    #[derive(Deserialize)]
    pub struct BaseMap {
        #[serde(rename = "type")]
        pub type_field: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct MapView {
        pub zoom: f64,
        pub lat: f64,
        pub lng: f64,
    }

    #[derive(Deserialize)]
    pub struct LegendWidth {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub unit: Option<String>,
    }
}

/// Representation of the v3 dashboard inner data schema at the time this
/// migration executes.
#[allow(dead_code)]
mod v3 {
    use chrono::{DateTime, FixedOffset};
    use serde::Deserialize;

    use super::{datetime_now, stream::StreamType};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dashboard {
        version: i32,
        #[serde(default)]
        pub dashboard_id: String,
        pub title: String,
        pub description: String,
        #[serde(default)]
        pub role: String,
        #[serde(default)]
        pub owner: String,
        #[serde(default = "datetime_now")]
        pub created: DateTime<FixedOffset>,
        #[serde(default)]
        pub tabs: Vec<Tab>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variables: Option<Variables>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_datetime_duration: Option<DateTimeOptions>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Layout {
        pub x: i64,
        pub y: i64,
        pub w: i64,
        pub h: i64,
        pub i: i64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Tab {
        pub tab_id: String,
        pub name: String,
        #[serde(default)]
        pub panels: Vec<Panel>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Panel {
        pub id: String,
        #[serde(rename = "type")]
        pub typ: String,
        pub title: String,
        pub description: String,
        pub config: PanelConfig,
        #[serde(default)]
        pub query_type: String,
        pub queries: Vec<Query>,
        pub layout: Layout,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub html_content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub markdown_content: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Query {
        pub query: Option<String>,
        pub custom_query: bool,
        pub fields: PanelFields,
        pub config: QueryConfig,
    }

    #[derive(Deserialize)]
    pub struct PanelFields {
        pub stream: String,
        pub stream_type: StreamType,
        pub x: Vec<AxisItem>,
        pub y: Vec<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub z: Option<Vec<AxisItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub longitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub weight: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub target: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<AxisItem>,
        pub filter: Vec<PanelFilter>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AxisItem {
        pub label: String,
        pub alias: String,
        pub column: String,
        pub color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aggregation_function: Option<AggregationFunc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sort_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub args: Option<Vec<AxisArg>>,
    }

    #[derive(Deserialize)]
    pub struct AxisArg {
        value: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AggregationFunc {
        Count,
        #[serde(rename = "count-distinct")]
        CountDistinct,
        Histogram,
        Sum,
        Min,
        Max,
        Avg,
        Median,
        P50,
        P90,
        P95,
        P99,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelFilter {
        #[serde(rename = "type")]
        pub typ: String,
        pub values: Vec<String>,
        pub column: String,
        pub operator: Option<String>,
        pub value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct PanelConfig {
        show_legends: bool,
        legends_position: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_custom: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        decimals: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_border_show: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        legend_width: Option<LegendWidth>,
        base_map: Option<BaseMap>,
        map_view: Option<MapView>,
        #[serde(skip_serializing_if = "Option::is_none")]
        map_symbol_style: Option<MapSymbolStyle>,
        #[serde(skip_serializing_if = "Option::is_none")]
        drilldown: Option<Vec<DrillDown>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        connect_nulls: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        no_value_replacement: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        wrap_table_cells: Option<bool>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DrillDown {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        type_field: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_blank: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        find_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<DrillDownData>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DrillDownData {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dashboard: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_all_variables: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        variables: Option<Vec<DrillDownVariables>>,
    }

    #[derive(Deserialize)]
    pub struct DrillDownVariables {
        name: Option<String>,
        value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct QueryConfig {
        promql_legend: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        layer_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        weight_fixed: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Variables {
        pub list: Vec<VariableList>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub show_dynamic_filters: Option<bool>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DateTimeOptions {
        #[serde(rename = "type")]
        pub typee: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub relative_time_period: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub start_time: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub end_time: Option<i64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariableList {
        #[serde(rename = "type")]
        pub type_field: String,
        pub name: String,
        pub label: String,
        #[serde(rename = "query_data")]
        pub query_data: Option<QueryData>,
        pub value: Option<String>,
        pub options: Option<Vec<CustomFieldsOption>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub multi_select: Option<bool>,
    }

    #[derive(Default, Deserialize)]
    pub struct QueryData {
        pub stream_type: StreamType,
        pub stream: String,
        pub field: String,
        pub max_record_size: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub filter: Option<Vec<Filters>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Filters {
        pub name: Option<String>,
        pub operator: Option<String>,
        pub value: String,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CustomFieldsOption {
        pub label: String,
        pub value: String,
    }

    #[derive(Deserialize)]
    pub struct BaseMap {
        #[serde(rename = "type")]
        pub type_field: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct MapView {
        pub zoom: f64,
        pub lat: f64,
        pub lng: f64,
    }

    #[derive(Deserialize)]
    pub struct MapSymbolStyle {
        pub size: String,
        pub size_by_value: Option<SizeByValue>,
        pub size_fixed: f64,
    }

    #[derive(Deserialize)]
    pub struct SizeByValue {
        pub min: f64,
        pub max: f64,
    }

    #[derive(Deserialize)]
    pub struct LegendWidth {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub unit: Option<String>,
    }
}

/// Representation of the v4 dashboard inner data schema at the time this
/// migration executes.
#[allow(dead_code)]
mod v4 {
    use chrono::{DateTime, FixedOffset};
    use serde::Deserialize;

    use super::{datetime_now, stream::StreamType};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dashboard {
        version: i32,
        #[serde(default)]
        pub dashboard_id: String,
        pub title: String,
        pub description: String,
        #[serde(default)]
        pub role: String,
        #[serde(default)]
        pub owner: String,
        #[serde(default = "datetime_now")]
        pub created: DateTime<FixedOffset>,
        #[serde(default)]
        pub tabs: Vec<Tab>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variables: Option<Variables>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_datetime_duration: Option<DateTimeOptions>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Layout {
        pub x: i64,
        pub y: i64,
        pub w: i64,
        pub h: i64,
        pub i: i64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Tab {
        pub tab_id: String,
        pub name: String,
        #[serde(default)]
        pub panels: Vec<Panel>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Panel {
        pub id: String,
        #[serde(rename = "type")]
        pub typ: String,
        pub title: String,
        pub description: String,
        pub config: PanelConfig,
        #[serde(default)]
        pub query_type: String,
        pub queries: Vec<Query>,
        pub layout: Layout,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub html_content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub markdown_content: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Query {
        pub query: Option<String>,
        pub vrl_function_query: Option<String>,
        pub custom_query: bool,
        pub fields: PanelFields,
        pub config: QueryConfig,
    }

    #[derive(Deserialize)]
    pub struct PanelFields {
        pub stream: String,
        pub stream_type: StreamType,
        pub x: Vec<AxisItem>,
        pub y: Vec<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub z: Option<Vec<AxisItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub breakdown: Option<Vec<AxisItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub longitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub weight: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub target: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<AxisItem>,
        pub filter: Vec<PanelFilter>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AxisItem {
        pub label: String,
        pub alias: String,
        pub column: String,
        pub color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aggregation_function: Option<AggregationFunc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sort_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub args: Option<Vec<AxisArg>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub is_derived: Option<bool>,
    }

    #[derive(Deserialize)]
    pub struct AxisArg {
        value: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AggregationFunc {
        Count,
        #[serde(rename = "count-distinct")]
        CountDistinct,
        Histogram,
        Sum,
        Min,
        Max,
        Avg,
        Median,
        P50,
        P90,
        P95,
        P99,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelFilter {
        #[serde(rename = "type")]
        pub typ: String,
        pub values: Vec<String>,
        pub column: String,
        pub operator: Option<String>,
        pub value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct PanelConfig {
        show_legends: bool,
        legends_position: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_custom: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        decimals: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_results: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_results_others: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_border_show: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        legend_width: Option<LegendWidth>,
        base_map: Option<BaseMap>,
        map_view: Option<MapView>,
        #[serde(skip_serializing_if = "Option::is_none")]
        map_symbol_style: Option<MapSymbolStyle>,
        #[serde(skip_serializing_if = "Option::is_none")]
        drilldown: Option<Vec<DrillDown>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mark_line: Option<Vec<MarkLine>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        connect_nulls: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        no_value_replacement: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        wrap_table_cells: Option<bool>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DrillDown {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        type_field: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_blank: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        find_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<DrillDownData>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MarkLine {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        typee: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DrillDownData {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dashboard: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_all_variables: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        variables: Option<Vec<DrillDownVariables>>,
    }

    #[derive(Deserialize)]
    pub struct DrillDownVariables {
        name: Option<String>,
        value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct QueryConfig {
        promql_legend: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        layer_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        weight_fixed: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Variables {
        pub list: Vec<VariableList>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub show_dynamic_filters: Option<bool>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DateTimeOptions {
        #[serde(rename = "type")]
        pub typee: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub relative_time_period: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub start_time: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub end_time: Option<i64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariableList {
        #[serde(rename = "type")]
        pub type_field: String,
        pub name: String,
        pub label: String,
        #[serde(rename = "query_data")]
        pub query_data: Option<QueryData>,
        pub value: Option<String>,
        pub options: Option<Vec<CustomFieldsOption>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub multi_select: Option<bool>,
    }

    #[derive(Default, Deserialize)]
    pub struct QueryData {
        pub stream_type: StreamType,
        pub stream: String,
        pub field: String,
        pub max_record_size: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub filter: Option<Vec<Filters>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Filters {
        pub name: Option<String>,
        pub operator: Option<String>,
        pub value: String,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CustomFieldsOption {
        pub label: String,
        pub value: String,
    }

    #[derive(Deserialize)]
    pub struct BaseMap {
        #[serde(rename = "type")]
        pub type_field: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct MapView {
        pub zoom: f64,
        pub lat: f64,
        pub lng: f64,
    }

    #[derive(Deserialize)]
    pub struct MapSymbolStyle {
        pub size: String,
        pub size_by_value: Option<SizeByValue>,
        pub size_fixed: f64,
    }

    #[derive(Deserialize)]
    pub struct SizeByValue {
        pub min: f64,
        pub max: f64,
    }

    #[derive(Deserialize)]
    pub struct LegendWidth {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub unit: Option<String>,
    }
}

/// Representation of the v5 dashboard inner data schema at the time this
/// migration executes.
#[allow(dead_code)]
mod v5 {
    use chrono::{DateTime, FixedOffset};
    use serde::Deserialize;

    use super::{datetime_now, stream::StreamType};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dashboard {
        version: i32,
        #[serde(default)]
        pub dashboard_id: String,
        pub title: String,
        pub description: String,
        #[serde(default)]
        pub role: String,
        #[serde(default)]
        pub owner: String,
        #[serde(default = "datetime_now")]
        pub created: DateTime<FixedOffset>,
        #[serde(default)]
        pub tabs: Vec<Tab>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub variables: Option<Variables>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_datetime_duration: Option<DateTimeOptions>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Layout {
        pub x: i64,
        pub y: i64,
        pub w: i64,
        pub h: i64,
        pub i: i64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Tab {
        pub tab_id: String,
        pub name: String,
        #[serde(default)]
        pub panels: Vec<Panel>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Panel {
        pub id: String,
        #[serde(rename = "type")]
        pub typ: String,
        pub title: String,
        pub description: String,
        pub config: PanelConfig,
        #[serde(default)]
        pub query_type: String,
        pub queries: Vec<Query>,
        pub layout: Layout,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub html_content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub markdown_content: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Query {
        pub query: Option<String>,
        pub vrl_function_query: Option<String>,
        pub custom_query: bool,
        pub fields: PanelFields,
        pub config: QueryConfig,
    }

    #[derive(Deserialize)]
    pub struct PanelFields {
        pub stream: String,
        pub stream_type: StreamType,
        pub x: Vec<AxisItem>,
        pub y: Vec<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub z: Option<Vec<AxisItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub breakdown: Option<Vec<AxisItem>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub longitude: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub weight: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value_for_maps: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub target: Option<AxisItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<AxisItem>,
        pub filter: PanelFilter,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AxisItem {
        pub label: String,
        pub alias: String,
        pub column: String,
        pub color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aggregation_function: Option<AggregationFunc>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sort_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub args: Option<Vec<AxisArg>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub is_derived: Option<bool>,
    }

    #[derive(Deserialize)]
    pub struct AxisArg {
        value: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum AggregationFunc {
        Count,
        #[serde(rename = "count-distinct")]
        CountDistinct,
        Histogram,
        Sum,
        Min,
        Max,
        Avg,
        Median,
        P50,
        P90,
        P95,
        P99,
    }

    #[derive(Deserialize)]
    #[serde(untagged, rename_all = "camelCase")]
    pub enum PanelFilter {
        #[serde(rename = "condition")]
        Condition(FilterCondition),
        #[serde(rename = "group")]
        Group(GroupType),
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GroupType {
        pub filter_type: String,
        pub logical_operator: String,
        pub conditions: Vec<PanelFilter>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FilterCondition {
        #[serde(rename = "type")]
        pub typ: String,
        pub values: Vec<String>,
        pub column: String,
        pub operator: Option<String>,
        pub value: Option<String>,
        pub logical_operator: String,
        pub filter_type: String,
    }

    #[derive(Deserialize)]
    pub struct PanelConfig {
        show_legends: bool,
        legends_position: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_custom: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        decimals: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        line_thickness: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_results: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        top_results_others: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_width: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        axis_border_show: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        label_option: Option<LabelOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        show_symbol: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        line_interpolation: Option<LineInterpolation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        legend_width: Option<LegendWidth>,
        base_map: Option<BaseMap>,
        #[serde(skip_serializing_if = "Option::is_none")]
        map_type: Option<MapType>,
        map_view: Option<MapView>,
        #[serde(skip_serializing_if = "Option::is_none")]
        map_symbol_style: Option<MapSymbolStyle>,
        #[serde(skip_serializing_if = "Option::is_none")]
        drilldown: Option<Vec<DrillDown>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mark_line: Option<Vec<MarkLine>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        override_config: Option<Vec<OverrideConfig>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        connect_nulls: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        no_value_replacement: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        wrap_table_cells: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        table_transpose: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        table_dynamic_columns: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mappings: Option<Vec<Mapping>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<ColorCfg>,
        #[serde(skip_serializing_if = "Option::is_none")]
        trellis: Option<Trellis>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ColorCfg {
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fixed_color: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        series_by: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Mapping {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        typee: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "match")]
        matchh: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DrillDown {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        type_field: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_blank: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        find_by: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<DrillDownData>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MarkLine {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "type")]
        typee: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OverrideConfig {
        #[serde(skip_serializing_if = "Option::is_none")]
        field: Option<Field>,
        #[serde(skip_serializing_if = "Option::is_none")]
        config: Option<Vec<Config>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Field {
        match_by: String,
        value: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Config {
        #[serde(rename = "type")]
        typee: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<Value>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Value {
        unit: String,
        custom_unit: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DrillDownData {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dashboard: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pass_all_variables: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        variables: Option<Vec<DrillDownVariables>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logs_mode: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logs_query: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct DrillDownVariables {
        name: Option<String>,
        value: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct QueryConfig {
        promql_legend: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        layer_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        weight_fixed: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        limit: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time_shift: Option<Vec<TimeShift>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TimeShift {
        #[serde(skip_serializing_if = "Option::is_none")]
        off_set: Option<String>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Variables {
        pub list: Vec<VariableList>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub show_dynamic_filters: Option<bool>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DateTimeOptions {
        #[serde(rename = "type")]
        pub typee: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub relative_time_period: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub start_time: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub end_time: Option<i64>,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VariableList {
        #[serde(rename = "type")]
        pub type_field: String,
        pub name: String,
        pub label: String,
        #[serde(rename = "query_data")]
        pub query_data: Option<QueryData>,
        pub value: Option<String>,
        pub options: Option<Vec<CustomFieldsOption>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub multi_select: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub hide_on_dashboard: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub select_all_value_for_multi_select: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub custom_multi_select_value: Option<Vec<String>>,
    }

    #[derive(Default, Deserialize)]
    pub struct QueryData {
        pub stream_type: StreamType,
        pub stream: String,
        pub field: String,
        pub max_record_size: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub filter: Option<Vec<Filters>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Filters {
        pub name: Option<String>,
        pub operator: Option<String>,
        pub value: String,
    }

    #[derive(Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CustomFieldsOption {
        pub label: String,
        pub value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub selected: Option<bool>,
    }

    #[derive(Deserialize)]
    pub struct BaseMap {
        #[serde(rename = "type")]
        pub type_field: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct MapType {
        #[serde(rename = "type")]
        pub type_field: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct MapView {
        pub zoom: f64,
        pub lat: f64,
        pub lng: f64,
    }

    #[derive(Deserialize)]
    pub struct Trellis {
        pub layout: Option<String>,
        pub num_of_columns: i64,
    }

    #[derive(Deserialize)]
    pub struct MapSymbolStyle {
        pub size: String,
        pub size_by_value: Option<SizeByValue>,
        pub size_fixed: f64,
    }

    #[derive(Deserialize)]
    pub struct SizeByValue {
        pub min: f64,
        pub max: f64,
    }

    #[derive(Deserialize)]
    pub struct LegendWidth {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub unit: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct LabelOption {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub position: Option<LabelPosition>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub rotate: Option<f64>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum LineInterpolation {
        Smooth,
        Linear,
        #[serde(rename = "step-start")]
        StepStart,
        #[serde(rename = "step-end")]
        StepEnd,
        #[serde(rename = "step-middle")]
        StepMiddle,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum LabelPosition {
        Top,
        Inside,
        InsideTop,
        InsideBottom,
    }
}

mod stream {
    use serde::Deserialize;

    #[derive(Clone, Debug, Default, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum StreamType {
        #[default]
        Logs,
        Metrics,
        Traces,
        #[serde(rename = "enrichment_tables")]
        EnrichmentTables,
        #[serde(rename = "file_list")]
        Filelist,
        Metadata,
        Index,
    }
}

fn datetime_now() -> DateTime<FixedOffset> {
    chrono::Utc::now().with_timezone(&FixedOffset::east_opt(0).expect(
        "BUG", // This can't possibly fail. Can it?
    ))
}
