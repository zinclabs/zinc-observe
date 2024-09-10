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

use components::{DerivedStream, Edge, Node, PipelineSource};
use serde::{Deserialize, Serialize};
use sqlx::{Decode, Error, FromRow, Row, Type};
use utoipa::ToSchema;

use crate::{
    meta::stream::{StreamParams, StreamType},
    utils::json,
};

pub mod components;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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
    pub source: PipelineSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<Node>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<Edge>>,
}

impl Pipeline {
    pub fn get_cache_key(&self) -> String {
        match &self.source {
            PipelineSource::Stream(stream_params) => {
                format!("{}/{}", self.org, stream_params)
            }
            PipelineSource::Query(_) => {
                format!("{}/{}", self.org, self.id)
            }
        }
    }

    pub fn get_derived_stream(&self) -> Option<DerivedStream> {
        match &self.source {
            PipelineSource::Query(derived_stream) => Some(derived_stream.to_owned()),
            PipelineSource::Stream(_) => None,
        }
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
            "stream" => {
                let stream_org: String = row.try_get("stream_org")?;
                let stream_name: String = row.try_get("stream_name")?;
                let stream_type: String = row.try_get("stream_name")?;
                let stream_params = StreamParams::new(
                    &stream_org,
                    &stream_name,
                    StreamType::from(stream_type.as_str()),
                );
                PipelineSource::Stream(stream_params)
            }
            "derived_stream" => {
                let derived_stream_raw: String = row.try_get("derived_stream")?;
                let derived_stream: DerivedStream = json::from_str(&derived_stream_raw)
                    .expect("Deserializing DerivedStream from ROW error");
                PipelineSource::Query(derived_stream)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::json;

    #[test]
    fn test_stream_pipeline_serialization() {
        let payload = json::json!(
            {
                "name": "pipeline test",
                "description": "with or without this field",
                "org": "default",
                "source": {
                  "source_type": "stream",
                  "org_id": "default",
                  "stream_name": "default",
                  "stream_type": "logs"
                },
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
    }

    #[test]
    fn test_query_pipeline_serialization() {
        let payload = json::json!(
          {
            "pipeline_id": "uuid",
            "name": "pipeline test",
            "description": "with or without this field",
            "org": "prod",
            "source": {
              "source_type": "query",
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
        let pl = from_value.unwrap();
        let nodes = json::to_string(&pl.nodes);
        assert!(nodes.is_ok());
        let nodes = nodes.unwrap();
        let new_nodes = json::from_str::<Option<Vec<Node>>>(&nodes);
        assert!(new_nodes.is_ok());
    }

    #[test]
    fn test_stream_pipeline_empty_serialization() {
        let payload = json::json!(
          {
            "name": "pipeline test",
            "description": "with or without this field",
            "source": {
              "source_type": "stream",
              "org_id": "default",
              "stream_name": "default",
              "stream_type": "logs"
            },
          }
        );
        let from_value = json::from_value::<Pipeline>(payload);
        assert!(from_value.is_ok());
        let pl = from_value.unwrap();
        let nodes = json::to_string(&pl.nodes).unwrap();
        let new_nodes = json::from_str::<Option<Vec<Node>>>(&nodes);
        assert!(new_nodes.is_ok());
    }
}
