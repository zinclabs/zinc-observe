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

//! Defines intermediate data types for translating between service layer data
//! structures and the database.

use std::{fmt::Display, str::FromStr};

use config::meta::{
    alerts::{
        AggFunction as MetaAggFunction, Aggregation as MetaAggregation,
        CompareHistoricData as MetaCompareHistoricData, Condition as MetaCondition,
        FrequencyType as MetaFrequencyType, Operator as MetaOperator, QueryType as MetaQueryType,
    },
    search::SearchEventType as MetaSearchEventType,
    stream::StreamType as MetaStreamType,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::errors::{FromI16Error, FromStrError};

/// Query historic data comparison. Stored in the DB as a JSON object.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QueryCompareHistoricData {
    pub offset: String,
}

impl From<MetaCompareHistoricData> for QueryCompareHistoricData {
    fn from(value: MetaCompareHistoricData) -> Self {
        Self {
            offset: value.offset,
        }
    }
}

impl From<QueryCompareHistoricData> for MetaCompareHistoricData {
    fn from(value: QueryCompareHistoricData) -> Self {
        Self {
            offset: value.offset,
        }
    }
}

/// Threshold frequency type. Stored in the DB as a 16-bit integere.
pub enum TriggerFrequencyType {
    Cron,
    Seconds,
}

impl TriggerFrequencyType {
    const CRON: i16 = 0;
    const SECONDS: i16 = 1;
}

impl From<TriggerFrequencyType> for i16 {
    fn from(value: TriggerFrequencyType) -> Self {
        match value {
            TriggerFrequencyType::Cron => TriggerFrequencyType::CRON,
            TriggerFrequencyType::Seconds => TriggerFrequencyType::SECONDS,
        }
    }
}

impl TryFrom<i16> for TriggerFrequencyType {
    type Error = FromI16Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            Self::CRON => Ok(TriggerFrequencyType::Cron),
            Self::SECONDS => Ok(TriggerFrequencyType::Seconds),
            _ => Err(FromI16Error {
                value,
                ty: "FrequencyType".to_owned(),
            }),
        }
    }
}

impl From<MetaFrequencyType> for TriggerFrequencyType {
    fn from(value: MetaFrequencyType) -> Self {
        match value {
            MetaFrequencyType::Cron => TriggerFrequencyType::Cron,
            MetaFrequencyType::Minutes => TriggerFrequencyType::Seconds,
        }
    }
}

impl From<TriggerFrequencyType> for MetaFrequencyType {
    fn from(value: TriggerFrequencyType) -> Self {
        match value {
            TriggerFrequencyType::Cron => MetaFrequencyType::Cron,
            TriggerFrequencyType::Seconds => MetaFrequencyType::Minutes,
        }
    }
}

/// Query aggregation. Stored in the DB as a JSON object.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QueryAggregation {
    pub group_by: Option<Vec<String>>,
    pub function: AggFunction,
    pub having: QueryCondition,
}

impl From<MetaAggregation> for QueryAggregation {
    fn from(value: MetaAggregation) -> Self {
        Self {
            group_by: value.group_by,
            function: value.function.into(),
            having: value.having.into(),
        }
    }
}

impl From<QueryAggregation> for MetaAggregation {
    fn from(value: QueryAggregation) -> Self {
        Self {
            group_by: value.group_by,
            function: value.function.into(),
            having: value.having.into(),
        }
    }
}

/// Query aggregation function. Stored in the DB as a JSON string.
#[derive(Serialize, Deserialize)]
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

impl From<MetaAggFunction> for AggFunction {
    fn from(value: MetaAggFunction) -> Self {
        match value {
            MetaAggFunction::Avg => Self::Avg,
            MetaAggFunction::Min => Self::Min,
            MetaAggFunction::Max => Self::Max,
            MetaAggFunction::Sum => Self::Sum,
            MetaAggFunction::Count => Self::Count,
            MetaAggFunction::Median => Self::Median,
            MetaAggFunction::P50 => Self::P50,
            MetaAggFunction::P75 => Self::P75,
            MetaAggFunction::P90 => Self::P90,
            MetaAggFunction::P95 => Self::P95,
            MetaAggFunction::P99 => Self::P99,
        }
    }
}

impl From<AggFunction> for MetaAggFunction {
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

/// Query type. Stored in the DB as a 16-bit integer
pub enum QueryType {
    Custom,
    Sql,
    Promql,
}

impl QueryType {
    const CUSTOM: i16 = 0;
    const SQL: i16 = 1;
    const PROMQL: i16 = 2;
}

impl From<QueryType> for i16 {
    fn from(value: QueryType) -> Self {
        match value {
            QueryType::Custom => QueryType::CUSTOM,
            QueryType::Sql => QueryType::SQL,
            QueryType::Promql => QueryType::PROMQL,
        }
    }
}

impl TryFrom<i16> for QueryType {
    type Error = FromI16Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            Self::CUSTOM => Ok(QueryType::Custom),
            Self::SQL => Ok(QueryType::Sql),
            Self::PROMQL => Ok(QueryType::Promql),
            _ => Err(FromI16Error {
                value,
                ty: "QueryType".to_string(),
            }),
        }
    }
}

impl From<MetaQueryType> for QueryType {
    fn from(value: MetaQueryType) -> Self {
        match value {
            MetaQueryType::Custom => QueryType::Custom,
            MetaQueryType::SQL => QueryType::Sql,
            MetaQueryType::PromQL => QueryType::Promql,
        }
    }
}

impl From<QueryType> for MetaQueryType {
    fn from(value: QueryType) -> Self {
        match value {
            QueryType::Custom => MetaQueryType::Custom,
            QueryType::Sql => MetaQueryType::SQL,
            QueryType::Promql => MetaQueryType::PromQL,
        }
    }
}

/// Query condition. Stored in the DB as a JSON object.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QueryCondition {
    pub column: String,
    pub operator: ConditionOperator,
    pub value: JsonValue,
    pub ignore_case: bool,
}

impl From<MetaCondition> for QueryCondition {
    fn from(value: MetaCondition) -> Self {
        Self {
            column: value.column,
            operator: value.operator.into(),
            value: value.value,
            ignore_case: value.ignore_case,
        }
    }
}

impl From<QueryCondition> for MetaCondition {
    fn from(value: QueryCondition) -> Self {
        Self {
            column: value.column,
            operator: value.operator.into(),
            value: value.value,
            ignore_case: value.ignore_case,
        }
    }
}

/// Query condition operator. Includes binary predicates that can be applied to
/// untyped values. Stored in the DB as a JSON string.
#[derive(Serialize, Deserialize)]
pub enum ConditionOperator {
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
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "not_contains")]
    NotContains,
}

impl From<MetaOperator> for ConditionOperator {
    fn from(value: MetaOperator) -> Self {
        match value {
            MetaOperator::EqualTo => Self::EqualTo,
            MetaOperator::NotEqualTo => Self::NotEqualTo,
            MetaOperator::GreaterThan => Self::GreaterThan,
            MetaOperator::GreaterThanEquals => Self::GreaterThanEquals,
            MetaOperator::LessThan => Self::LessThan,
            MetaOperator::LessThanEquals => Self::LessThanEquals,
            MetaOperator::Contains => Self::Contains,
            MetaOperator::NotContains => Self::NotContains,
        }
    }
}

impl From<ConditionOperator> for MetaOperator {
    fn from(value: ConditionOperator) -> Self {
        match value {
            ConditionOperator::EqualTo => Self::EqualTo,
            ConditionOperator::NotEqualTo => Self::NotEqualTo,
            ConditionOperator::GreaterThan => Self::GreaterThan,
            ConditionOperator::GreaterThanEquals => Self::GreaterThanEquals,
            ConditionOperator::LessThan => Self::LessThan,
            ConditionOperator::LessThanEquals => Self::LessThanEquals,
            ConditionOperator::Contains => Self::Contains,
            ConditionOperator::NotContains => Self::NotContains,
        }
    }
}

/// Threshold operator. Includes binary predicates for comparison of numeric
/// values. Stored in the DB as a string.
pub enum TriggerThresholdOperator {
    EqualTo,
    NotEqualTo,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
}

impl TriggerThresholdOperator {
    const EQUAL_TO: &'static str = "=";
    const NOT_EQUAL_TO: &'static str = "!=";
    const GREATER_THAN: &'static str = ">";
    const GREATER_THAN_EQUALS: &'static str = ">=";
    const LESS_THAN: &'static str = "<";
    const LESS_THAN_EQUALS: &'static str = "<=";
}

// Implementation for translating from intermediate representation into DB
// representation.
impl Display for TriggerThresholdOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            TriggerThresholdOperator::EqualTo => Self::EQUAL_TO,
            TriggerThresholdOperator::NotEqualTo => Self::NOT_EQUAL_TO,
            TriggerThresholdOperator::GreaterThan => Self::GREATER_THAN,
            TriggerThresholdOperator::GreaterThanEquals => Self::GREATER_THAN_EQUALS,
            TriggerThresholdOperator::LessThan => Self::LESS_THAN,
            TriggerThresholdOperator::LessThanEquals => Self::LESS_THAN_EQUALS,
        };
        write!(f, "{str}")
    }
}

// Implementation for translating from DB representation into intermediate
// representation.
impl FromStr for TriggerThresholdOperator {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::EQUAL_TO => Ok(TriggerThresholdOperator::EqualTo),
            Self::NOT_EQUAL_TO => Ok(TriggerThresholdOperator::NotEqualTo),
            Self::GREATER_THAN => Ok(TriggerThresholdOperator::GreaterThan),
            Self::GREATER_THAN_EQUALS => Ok(TriggerThresholdOperator::GreaterThanEquals),
            Self::LESS_THAN => Ok(TriggerThresholdOperator::LessThan),
            Self::LESS_THAN_EQUALS => Ok(TriggerThresholdOperator::LessThanEquals),
            _ => Err(FromStrError {
                value: s.to_owned(),
                ty: "ThresholdOperator".to_owned(),
            }),
        }
    }
}

impl TryFrom<MetaOperator> for TriggerThresholdOperator {
    type Error = ();

    fn try_from(value: MetaOperator) -> Result<Self, Self::Error> {
        match value {
            MetaOperator::EqualTo => Ok(Self::EqualTo),
            MetaOperator::NotEqualTo => Ok(Self::NotEqualTo),
            MetaOperator::GreaterThan => Ok(Self::GreaterThan),
            MetaOperator::GreaterThanEquals => Ok(Self::GreaterThanEquals),
            MetaOperator::LessThan => Ok(Self::LessThan),
            MetaOperator::LessThanEquals => Ok(Self::LessThanEquals),
            _ => Err(()),
        }
    }
}

impl From<TriggerThresholdOperator> for MetaOperator {
    fn from(value: TriggerThresholdOperator) -> Self {
        match value {
            TriggerThresholdOperator::EqualTo => Self::EqualTo,
            TriggerThresholdOperator::NotEqualTo => Self::NotEqualTo,
            TriggerThresholdOperator::GreaterThan => Self::GreaterThan,
            TriggerThresholdOperator::GreaterThanEquals => Self::GreaterThanEquals,
            TriggerThresholdOperator::LessThan => Self::LessThan,
            TriggerThresholdOperator::LessThanEquals => Self::LessThanEquals,
        }
    }
}

/// Search event type. Stored in the DB as a 16-bit integer.
pub enum QuerySearchEventType {
    Ui,
    Dashboards,
    Reports,
    Alerts,
    Values,
    Other,
    Rum,
    DerivedStream,
}

impl QuerySearchEventType {
    const UI: i16 = 0;
    const DASHBOARDS: i16 = 1;
    const REPORTS: i16 = 2;
    const ALERTS: i16 = 3;
    const VALUES: i16 = 4;
    const OTHER: i16 = 5;
    const RUM: i16 = 6;
    const DERIVED_STREAM: i16 = 7;
}

// Implementation for translating from intermediate representation into DB
// representation.
impl From<QuerySearchEventType> for i16 {
    fn from(value: QuerySearchEventType) -> Self {
        match value {
            QuerySearchEventType::Ui => QuerySearchEventType::UI,
            QuerySearchEventType::Dashboards => QuerySearchEventType::DASHBOARDS,
            QuerySearchEventType::Reports => QuerySearchEventType::REPORTS,
            QuerySearchEventType::Alerts => QuerySearchEventType::ALERTS,
            QuerySearchEventType::Values => QuerySearchEventType::VALUES,
            QuerySearchEventType::Other => QuerySearchEventType::OTHER,
            QuerySearchEventType::Rum => QuerySearchEventType::RUM,
            QuerySearchEventType::DerivedStream => QuerySearchEventType::DERIVED_STREAM,
        }
    }
}

// Implementation for translating from DB representation into intermediate
// representation.
impl TryFrom<i16> for QuerySearchEventType {
    type Error = FromI16Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            Self::UI => Ok(QuerySearchEventType::Ui),
            Self::DASHBOARDS => Ok(QuerySearchEventType::Dashboards),
            Self::REPORTS => Ok(QuerySearchEventType::Reports),
            Self::ALERTS => Ok(QuerySearchEventType::Alerts),
            Self::VALUES => Ok(QuerySearchEventType::Values),
            Self::OTHER => Ok(QuerySearchEventType::Other),
            Self::RUM => Ok(QuerySearchEventType::Rum),
            Self::DERIVED_STREAM => Ok(QuerySearchEventType::DerivedStream),
            _ => Err(FromI16Error {
                value,
                ty: "SeachEventType".to_owned(),
            }),
        }
    }
}

impl From<MetaSearchEventType> for QuerySearchEventType {
    fn from(value: MetaSearchEventType) -> Self {
        match value {
            MetaSearchEventType::UI => Self::Ui,
            MetaSearchEventType::Dashboards => Self::Dashboards,
            MetaSearchEventType::Reports => Self::Reports,
            MetaSearchEventType::Alerts => Self::Alerts,
            MetaSearchEventType::Values => Self::Values,
            MetaSearchEventType::Other => Self::Other,
            MetaSearchEventType::RUM => Self::Rum,
            MetaSearchEventType::DerivedStream => Self::DerivedStream,
        }
    }
}

impl From<QuerySearchEventType> for MetaSearchEventType {
    fn from(value: QuerySearchEventType) -> Self {
        match value {
            QuerySearchEventType::Ui => Self::UI,
            QuerySearchEventType::Dashboards => Self::Dashboards,
            QuerySearchEventType::Reports => Self::Reports,
            QuerySearchEventType::Alerts => Self::Alerts,
            QuerySearchEventType::Values => Self::Values,
            QuerySearchEventType::Other => Self::Other,
            QuerySearchEventType::Rum => Self::RUM,
            QuerySearchEventType::DerivedStream => Self::DerivedStream,
        }
    }
}

/// Stream type. Stored in the DB as a string.
pub enum StreamType {
    Logs,
    Metrics,
    Traces,
    EnrichmentTables,
    FileList,
    Metadata,
    Index,
}

impl StreamType {
    const LOGS: &'static str = "logs";
    const METRICS: &'static str = "metrics";
    const TRACES: &'static str = "traces";
    const ENRICHMENT_TABLES: &'static str = "enrichment_tables";
    const FILE_LIST: &'static str = "file_list";
    const METADATA: &'static str = "metadata";
    const INDEX: &'static str = "index";
}

impl Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            StreamType::Logs => StreamType::LOGS,
            StreamType::Metrics => StreamType::METRICS,
            StreamType::Traces => StreamType::TRACES,
            StreamType::EnrichmentTables => StreamType::ENRICHMENT_TABLES,
            StreamType::FileList => StreamType::FILE_LIST,
            StreamType::Metadata => StreamType::METADATA,
            StreamType::Index => StreamType::INDEX,
        };
        write!(f, "{str}")
    }
}

impl FromStr for StreamType {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::LOGS => Ok(StreamType::Logs),
            Self::METRICS => Ok(StreamType::Metrics),
            Self::TRACES => Ok(StreamType::Traces),
            Self::ENRICHMENT_TABLES => Ok(StreamType::EnrichmentTables),
            Self::FILE_LIST => Ok(StreamType::FileList),
            Self::METADATA => Ok(StreamType::Metadata),
            Self::INDEX => Ok(StreamType::Index),
            _ => Err(FromStrError {
                value: s.to_owned(),
                ty: "StreamType".to_owned(),
            }),
        }
    }
}

impl From<MetaStreamType> for StreamType {
    fn from(value: MetaStreamType) -> Self {
        match value {
            MetaStreamType::Logs => Self::Logs,
            MetaStreamType::Metrics => Self::Metrics,
            MetaStreamType::Traces => Self::Traces,
            MetaStreamType::EnrichmentTables => Self::EnrichmentTables,
            MetaStreamType::Filelist => Self::FileList,
            MetaStreamType::Metadata => Self::Metadata,
            MetaStreamType::Index => Self::Index,
        }
    }
}

impl From<StreamType> for MetaStreamType {
    fn from(value: StreamType) -> Self {
        match value {
            StreamType::Logs => Self::Logs,
            StreamType::Metrics => Self::Metrics,
            StreamType::Traces => Self::Traces,
            StreamType::EnrichmentTables => Self::EnrichmentTables,
            StreamType::FileList => Self::Filelist,
            StreamType::Metadata => Self::Metadata,
            StreamType::Index => Self::Index,
        }
    }
}
