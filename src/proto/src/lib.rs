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

mod datafusion;
mod generated;

pub use generated::{cluster as cluster_rpc, prometheus as prometheus_rpc};

impl From<Vec<serde_json::Value>> for cluster_rpc::UsageData {
    fn from(usages: Vec<serde_json::Value>) -> Self {
        Self {
            data: serde_json::to_vec(&usages).unwrap(),
        }
    }
}
