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
use utoipa::ToSchema;

use crate::meta::{
    alerts::{QueryCondition, TriggerCondition},
    stream::{RoutingCondition, StreamParams, StreamType},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "source_type")]
#[serde(rename_all = "snake_case")]
pub enum PipelineSource {
    Stream(StreamParams),
    Query(DerivedStream),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DerivedStream {
    #[serde(default)]
    pub org_id: String,
    pub stream_type: StreamType,
    #[serde(default)]
    pub query_condition: QueryCondition,
    #[serde(default)]
    pub trigger_condition: TriggerCondition, // Frequency type only supports minutes
    /// Timezone offset in minutes.
    /// The negative secs means the Western Hemisphere
    #[serde(default)]
    pub tz_offset: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    id: String,
    data: NodeData,
    position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<NodeStyle>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.data == other.data
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    id: String,
    source: String,
    target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "node_type")]
#[serde(rename_all = "snake_case")]
enum NodeData {
    Stream(StreamParams),
    Function(FunctionParams),
    Condition(ConditionParams),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct FunctionParams {
    #[serde(default)]
    pub name: String,
    pub vrl_script: String,
    #[serde(default)]
    pub after_flatten: bool,
    #[serde(default)]
    pub params: String,
    #[serde(default)]
    pub num_args: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ConditionParams {
    conditions: Vec<RoutingCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeStyle {
    background_color: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{meta::stream::StreamType, utils::json};

    #[test]
    fn test_pipeline_source_serialization() {
        let data = json::json!({
          "source_type": "stream",
          "org_id": "default",
          "stream_name": "default",
          "stream_type": "logs"
        });
        let from_json: PipelineSource = json::from_value(data).unwrap();
        let stream_params = StreamParams {
            org_id: "default".into(),
            stream_name: "default".into(),
            stream_type: StreamType::Logs,
        };
        let source = PipelineSource::Stream(stream_params);
        assert_eq!(from_json, source);
    }

    #[test]
    fn test_node_data_serialization() {
        let data = json::json!({
          "node_type": "stream",
          "org_id": "default",
          "stream_name": "default",
          "stream_type": "logs"
        });
        let node: NodeData = json::from_value(data).unwrap();
        let node_data = NodeData::Stream(StreamParams {
            org_id: "default".into(),
            stream_name: "default".into(),
            stream_type: StreamType::Logs,
        });
        assert_eq!(node_data, node);
    }

    #[test]
    fn test_function_node_serialization() {
        let func = FunctionParams {
            vrl_script: "vrl_script".to_string(),
            name: "func".to_string(),
            after_flatten: false,
            params: "row".to_string(),
            num_args: 0,
        };
        let func_node = NodeData::Function(func);
        let payload = json::json!({
            "node_type": "function",
            "name": "func",
            "after_flatten": false,
            "params": "row",
            "vrl_script": "vrl_script",
        });
        let node_data: NodeData = json::from_value(payload).unwrap();
        assert_eq!(func_node, node_data);
    }

    #[test]
    fn test_condition_node_serialization() {
        let payload = json::json!({
            "node_type": "condition",  // required
            "conditions": [            // required
              {
                "column": "body",
                "operator": ">=",
                "value": {
                    "key": "value"
                },
                "ignore_case": false    // optional
              }
            ]
        });

        let node_data = json::from_value::<NodeData>(payload);
        assert!(node_data.is_ok());
    }
}
