use serde::{Deserialize, Serialize};

use crate::meta::search::{Response, SearchEventContext};
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
