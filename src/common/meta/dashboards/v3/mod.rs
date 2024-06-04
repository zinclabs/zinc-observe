// Copyright 2024 Zinc Labs Inc.
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

use chrono::{DateTime, FixedOffset};
use config::meta::stream::StreamType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::datetime_now;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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
    #[schema(value_type = String, format = DateTime)]
    pub created: DateTime<FixedOffset>,
    #[serde(default)]
    pub tabs: Vec<Tab>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Variables>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_datetime_duration: Option<DateTimeOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub i: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub tab_id: String,
    pub name: String,
    #[serde(default)]
    pub panels: Vec<Panel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub query: Option<String>,
    pub custom_query: bool,
    pub fields: PanelFields,
    pub config: QueryConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AxisArg {
    value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
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
    P50,
    P90,
    P95,
    P99,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PanelFilter {
    #[serde(rename = "type")]
    pub typ: String,
    pub values: Vec<String>,
    pub column: String,
    pub operator: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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
    wrap_table_cells: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DrillDownVariables {
    name: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variables {
    pub list: Vec<VariableList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_dynamic_filters: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryData {
    pub stream_type: StreamType,
    pub stream: String,
    pub field: String,
    pub max_record_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<Filters>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Filters {
    pub name: Option<String>,
    pub operator: Option<String>,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldsOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BaseMap {
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MapView {
    pub zoom: f64,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MapSymbolStyle {
    pub size: String,
    pub size_by_value: Option<SizeByValue>,
    pub size_fixed: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SizeByValue {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct LegendWidth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}
