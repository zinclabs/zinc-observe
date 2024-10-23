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

use serde::{Deserialize, Serialize};

use super::search::SearchEventType;
use crate::{
    meta::stream::{FileMeta, StreamType},
    SIZE_IN_MB,
};

pub const USAGE_STREAM: &str = "usage";
pub const STATS_STREAM: &str = "stats";
pub const TRIGGERS_USAGE_STREAM: &str = "triggers";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TriggerDataStatus {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "condition_not_satisfied")]
    ConditionNotSatisfied,
    #[serde(rename = "skipped")]
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TriggerDataType {
    #[serde(rename = "report")]
    Report,
    #[serde(rename = "cached_report")]
    CachedReport,
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "derived_stream")]
    DerivedStream,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerData {
    pub _timestamp: i64,
    pub org: String,
    pub module: TriggerDataType,
    pub key: String,
    pub next_run_at: i64,
    pub is_realtime: bool,
    pub is_silenced: bool,
    pub status: TriggerDataStatus,
    pub start_time: i64,
    pub end_time: i64,
    pub retries: i32,
    pub error: Option<String>,
    pub success_response: Option<String>,
    pub is_partial: Option<bool>,
    pub delay_in_secs: Option<i64>,
    pub evaluation_took_in_secs: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageData {
    pub _timestamp: i64,
    pub event: UsageEvent,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub event_time_hour: String,
    pub org_id: String,
    pub request_body: String,
    pub size: f64,
    pub unit: String,
    pub user_email: String,
    pub response_time: f64,
    pub stream_type: StreamType,
    pub num_records: i64,
    pub stream_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_ratio: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_type: Option<SearchEventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub took_wait_in_queue: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_cache_ratio: Option<usize>,
    #[serde(default)]
    pub is_partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_group: Option<String>,
}

#[derive(Hash, PartialEq, Eq)]
pub struct GroupKey {
    pub stream_name: String,
    pub org_id: String,
    pub stream_type: StreamType,
    pub day: u32,
    pub hour: u32,
    pub event: UsageEvent,
    pub email: String,
}

pub struct AggregatedData {
    pub usage_data: UsageData,
    pub count: u64,
}

#[derive(Hash, Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum UsageEvent {
    Ingestion,
    Search,
    Functions,
    Other,
}

impl std::fmt::Display for UsageEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            UsageEvent::Ingestion => write!(f, "Ingestion"),
            UsageEvent::Search => write!(f, "Search"),
            UsageEvent::Functions => write!(f, "Functions"),
            UsageEvent::Other => write!(f, "Other"),
        }
    }
}

impl From<UsageType> for UsageEvent {
    fn from(usage: UsageType) -> UsageEvent {
        match usage {
            UsageType::Bulk
            | UsageType::Json
            | UsageType::Multi
            | UsageType::KinesisFirehose
            | UsageType::GCPSubscription
            | UsageType::Logs
            | UsageType::Traces
            | UsageType::Metrics
            | UsageType::PrometheusRemoteWrite
            | UsageType::JsonMetrics
            | UsageType::RUM
            | UsageType::EnrichmentTable
            | UsageType::Syslog => UsageEvent::Ingestion,
            UsageType::Search
            | UsageType::SearchAround
            | UsageType::SearchTopNValues
            | UsageType::MetricSearch
            | UsageType::SearchHistory => UsageEvent::Search,
            UsageType::Functions => UsageEvent::Functions,
            UsageType::Retention => UsageEvent::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum UsageType {
    #[serde(rename = "/logs/_bulk")]
    Bulk,
    #[serde(rename = "/logs/_json")]
    Json,
    #[serde(rename = "/logs/_multi")]
    Multi,
    #[serde(rename = "/_kinesis_firehose")]
    KinesisFirehose,
    #[serde(rename = "/gcp/_sub")]
    GCPSubscription,
    #[serde(rename = "/otlp/v1/logs")]
    Logs,
    #[serde(rename = "/otlp/v1/traces")]
    Traces,
    #[serde(rename = "/otlp/v1/metrics")]
    Metrics,
    #[serde(rename = "/prometheus/v1/write")]
    PrometheusRemoteWrite,
    #[serde(rename = "/metrics/_json")]
    JsonMetrics,
    #[serde(rename = "/v1/rum")]
    RUM,
    #[serde(rename = "/_search")]
    Search,
    #[serde(rename = "/metrics/_search")]
    MetricSearch,
    #[serde(rename = "/_around")]
    SearchAround,
    #[serde(rename = "/_values")]
    SearchTopNValues,
    #[serde(rename = "/_search_history")]
    SearchHistory,
    #[serde(rename = "functions")]
    Functions,
    #[serde(rename = "data_retention")]
    Retention,
    #[serde(rename = "syslog")]
    Syslog,
    #[serde(rename = "enrichment_table")]
    EnrichmentTable,
}

impl std::fmt::Display for UsageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsageType::Bulk => write!(f, "/logs/_bulk"),
            UsageType::Json => write!(f, "/logs/_json"),
            UsageType::Multi => write!(f, "/logs/_multi"),
            UsageType::KinesisFirehose => write!(f, "/_kinesis_firehose"),
            UsageType::GCPSubscription => write!(f, "/gcp/_sub"),
            UsageType::Logs => write!(f, "/otlp/v1/logs"),
            UsageType::Traces => write!(f, "/otlp/v1/traces"),
            UsageType::Metrics => write!(f, "/otlp/v1/metrics"),
            UsageType::PrometheusRemoteWrite => write!(f, "/prometheus/v1/write"),
            UsageType::JsonMetrics => write!(f, "/metrics/_json"),
            UsageType::RUM => write!(f, "/v1/rum"),
            UsageType::Search => write!(f, "/_search"),
            UsageType::MetricSearch => write!(f, "/metrics/_search"),
            UsageType::SearchAround => write!(f, "/_around"),
            UsageType::SearchTopNValues => write!(f, "/_values"),
            UsageType::SearchHistory => write!(f, "/_search_history"),
            UsageType::Functions => write!(f, "functions"),
            UsageType::Retention => write!(f, "data_retention"),
            UsageType::Syslog => write!(f, "syslog"),
            UsageType::EnrichmentTable => write!(f, "enrichment_table"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestStats {
    pub size: f64,
    pub records: i64,
    pub response_time: f64,
    #[serde(default)]
    pub request_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_ratio: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_type: Option<SearchEventType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub took_wait_in_queue: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_cache_ratio: Option<usize>,
    #[serde(default)]
    pub is_partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_group: Option<String>,
}
impl Default for RequestStats {
    fn default() -> Self {
        Self {
            size: 0.0,
            records: 0,
            response_time: 0.0,
            request_body: None,
            function: None,
            cached_ratio: None,
            compressed_size: None,
            min_ts: None,
            max_ts: None,
            user_email: None,
            search_type: None,
            trace_id: None,
            took_wait_in_queue: None,
            result_cache_ratio: None,
            is_partial: false,
            work_group: None,
        }
    }
}

impl From<FileMeta> for RequestStats {
    fn from(meta: FileMeta) -> RequestStats {
        RequestStats {
            size: meta.original_size as f64 / SIZE_IN_MB,
            records: meta.records,
            response_time: 0.0,
            function: None,
            request_body: None,
            cached_ratio: None,
            compressed_size: Some(meta.compressed_size as f64 / SIZE_IN_MB),
            min_ts: Some(meta.min_ts),
            max_ts: Some(meta.max_ts),
            user_email: None,
            search_type: None,
            trace_id: None,
            took_wait_in_queue: None,
            result_cache_ratio: None,
            is_partial: false,
            work_group: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub records: i64,
    pub stream_type: StreamType,
    pub org_id: String,
    pub stream_name: String,
    pub original_size: f64,
    #[serde(default)]
    pub _timestamp: i64,
    #[serde(default)]
    pub min_ts: i64,
    #[serde(default)]
    pub max_ts: i64,
    #[serde(default)]
    pub compressed_size: Option<f64>,
}
