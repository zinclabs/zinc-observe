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

use std::collections::HashMap;

use config::utils::json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub flags: u8,
    pub span_status: String,
    pub span_kind: String,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration: u64,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub reference: HashMap<String, String>,
    pub service_name: String,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, json::Value>,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub service: HashMap<String, json::Value>,
    pub events: String,
    pub links: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SpanRefType {
    #[default]
    #[serde(rename = "CHILD_OF")]
    ChildOf,
    #[serde(rename = "PARENT_OF")]
    ParentOf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub _timestamp: u64,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanLinkContext {
    pub trace_id: String,
    pub span_id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_flags: Option<u32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_state: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanLink {
    pub context: SpanLinkContext,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, json::Value>,
    #[serde(default)]
    pub dropped_attributes_count: u32,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct ExportTraceServiceResponse {
    // The details of a partially successful export request.
    //
    // If the request is only partially accepted
    // (i.e. when the server accepts only parts of the data and rejects the rest)
    // the server MUST initialize the `partial_success` field and MUST
    // set the `rejected_<signal>` with the number of items it rejected.
    //
    // Servers MAY also make use of the `partial_success` field to convey
    // warnings/suggestions to senders even when the request was fully accepted.
    // In such cases, the `rejected_<signal>` MUST have a value of `0` and
    // the `error_message` MUST be non-empty.
    //
    // A `partial_success` message with an empty value (rejected_<signal> = 0 and
    // `error_message` = "") is equivalent to it not being set/present. Senders
    // SHOULD interpret it the same way as in the full success case.
    pub partial_success: Option<ExportTracePartialSuccess>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct ExportTracePartialSuccess {
    // The number of rejected spans.
    //
    // A `rejected_<signal>` field holding a `0` value indicates that the
    // request was fully accepted.
    pub rejected_spans: i64,

    // A developer-facing human-readable message in English. It should be used
    // either to explain why the server rejected parts of the data during a partial
    // success or to convey warnings/suggestions during a full success. The message
    // should offer guidance on how users can address such issues.
    //
    // error_message is an optional field. An error_message with an empty value
    // is equivalent to it not being set.
    pub error_message: String,
}
