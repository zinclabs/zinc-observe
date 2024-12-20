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

use config::meta::stream::StreamType;
use proto::cluster_rpc::{self, IndexInfo, QueryIdentifier, SearchInfo, SuperClusterInfo};

#[derive(Debug, Clone)]
pub struct Request {
    pub trace_id: String,
    pub org_id: String,
    pub stream_type: StreamType,
    pub timeout: i64,
    pub user_id: Option<String>,
    pub work_group: Option<String>,
    pub time_range: Option<(i64, i64)>,
    pub search_event_type: Option<String>, // node rule
    pub use_inverted_index: bool,
    pub streaming_output: bool,
    pub streaming_id: Option<String>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            trace_id: "".to_string(),
            org_id: "".to_string(),
            stream_type: StreamType::default(),
            timeout: 0,
            user_id: None,
            work_group: None,
            time_range: None,
            search_event_type: None,
            use_inverted_index: false,
            streaming_output: false,
            streaming_id: None,
        }
    }
}

impl Request {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trace_id: String,
        org_id: String,
        stream_type: StreamType,
        timeout: i64,
        user_id: Option<String>,
        time_range: Option<(i64, i64)>,
        search_event_type: Option<String>,
    ) -> Self {
        Self {
            trace_id,
            org_id,
            stream_type,
            timeout,
            user_id,
            work_group: None,
            time_range,
            search_event_type,
            use_inverted_index: false,
            streaming_output: false,
            streaming_id: None,
        }
    }

    pub fn add_user_id(&mut self, user_id: Option<String>) {
        self.user_id = user_id;
    }

    pub fn add_work_group(&mut self, work_group: Option<String>) {
        self.work_group = work_group;
    }

    pub fn add_time_range(&mut self, time_range: Option<(i64, i64)>) {
        self.time_range = time_range;
    }

    pub fn add_search_event_type(&mut self, search_event_type: Option<String>) {
        self.search_event_type = search_event_type;
    }

    pub fn set_use_inverted_index(&mut self, use_inverted_index: bool) {
        self.use_inverted_index = use_inverted_index;
    }

    pub fn set_streaming_output(&mut self, streaming_output: bool, streaming_id: Option<String>) {
        self.streaming_output = streaming_output;
        self.streaming_id = streaming_id;
    }
}

impl From<FlightSearchRequest> for Request {
    fn from(req: FlightSearchRequest) -> Self {
        Self {
            trace_id: req.query_identifier.trace_id,
            org_id: req.query_identifier.org_id,
            stream_type: StreamType::from(req.query_identifier.stream_type.as_str()),
            timeout: req.search_info.timeout,
            user_id: req.super_cluster_info.user_id,
            work_group: req.super_cluster_info.work_group,
            time_range: Some((req.search_info.start_time, req.search_info.end_time)),
            search_event_type: req.super_cluster_info.search_event_type,
            use_inverted_index: req.index_info.use_inverted_index,
            streaming_output: false,
            streaming_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlightSearchRequest {
    pub query_identifier: QueryIdentifier,
    pub search_info: SearchInfo,
    pub index_info: IndexInfo,
    pub super_cluster_info: SuperClusterInfo,
}

impl FlightSearchRequest {
    // used in RemoteScanExec
    pub fn set_partition(&mut self, partition: usize) {
        self.query_identifier.partition = partition as u32;
    }

    // used in RemoteScanExec
    pub fn set_job_id(&mut self, job_id: String) {
        self.query_identifier.job_id = job_id;
    }
}

impl From<cluster_rpc::FlightSearchRequest> for FlightSearchRequest {
    fn from(request: cluster_rpc::FlightSearchRequest) -> Self {
        Self {
            query_identifier: request.query_identifier.unwrap(),
            search_info: request.search_info.unwrap(),
            index_info: request.index_info.unwrap(),
            super_cluster_info: request.super_cluster_info.unwrap(),
        }
    }
}

impl From<FlightSearchRequest> for cluster_rpc::FlightSearchRequest {
    fn from(request: FlightSearchRequest) -> Self {
        Self {
            query_identifier: Some(request.query_identifier),
            search_info: Some(request.search_info),
            index_info: Some(request.index_info),
            super_cluster_info: Some(request.super_cluster_info),
        }
    }
}
