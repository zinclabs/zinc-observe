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

use anyhow::{anyhow, Result};
use components::{DerivedStream, Edge, Node, NodeData, PipelineSource};
use serde::{Deserialize, Serialize};
use sqlx::{Decode, Error, FromRow, Row, Type};
use utoipa::ToSchema;

use crate::{
    meta::stream::{StreamParams, StreamType},
    utils::json,
};

pub mod components;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct Pipeline {
    #[serde(default)]
    #[serde(rename = "pipeline_id")]
    pub id: String,
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub org: String, // org this pipeline belongs to. diff from source stream org_id
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: PipelineSource,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Pipeline {
    pub fn get_cache_key(&self) -> String {
        match &self.source {
            PipelineSource::Realtime(stream_params) => {
                format!("{}/{}", self.org, stream_params)
            }
            PipelineSource::Scheduled(_) => {
                format!("{}/{}", self.org, self.id)
            }
        }
    }

    pub fn get_derived_stream(&self) -> Option<DerivedStream> {
        match &self.source {
            PipelineSource::Scheduled(derived_stream) => Some(derived_stream.to_owned()),
            _ => None,
        }
    }

    /// Verifies the pipeline is valid by:
    /// 1. non-empty nodes list
    /// 2. non-empty edges list
    /// 3. 1st node in nodes list is either [components::NodeData::Stream] or
    ///    [components::NodeData::Query]
    /// 4. non-empty `conditions` in all [components::NodeData::Condition] nodes in nodes list
    /// 5. all leaf nodes are of type [components::NodeData::Stream]
    ///
    /// If all satisfies, populates the [Pipeline::source] with the first node in nodes list
    pub fn validate(&mut self) -> Result<()> {
        // ck 1 & 2
        match (self.nodes.is_empty(), self.edges.is_empty()) {
            (true, true) => return Err(anyhow!("empty nodes and edges list")),
            (true, false) => return Err(anyhow!("empty nodes list")),
            (false, true) => return Err(anyhow!("empty edges list")),
            _ => {}
        };

        // ck 3
        match self.nodes.first().unwrap().get_node_data() {
            NodeData::Stream(stream_params) => {
                self.source = PipelineSource::Realtime(stream_params)
            }
            NodeData::Query(derived_stream) => {
                if derived_stream.trigger_condition.period == 0 {
                    return Err(anyhow!(
                        "DerivedStream source's TriggerCondition period missing or is 0"
                    ));
                }
                self.source = PipelineSource::Scheduled(derived_stream)
            }
            _ => return Err(anyhow!("source must be either a StreamNode or QueryNode")),
        };

        // ck 4
        if self.nodes.iter().any(|node| {
            matches!(node.get_node_data(), NodeData::Condition(condition_params) if condition_params.conditions.is_empty())
        }) {
            return Err(anyhow!("ConditionNode must have non-empty conditions"));
        }

        // ck 5: build adjacency list and check leaf notes
        let node_map = self.get_node_map();
        let adjacency_list = self.build_adjacency_list(&node_map)?;
        // any leaf nodes not a StreamNode
        if self.nodes.iter().any(|node| {
            if !adjacency_list.contains_key(&node.id) {
                // leaf node
                !matches!(node.get_node_data(), NodeData::Stream(_)) // not a StreamNode
            } else {
                false
            }
        }) {
            return Err(anyhow!("All leaf nodes must be StreamNode"));
        }

        Ok(())
    }

    /// Converts pipeline's node list to a map for quick lookup
    pub fn get_node_map(&self) -> HashMap<String, Node> {
        self.nodes
            .iter()
            .map(|node| (node.get_node_id(), node.clone()))
            .collect()
    }

    /// Builds the graph representation of this pipeline's nodes structure.
    ///
    /// Used for pipeline validation and execution.
    pub fn build_adjacency_list(
        &self,
        node_map: &HashMap<String, Node>,
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut adjacency_list = HashMap::new();

        for (idx, edge) in self.edges.iter().enumerate() {
            if !node_map.contains_key(&edge.source) {
                return Err(anyhow!("Edge #{idx}'s source node not found in nodes list"));
            }
            if !node_map.contains_key(&edge.target) {
                return Err(anyhow!("Edge #{idx}'s target node not found in nodes list"));
            }
            adjacency_list
                .entry(edge.source.clone())
                .or_insert_with(Vec::new)
                .push(edge.target.clone());
        }

        Ok(adjacency_list)
    }

    /// Finds all the destination streams in the pipeline.
    pub fn get_all_destination_streams(&self) -> Result<Vec<StreamParams>> {
        let node_map = self.get_node_map();
        let adj_list = self.build_adjacency_list(&node_map)?;

        let streams = node_map
            .iter()
            .filter_map(|(id, node)| {
                if !adj_list.contains_key(id) {
                    if let NodeData::Stream(stream_params) = node.get_node_data() {
                        Some(stream_params)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(streams)
    }

    /// Returns the number of functions nodes in this pipeline.
    ///
    /// Used for usage report.
    pub fn num_of_func(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.is_function_node())
            .count()
    }
}

impl<'r, R: Row> FromRow<'r, R> for Pipeline
where
    &'r ::std::primitive::str: ::sqlx::ColumnIndex<R>,
    String: Type<R::Database> + Decode<'r, R::Database>,
    i32: Type<R::Database> + Decode<'r, R::Database>,
{
    fn from_row(row: &'r R) -> Result<Self, Error> {
        let id: String = row.try_get("id")?;
        let version: i32 = row.try_get("version")?;
        let org: String = row.try_get("org")?;
        let name: String = row.try_get("name")?;
        let description: String = row.try_get("description")?;
        let source_type: String = row.try_get("source_type")?;

        let source = match source_type.as_str() {
            "realtime" => {
                let stream_org: String = row.try_get("stream_org")?;
                let stream_name: String = row.try_get("stream_name")?;
                let stream_type: String = row.try_get("stream_name")?;
                let stream_params = StreamParams::new(
                    &stream_org,
                    &stream_name,
                    StreamType::from(stream_type.as_str()),
                );
                PipelineSource::Realtime(stream_params)
            }
            "scheduled" => {
                let derived_stream_raw: String = row.try_get("derived_stream")?;
                let derived_stream: DerivedStream = json::from_str(&derived_stream_raw)
                    .expect("Deserializing DerivedStream from ROW error");
                PipelineSource::Scheduled(derived_stream)
            }
            _ => return Err(sqlx::Error::ColumnNotFound("Invalid source type".into())),
        };

        let (nodes, edges) = {
            let nodes_raw: String = row.try_get("nodes")?;
            let edges_raw: String = row.try_get("edges")?;
            (
                json::from_str(&nodes_raw).expect("Deserializing Nodes from ROW error"),
                json::from_str(&edges_raw).expect("Deserializing Edges from ROW error"),
            )
        };

        Ok(Pipeline {
            id,
            version,
            org,
            name,
            description,
            source,
            nodes,
            edges,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PipelineList {
    pub list: Vec<Pipeline>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::json;

    #[test]
    fn test_stream_pipeline_serialization() {
        let payload = json::json!(
          {
            "pipeline_id": "uuid",
            "name": "pipeline test",
            "description": "with or without this field",
            "org": "prod",
            "nodes": [
              {
                "id": "1",
                "data": {
                  "node_type": "stream",
                  "org_id": "default",
                  "stream_name": "default",
                  "stream_type": "logs"
                },
                "position": {
                  "x": 100,
                  "y": 100
                },
                "style": {
                  "backgroundColor": "#f0f0f0"
                }
              },
              {
                "id": "2",
                "data": {
                  "node_type": "stream",
                  "org_id": "default",
                  "stream_name": "default",
                  "stream_type": "logs"
                },
                "position": {
                  "x": 300,
                  "y": 100
                }
              }
            ],
            "edges": [
              {
                "id": "e1-2",
                "source": "1",
                "target": "2"
              }
            ]
          }
        );
        let from_value = json::from_value::<Pipeline>(payload);
        assert!(from_value.is_ok());
        let mut pl = from_value.unwrap();
        assert!(pl.build_adjacency_list(&pl.get_node_map()).is_ok());
        assert!(pl.validate().is_ok());
        let nodes = json::to_string(&pl.nodes);
        assert!(nodes.is_ok());
        let nodes = nodes.unwrap();
        let new_nodes = json::from_str::<Option<Vec<Node>>>(&nodes);
        assert!(new_nodes.is_ok());
    }

    #[test]
    fn test_query_pipeline_serialization() {
        let payload = json::json!(
          {
            "pipeline_id": "uuid",
            "name": "pipeline test",
            "description": "with or without this field",
            "org": "prod",
            "nodes": [
              {
                "id": "1",
                "data": {
                    "node_type": "query",
                    "org_id": "default",
                    "stream_type": "logs",
                    "query_condition": {
                        "type": "sql",
                        "conditions": null,
                        "sql": "SELECT min(_timestamp) min_ts,max(_timestamp) max_ts, count(_timestamp) as counts  FROM \"default1\" ",
                        "promql": null,
                        "promql_condition": null,
                        "aggregation": null,
                        "vrl_function": null,
                        "search_event_type": "DerivedStream"
                      },
                      "trigger_condition": {
                        "period": 60,
                        "operator": "=",
                        "threshold": 0,
                        "frequency": 15,
                        "cron": "10 56 * * * *",
                        "frequency_type": "cron",
                        "silence": 0
                      },
                      "tz_offset": 0
                },
                "position": {
                  "x": 100,
                  "y": 100
                },
                "style": {
                  "backgroundColor": "#f0f0f0"
                }
              },
              {
                "id": "2",
                "data": {
                  "node_type": "stream",
                  "org_id": "default",
                  "stream_name": "default",
                  "stream_type": "logs"
                },
                "position": {
                  "x": 300,
                  "y": 100
                }
              }
            ],
            "edges": [
              {
                "id": "e1-2",
                "source": "1",
                "target": "2"
              }
            ]
          }
        );
        let from_value = json::from_value::<Pipeline>(payload);
        assert!(from_value.is_ok());
        let mut pl = from_value.unwrap();
        assert!(pl.build_adjacency_list(&pl.get_node_map()).is_ok());
        assert!(pl.validate().is_ok());
        let nodes = json::to_string(&pl.nodes);
        assert!(nodes.is_ok());
        let nodes = nodes.unwrap();
        let new_nodes = json::from_str::<Option<Vec<Node>>>(&nodes);
        assert!(new_nodes.is_ok());
    }
}
