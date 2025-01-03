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

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use vrl::{
    compiler::{CompileConfig, Program, TimeZone, VrlRuntime},
    prelude::Function,
};

use crate::{meta::stream::StreamType, utils::json};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub function: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub params: String,
    #[serde(default)]
    pub num_args: u8,
    #[serde(default = "default_trans_type")]
    pub trans_type: Option<u8>, // 0=vrl 1=lua
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streams: Option<Vec<StreamOrder>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TestVRLRequest {
    pub function: String,         // VRL function as a string
    pub events: Vec<json::Value>, // List of events (JSON objects)
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TestVRLResponse {
    pub results: Vec<VRLResult>, // Transformed events
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct VRLResult {
    pub message: String,
    pub event: json::Value,
}

impl VRLResult {
    pub fn new(message: &str, event: json::Value) -> Self {
        Self {
            message: message.to_string(),
            event,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamOrder {
    #[serde(default)]
    pub stream: String,
    #[serde(default)]
    pub order: u8,
    #[serde(default)]
    pub stream_type: StreamType,
    #[serde(default)]
    pub is_removed: bool,
    #[serde(default)]
    pub apply_before_flattening: bool,
}

impl PartialEq for Transform {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.function == other.function && self.params == other.params
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ZoFunction<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct FunctionList {
    pub list: Vec<Transform>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VRLConfig {
    pub runtime: VrlRuntime,
    pub timezone: TimeZone,
}

fn default_trans_type() -> Option<u8> {
    Some(0)
}

pub struct VRLCompilerConfig {
    pub config: CompileConfig,
    pub functions: Vec<Box<dyn Function>>,
}

pub struct VRLRuntimeConfig {
    pub config: CompileConfig,
    pub program: Program,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct VRLResultResolver {
    pub program: Program,
    pub fields: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::json;

    #[test]
    fn test_functions() {
        let trans = Transform {
            function: "function jsconcat(a,b){return a+b}".to_string(),
            name: "jsconcat".to_string(),
            trans_type: Some(1),
            params: "row".to_string(),
            num_args: 1,
            streams: Some(vec![StreamOrder {
                stream: "test".to_string(),
                order: 1,
                stream_type: StreamType::Logs,
                is_removed: false,
                apply_before_flattening: false,
            }]),
        };

        let mod_trans = Transform {
            function: "function jsconcat(a,b){return a+b}".to_string(),
            name: "jsconcat".to_string(),
            trans_type: Some(1),
            params: "row".to_string(),
            num_args: 1,
            streams: None,
        };
        assert_eq!(trans, mod_trans);

        let trans_str = json::to_string(&trans).unwrap();
        let trans2: Transform = json::from_str(&trans_str).unwrap();
        assert_eq!(format!("{:?}", trans), format!("{:?}", trans2));

        let trans_list = FunctionList {
            list: vec![trans, trans2],
        };
        assert!(!trans_list.list.is_empty());
        let trans_list_str = json::to_string(&trans_list.clone()).unwrap();
        let trans_list2: FunctionList = json::from_str(&trans_list_str).unwrap();
        assert_eq!(trans_list.list.len(), trans_list2.list.len());
    }

    #[test]
    fn test_zo_function() {
        let f1 = ZoFunction {
            name: "test",
            text: "test",
        };
        let f1_str = json::to_string(&f1).unwrap();
        let f2: ZoFunction = json::from_str(&f1_str).unwrap();
        assert_eq!(f1.name, f2.name);
        assert_eq!(format!("{:?}", f1), format!("{:?}", f2));
    }
}
