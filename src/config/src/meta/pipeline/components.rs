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

use crate::meta::{
    alerts::{QueryCondition, TriggerCondition},
    function::Transform,
    stream::{RoutingCondition, StreamParams},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "sourceType")]
#[serde(rename_all = "camelCase")]
pub enum PipelineSource {
    Stream(StreamParams),
    Query(QueryInner),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QueryInner {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    id: String,
    source: String,
    target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "nodeType")]
#[serde(rename_all = "camelCase")]
enum NodeData {
    Stream(StreamParams),
    Function(Transform),
    Condition(Vec<RoutingCondition>),
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
          "source": "stream",
          "org_id": "default",
          "stream_name": "default",
          "stream_type": "logs"
        });
        let from_json: PipelineSource = json::from_value(data).unwrap();
        let source = PipelineSource::Stream(StreamParams {
            org_id: "default".into(),
            stream_name: "default".into(),
            stream_type: StreamType::Logs,
        });
        assert_eq!(from_json, source);
    }

    #[test]
    fn test_node_data_serialization() {
        let data = json::json!({
          "type": "stream",
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
}
