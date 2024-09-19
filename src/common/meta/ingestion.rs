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

use std::{
    collections::HashMap,
    io::{BufReader, Lines},
};

use actix_web::web;
use config::utils::json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::stream::SchemaRecords;

// Columns added to ingested records for _INTERNAL_ use only.
// Used for storing and querying unflattened original data
pub const ORIGINAL_DATA_COL_NAME: &str = "_original";
pub const ID_COL_NAME: &str = "_o2_id";

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct RecordStatus {
    pub successful: u32,
    pub failed: u32,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
}

pub struct BulkStreamData {
    pub data: HashMap<String, SchemaRecords>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct StreamStatus {
    pub name: String,
    #[serde(flatten)]
    pub status: RecordStatus,
}

impl StreamStatus {
    pub fn new(name: &str) -> Self {
        StreamStatus {
            name: name.to_string(),
            status: RecordStatus::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct IngestionResponse {
    pub code: u16,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub status: Vec<StreamStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl IngestionResponse {
    pub fn new(code: u16, status: Vec<StreamStatus>) -> Self {
        IngestionResponse {
            code,
            status,
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamSchemaChk {
    pub conforms: bool,
    pub has_fields: bool,
    pub has_partition_keys: bool,
    pub has_metadata: bool,
}

pub const INGESTION_EP: [&str; 14] = [
    "_bulk",
    "_json",
    "_multi",
    "traces",
    "write",
    "_kinesis_firehose",
    "_license",
    "_xpack",
    "_index_template",
    "_data_stream",
    "_sub",
    "logs",
    "metrics",
    "_json_arrow",
];

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkResponse {
    pub took: u128,
    pub errors: bool,
    pub items: Vec<HashMap<String, BulkResponseItem>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkResponseItem {
    pub _index: String,
    pub _id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _shards: Option<ShardResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _seq_no: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _primary_term: Option<i64>,
    pub status: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BulkResponseError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "originalRecord")]
    #[schema(value_type = Object)]
    pub original_record: Option<json::Value>,
}

pub enum IngestionStatus {
    Record(RecordStatus),
    Bulk(BulkResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ShardResponse {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkResponseError {
    #[serde(rename = "type")]
    pub err_type: String,
    pub reason: String,
    pub index_uuid: String,
    pub shard: String,
    pub index: String,
}

impl BulkResponseError {
    pub fn new(err_type: String, index: String, reason: String, index_uuid: String) -> Self {
        BulkResponseError {
            err_type,
            reason,
            index_uuid,
            shard: "1".to_owned(),
            index,
        }
    }
}

impl BulkResponseItem {
    pub fn new_failed(
        _index: String,
        _id: String,
        error: BulkResponseError,
        orig_record: Option<json::Value>,
        stream_name: String,
    ) -> Self {
        BulkResponseItem {
            _index: stream_name,
            _id,
            _version: None,
            result: None,
            _shards: None,
            _seq_no: None,
            _primary_term: None,
            status: 422,
            error: Some(error),
            original_record: orig_record,
        }
    }

    pub fn new(
        _index: String,
        _id: String,
        _orig_record: Option<json::Value>,
        stream_name: String,
    ) -> Self {
        BulkResponseItem {
            _index: stream_name,
            _id,
            _version: Some(1),
            result: Some("created".to_owned()),
            _shards: Some(ShardResponse {
                total: 1,
                successful: 1,
                failed: 0,
            }),
            _seq_no: Some(1),
            _primary_term: Some(1),
            status: 200,
            error: None,
            original_record: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KinesisFHRequest {
    pub records: Vec<KFHRecordRequest>,
    pub request_id: String,
    pub timestamp: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct KFHRecordRequest {
    pub data: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KinesisFHLogData {
    pub log_events: Vec<KinesisFHLogEvent>,
    pub log_group: String,
    pub log_stream: String,
    pub message_type: String,
    pub owner: String,
    pub subscription_filters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KinesisFHLogEvent {
    pub message: json::Value,
    pub id: String,
    pub timestamp: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KinesisFHMetricData {
    #[serde(rename = "metric_stream_name")]
    pub metric_stream_name: String,
    #[serde(rename = "account_id")]
    pub account_id: String,
    pub region: String,
    pub namespace: String,
    #[serde(rename = "metric_name")]
    pub metric_name: String,
    pub dimensions: json::Value,
    pub timestamp: i64,
    pub value: KinesisFHMetricValue,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KinesisFHMetricValue {
    pub count: f32,
    pub sum: f32,
    pub max: f32,
    pub min: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KinesisFHIngestionResponse {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum AWSRecordType {
    KinesisFHLogs(KinesisFHLogData),
    KinesisFHMetrics(KinesisFHMetricData),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCPIngestionRequest {
    pub message: GCPMessage,
    pub subscription: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCPMessage {
    pub attributes: GCPAttributes,
    pub data: String,
    pub message_id: String,
    #[serde(rename = "message_id")]
    pub message_id_dup: String,
    pub publish_time: String,
    #[serde(rename = "publish_time")]
    pub publish_time_dup: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCPAttributes {
    #[serde(rename = "logging.googleapis.com/timestamp")]
    pub logging_googleapis_com_timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GCPIngestionResponse {
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub timestamp: String,
}

pub enum IngestionRequest<'a> {
    JSON(&'a web::Bytes),
    Multi(&'a web::Bytes),
    GCP(&'a GCPIngestionRequest),
    KinesisFH(&'a KinesisFHRequest),
    RUM(&'a web::Bytes),
    Usage(&'a web::Bytes),
}

pub enum IngestionData<'a> {
    JSON(&'a Vec<json::Value>),
    Multi(&'a [u8]),
    GCP(&'a GCPIngestionRequest),
    KinesisFH(&'a KinesisFHRequest),
}

#[derive(Debug)]
pub enum IngestionError {
    IoError(std::io::Error),
    JsonError(json::Error),
    AWSError(KinesisFHIngestionResponse),
    GCPError(GCPIngestionResponse),
}

impl From<json::Error> for IngestionError {
    fn from(err: json::Error) -> Self {
        IngestionError::JsonError(err)
    }
}

impl From<std::io::Error> for IngestionError {
    fn from(err: std::io::Error) -> Self {
        IngestionError::IoError(err)
    }
}

pub enum IngestionDataIter<'a> {
    JSONIter(std::slice::Iter<'a, json::Value>),
    MultiIter(Lines<BufReader<&'a [u8]>>),
    GCP(
        std::vec::IntoIter<json::Value>,
        Option<GCPIngestionResponse>,
    ),
    KinesisFH(
        std::vec::IntoIter<json::Value>,
        Option<KinesisFHIngestionResponse>,
    ),
}
