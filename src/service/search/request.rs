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
use proto::cluster_rpc::FlightSearchRequest;

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
    pub inverted_index_type: Option<String>,
    pub use_inverted_index: bool,
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
            inverted_index_type: None,
            use_inverted_index: false,
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
        inverted_index_type: Option<String>,
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
            inverted_index_type,
            use_inverted_index: false,
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

    pub fn set_inverted_index_type(&mut self, index_type: Option<String>) {
        self.inverted_index_type = index_type;
    }
}

impl From<FlightSearchRequest> for Request {
    fn from(request: FlightSearchRequest) -> Self {
        Self {
            trace_id: request.trace_id,
            org_id: request.org_id,
            stream_type: StreamType::from(request.stream_type.as_str()),
            timeout: request.timeout,
            user_id: request.user_id,
            work_group: request.work_group,
            time_range: Some((request.start_time, request.end_time)),
            search_event_type: request.search_event_type,
            inverted_index_type: request.index_type,
            use_inverted_index: request.use_inverted_index,
        }
    }
}
