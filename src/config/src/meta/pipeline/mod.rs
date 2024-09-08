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

use components::{Edge, Node, PipelineSource};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod components;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub pipeline_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub source: PipelineSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<Node>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edges: Option<Vec<Edge>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::json;

    #[test]
    fn test_pipeline_serialization() {
        let payload = json::json!(
            {
                "pipelineId": "uuid",
                "name": "pipeline test",
                "description": "with or without this field",
                "source": {
                  "sourceType": "stream",
                  "org_id": "default",
                  "stream_name": "default",
                  "stream_type": "logs"
                },
                "nodes": [
                  {
                    "id": "1",
                    "data": {
                      "nodeType": "stream",
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
                      "nodeType": "stream",
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
}
