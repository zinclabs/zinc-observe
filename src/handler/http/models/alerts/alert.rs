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

use chrono::{DateTime, FixedOffset};
use config::meta::alerts as meta_alerts;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

use crate::handler::http::models::{search::SearchEventType, stream::StreamType};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Alert {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub stream_type: StreamType,
    #[serde(default)]
    pub stream_name: String,
    #[serde(default)]
    pub is_real_time: bool,
    #[serde(default)]
    pub query_condition: QueryCondition,
    #[serde(default)]
    pub trigger_condition: TriggerCondition,
    pub destinations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_attributes: Option<HashMap<String, String>>,
    #[serde(default)]
    pub row_template: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    /// Timezone offset in minutes.
    /// The negative secs means the Western Hemisphere
    pub tz_offset: i32,
    #[serde(default)]
    pub last_triggered_at: Option<i64>,
    #[serde(default)]
    pub last_satisfied_at: Option<i64>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: Option<DateTime<FixedOffset>>,
    #[serde(default)]
    pub last_edited_by: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct TriggerCondition {
    pub period: i64, // 10 minutes
    #[serde(default)]
    pub operator: Operator, // >=
    #[serde(default)]
    pub threshold: i64, // 3 times
    #[serde(default)]
    pub frequency: i64, // 1 minute
    #[serde(default)]
    pub cron: String, // Cron Expression
    #[serde(default)]
    pub frequency_type: FrequencyType,
    #[serde(default)]
    pub silence: i64, // silence for 10 minutes after fire an alert
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(default)]
    pub tolerance_in_secs: Option<i64>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct CompareHistoricData {
    #[serde(rename = "offSet")]
    pub offset: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum FrequencyType {
    #[serde(rename = "cron")]
    Cron,
    #[serde(rename = "minutes")]
    #[default]
    Minutes,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct QueryCondition {
    #[serde(default)]
    #[serde(rename = "type")]
    pub query_type: QueryType,
    pub conditions: Option<Vec<Condition>>,
    pub sql: Option<String>,
    pub promql: Option<String>,              // (cpu usage / cpu total)
    pub promql_condition: Option<Condition>, // value >= 80
    pub aggregation: Option<Aggregation>,
    #[serde(default)]
    pub vrl_function: Option<String>,
    #[serde(default)]
    pub search_event_type: Option<SearchEventType>,
    #[serde(default)]
    pub multi_time_range: Option<Vec<CompareHistoricData>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Aggregation {
    pub group_by: Option<Vec<String>>,
    pub function: AggFunction,
    pub having: Condition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum AggFunction {
    #[serde(rename = "avg")]
    Avg,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "median")]
    Median,
    #[serde(rename = "p50")]
    P50,
    #[serde(rename = "p75")]
    P75,
    #[serde(rename = "p90")]
    P90,
    #[serde(rename = "p95")]
    P95,
    #[serde(rename = "p99")]
    P99,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum QueryType {
    #[default]
    #[serde(rename = "custom")]
    Custom,
    #[serde(rename = "sql")]
    SQL,
    #[serde(rename = "promql")]
    PromQL,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Condition {
    pub column: String,
    pub operator: Operator,
    #[schema(value_type = Object)]
    pub value: JsonValue,
    #[serde(default)]
    pub ignore_case: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum Operator {
    #[serde(rename = "=")]
    EqualTo,
    #[serde(rename = "!=")]
    NotEqualTo,
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = ">=")]
    GreaterThanEquals,
    #[serde(rename = "<")]
    LessThan,
    #[serde(rename = "<=")]
    LessThanEquals,
    Contains,
    NotContains,
}

impl Default for Operator {
    fn default() -> Self {
        Self::EqualTo
    }
}

// Translation functions from models in the config::meta module to models the
// http::models module.

impl From<meta_alerts::alert::Alert> for Alert {
    fn from(value: meta_alerts::alert::Alert) -> Self {
        Self {
            name: value.name,
            org_id: value.org_id,
            stream_type: value.stream_type.into(),
            stream_name: value.stream_name,
            is_real_time: value.is_real_time,
            query_condition: value.query_condition.into(),
            trigger_condition: value.trigger_condition.into(),
            destinations: value.destinations,
            context_attributes: value.context_attributes,
            row_template: value.row_template,
            description: value.description,
            enabled: value.enabled,
            tz_offset: value.tz_offset,
            last_triggered_at: value.last_triggered_at,
            last_satisfied_at: value.last_satisfied_at,
            owner: value.owner,
            updated_at: value.updated_at,
            last_edited_by: value.last_edited_by,
        }
    }
}

impl From<meta_alerts::TriggerCondition> for TriggerCondition {
    fn from(value: meta_alerts::TriggerCondition) -> Self {
        Self {
            period: value.period,
            operator: value.operator.into(),
            threshold: value.threshold,
            frequency: value.frequency,
            cron: value.cron,
            frequency_type: value.frequency_type.into(),
            silence: value.silence,
            timezone: value.timezone,
            tolerance_in_secs: value.tolerance_in_secs,
        }
    }
}

impl From<meta_alerts::CompareHistoricData> for CompareHistoricData {
    fn from(value: meta_alerts::CompareHistoricData) -> Self {
        Self {
            offset: value.offset,
        }
    }
}

impl From<meta_alerts::FrequencyType> for FrequencyType {
    fn from(value: meta_alerts::FrequencyType) -> Self {
        match value {
            meta_alerts::FrequencyType::Cron => Self::Cron,
            meta_alerts::FrequencyType::Minutes => Self::Minutes,
        }
    }
}

impl From<meta_alerts::QueryCondition> for QueryCondition {
    fn from(value: meta_alerts::QueryCondition) -> Self {
        Self {
            query_type: value.query_type.into(),
            conditions: value
                .conditions
                .map(|cs| cs.into_iter().map(|c| c.into()).collect()),
            sql: value.sql,
            promql: value.promql,
            promql_condition: value.promql_condition.map(|pc| pc.into()),
            aggregation: value.aggregation.map(|a| a.into()),
            vrl_function: value.vrl_function,
            search_event_type: value.search_event_type.map(|t| t.into()),
            multi_time_range: value
                .multi_time_range
                .map(|cs| cs.into_iter().map(|c| c.into()).collect()),
        }
    }
}

impl From<meta_alerts::Aggregation> for Aggregation {
    fn from(value: meta_alerts::Aggregation) -> Self {
        Self {
            group_by: value.group_by,
            function: value.function.into(),
            having: value.having.into(),
        }
    }
}

impl From<meta_alerts::AggFunction> for AggFunction {
    fn from(value: meta_alerts::AggFunction) -> Self {
        match value {
            meta_alerts::AggFunction::Avg => Self::Avg,
            meta_alerts::AggFunction::Min => Self::Min,
            meta_alerts::AggFunction::Max => Self::Max,
            meta_alerts::AggFunction::Sum => Self::Sum,
            meta_alerts::AggFunction::Count => Self::Count,
            meta_alerts::AggFunction::Median => Self::Median,
            meta_alerts::AggFunction::P50 => Self::P50,
            meta_alerts::AggFunction::P75 => Self::P75,
            meta_alerts::AggFunction::P90 => Self::P90,
            meta_alerts::AggFunction::P95 => Self::P95,
            meta_alerts::AggFunction::P99 => Self::P99,
        }
    }
}

impl From<meta_alerts::QueryType> for QueryType {
    fn from(value: meta_alerts::QueryType) -> Self {
        match value {
            meta_alerts::QueryType::Custom => Self::Custom,
            meta_alerts::QueryType::SQL => Self::SQL,
            meta_alerts::QueryType::PromQL => Self::PromQL,
        }
    }
}

impl From<meta_alerts::Condition> for Condition {
    fn from(value: meta_alerts::Condition) -> Self {
        Self {
            column: value.column,
            operator: value.operator.into(),
            value: value.value,
            ignore_case: value.ignore_case,
        }
    }
}

impl From<meta_alerts::Operator> for Operator {
    fn from(value: meta_alerts::Operator) -> Self {
        match value {
            meta_alerts::Operator::EqualTo => Self::EqualTo,
            meta_alerts::Operator::NotEqualTo => Self::NotEqualTo,
            meta_alerts::Operator::GreaterThan => Self::GreaterThan,
            meta_alerts::Operator::GreaterThanEquals => Self::GreaterThanEquals,
            meta_alerts::Operator::LessThan => Self::LessThan,
            meta_alerts::Operator::LessThanEquals => Self::LessThanEquals,
            meta_alerts::Operator::Contains => Self::Contains,
            meta_alerts::Operator::NotContains => Self::NotContains,
        }
    }
}

// Translation functions from models in the http::models module to models in the
// config::meta module.

impl From<Alert> for meta_alerts::alert::Alert {
    fn from(value: Alert) -> Self {
        Self {
            name: value.name,
            org_id: value.org_id,
            stream_type: value.stream_type.into(),
            stream_name: value.stream_name,
            is_real_time: value.is_real_time,
            query_condition: value.query_condition.into(),
            trigger_condition: value.trigger_condition.into(),
            destinations: value.destinations,
            context_attributes: value.context_attributes,
            row_template: value.row_template,
            description: value.description,
            enabled: value.enabled,
            tz_offset: value.tz_offset,
            last_triggered_at: value.last_triggered_at,
            last_satisfied_at: value.last_satisfied_at,
            owner: value.owner,
            updated_at: value.updated_at,
            last_edited_by: value.last_edited_by,
        }
    }
}

impl From<TriggerCondition> for meta_alerts::TriggerCondition {
    fn from(value: TriggerCondition) -> Self {
        Self {
            period: value.period,
            operator: value.operator.into(),
            threshold: value.threshold,
            frequency: value.frequency,
            cron: value.cron,
            frequency_type: value.frequency_type.into(),
            silence: value.silence,
            timezone: value.timezone,
            tolerance_in_secs: value.tolerance_in_secs,
        }
    }
}

impl From<CompareHistoricData> for meta_alerts::CompareHistoricData {
    fn from(value: CompareHistoricData) -> Self {
        Self {
            offset: value.offset,
        }
    }
}

impl From<FrequencyType> for meta_alerts::FrequencyType {
    fn from(value: FrequencyType) -> Self {
        match value {
            FrequencyType::Cron => Self::Cron,
            FrequencyType::Minutes => Self::Minutes,
        }
    }
}

impl From<QueryCondition> for meta_alerts::QueryCondition {
    fn from(value: QueryCondition) -> Self {
        Self {
            query_type: value.query_type.into(),
            conditions: value
                .conditions
                .map(|cs| cs.into_iter().map(|c| c.into()).collect()),
            sql: value.sql,
            promql: value.promql,
            promql_condition: value.promql_condition.map(|pc| pc.into()),
            aggregation: value.aggregation.map(|a| a.into()),
            vrl_function: value.vrl_function,
            search_event_type: value.search_event_type.map(|t| t.into()),
            multi_time_range: value
                .multi_time_range
                .map(|cs| cs.into_iter().map(|c| c.into()).collect()),
        }
    }
}

impl From<Aggregation> for meta_alerts::Aggregation {
    fn from(value: Aggregation) -> Self {
        Self {
            group_by: value.group_by,
            function: value.function.into(),
            having: value.having.into(),
        }
    }
}

impl From<AggFunction> for meta_alerts::AggFunction {
    fn from(value: AggFunction) -> Self {
        match value {
            AggFunction::Avg => Self::Avg,
            AggFunction::Min => Self::Min,
            AggFunction::Max => Self::Max,
            AggFunction::Sum => Self::Sum,
            AggFunction::Count => Self::Count,
            AggFunction::Median => Self::Median,
            AggFunction::P50 => Self::P50,
            AggFunction::P75 => Self::P75,
            AggFunction::P90 => Self::P90,
            AggFunction::P95 => Self::P95,
            AggFunction::P99 => Self::P99,
        }
    }
}

impl From<QueryType> for meta_alerts::QueryType {
    fn from(value: QueryType) -> Self {
        match value {
            QueryType::Custom => Self::Custom,
            QueryType::SQL => Self::SQL,
            QueryType::PromQL => Self::PromQL,
        }
    }
}

impl From<Condition> for meta_alerts::Condition {
    fn from(value: Condition) -> Self {
        Self {
            column: value.column,
            operator: value.operator.into(),
            value: value.value,
            ignore_case: value.ignore_case,
        }
    }
}

impl From<Operator> for meta_alerts::Operator {
    fn from(value: Operator) -> Self {
        match value {
            Operator::EqualTo => Self::EqualTo,
            Operator::NotEqualTo => Self::NotEqualTo,
            Operator::GreaterThan => Self::GreaterThan,
            Operator::GreaterThanEquals => Self::GreaterThanEquals,
            Operator::LessThan => Self::LessThan,
            Operator::LessThanEquals => Self::LessThanEquals,
            Operator::Contains => Self::Contains,
            Operator::NotContains => Self::NotContains,
        }
    }
}
