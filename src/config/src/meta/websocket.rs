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

use crate::meta::search::{Response, SearchEventContext};

pub const MAX_QUERY_RANGE_LIMIT_ERROR_MESSAGE: &str = "Reached Max query range limit.";

pub enum SearchResultType {
    Cached(Response),
    Search(Response),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchEventReq {
    pub trace_id: String,
    pub payload: crate::meta::search::Request,
    pub time_offset: Option<i64>,
    pub stream_type: crate::meta::stream::StreamType,
    pub use_cache: bool,
    pub search_type: crate::meta::search::SearchEventType,
    #[serde(flatten)]
    pub search_event_context: Option<SearchEventContext>,
}
