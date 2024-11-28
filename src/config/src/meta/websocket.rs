use serde::{Deserialize, Serialize};

use crate::meta::search::Response;
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "error_type", content = "meta", rename_all = "snake_case")]
pub enum ErrorType {
    SearchError { trace_id: String, error: String },
    RequestError { request_id: String, error: String },
}
