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

use config::ider;
use proto::cluster_rpc;

use crate::service::promql;

pub mod auth;
pub mod flight;
pub mod request;

impl From<promql::MetricsQueryRequest> for cluster_rpc::MetricsQueryRequest {
    fn from(req: promql::MetricsQueryRequest) -> Self {
        let req_query = cluster_rpc::MetricsQueryStmt {
            query: req.query.to_owned(),
            start: req.start,
            end: req.end,
            step: req.step,
        };

        let job = cluster_rpc::Job {
            trace_id: ider::uuid(),
            job: "".to_string(),
            stage: 0,
            partition: 0,
        };

        cluster_rpc::MetricsQueryRequest {
            job: Some(job),
            org_id: "".to_string(),
            need_wal: false,
            query: Some(req_query),
            timeout: 0,
        }
    }
}

impl From<&cluster_rpc::Label> for promql::value::Label {
    fn from(req: &cluster_rpc::Label) -> Self {
        promql::value::Label {
            name: req.name.to_owned(),
            value: req.value.to_owned(),
        }
    }
}

impl From<&promql::value::Label> for cluster_rpc::Label {
    fn from(req: &promql::value::Label) -> Self {
        cluster_rpc::Label {
            name: req.name.to_owned(),
            value: req.value.to_owned(),
        }
    }
}

impl From<&cluster_rpc::Sample> for promql::value::Sample {
    fn from(req: &cluster_rpc::Sample) -> Self {
        promql::value::Sample::new(req.time, req.value)
    }
}

impl From<&promql::value::Sample> for cluster_rpc::Sample {
    fn from(req: &promql::value::Sample) -> Self {
        cluster_rpc::Sample {
            time: req.timestamp,
            value: req.value,
        }
    }
}
