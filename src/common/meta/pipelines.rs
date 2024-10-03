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

use config::{
    meta::{
        alerts::derived_streams::DerivedStreamMeta,
        function::StreamFunctionsList,
        stream::{RoutingCondition, StreamType},
    },
    utils::json::Value,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct PipeLine {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stream_name: String,
    #[serde(default)]
    pub stream_type: StreamType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<HashMap<String, Vec<RoutingCondition>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_streams: Option<Vec<DerivedStreamMeta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl PipeLine {
    pub fn into_response(self, functions: Option<StreamFunctionsList>) -> PipeLineResponse {
        PipeLineResponse {
            name: self.name,
            description: self.description,
            stream_name: self.stream_name,
            stream_type: self.stream_type,
            routing: self.routing,
            derived_streams: self.derived_streams,
            functions,
            meta: self.meta,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct PipeLineResponse {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stream_name: String,
    #[serde(default)]
    pub stream_type: StreamType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<HashMap<String, Vec<RoutingCondition>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<StreamFunctionsList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_streams: Option<Vec<DerivedStreamMeta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipeLineList {
    pub list: Vec<PipeLineResponse>,
}
