use serde::{Deserialize, Serialize};

pub mod sessions_cache_utils {
    use crate::common::infra::config::WS_SESSIONS;

    /// Insert a new session into the cache
    pub fn insert_session(session_id: &str, session: actix_ws::Session) {
        WS_SESSIONS.insert(session_id.to_string(), session);
    }

    /// Remove a session from the cache
    pub fn remove_session(session_id: &str) {
        WS_SESSIONS.remove(session_id);
    }

    /// Get a session from the cache
    pub fn get_session(session_id: &str) -> Option<actix_ws::Session> {
        WS_SESSIONS.get(session_id).map(|entry| entry.clone())
    }

    /// Check if a session exists in the cache
    pub fn contains_session(session_id: &str) -> bool {
        WS_SESSIONS.contains_key(session_id)
    }

    /// Get the number of sessions in the cache
    pub fn len_sessions() -> usize {
        WS_SESSIONS.len()
    }
}

/// Represents the different types of WebSocket client messages that can be sent.
///
/// The `WSClientMessage` enum is used to represent the different types of messages that can be sent
/// from a WebSocket client to the server. It includes two variants:
///
/// - `Search`: Represents a request to search for data, containing a trace ID, a query payload, and
///   a query type.
/// - `Cancel`: Represents a request to cancel a previous search, containing a trace ID.
///
/// The `WSQueryPayload` struct is used to encapsulate the details of a search query, including the
/// SQL query, start time, and end time.
///
/// This enum is serialized and deserialized using the `serde` crate, with the `type` field
/// indicating the variant and the `content` field containing the associated data.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize = "snake_case")
)]
pub enum WsClientEvents {
    Search(SearchEventReq),
    #[cfg(feature = "enterprise")]
    Cancel {
        trace_id: String,
    },
    Benchmark {
        id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchEventReq {
    pub trace_id: String,
    pub payload: config::meta::search::Request,
    pub time_offset: Option<i64>,
    pub stream_type: config::meta::stream::StreamType,
    pub use_cache: bool,
    pub search_type: config::meta::search::SearchEventType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize = "snake_case")
)]
pub enum WsServerEvents {
    SearchResponse {
        trace_id: String,
        results: config::meta::search::Response,
        response_type: SearchResponseType,
        time_offset: i64,
    },
    CancelResponse {
        trace_id: String,
        is_success: bool,
    },
    SearchError {
        trace_id: String,
        error: String,
    },
    RequestError {
        request_id: String,
        error: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SearchResponseType {
    Single,
    Partition { current: u64, total: u64 },
}

impl WsServerEvents {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize WsServerEvents")
    }
}
