use serde::{Deserialize, Serialize};

pub mod sessions_cache_utils {
    use crate::common::infra::config::WS_SESSIONS;

    /// Insert a new session into the cache
    pub fn insert_session(request_id: &str, session: actix_ws::Session) {
        WS_SESSIONS.insert(request_id.to_string(), session);
    }

    /// Remove a session from the cache
    pub fn remove_session(request_id: &str) {
        WS_SESSIONS.remove(request_id);
    }

    /// Get a session from the cache
    pub fn get_session(request_id: &str) -> Option<actix_ws::Session> {
        WS_SESSIONS.get(request_id).map(|entry| entry.clone())
    }

    /// Check if a session exists in the cache
    pub fn contains_session(request_id: &str) -> bool {
        WS_SESSIONS.contains_key(request_id)
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
pub enum WSClientMessage {
    Search {
        query: config::meta::search::Request,
    },
    Cancel {
        trace_id: String,
    },
}

/// Represents the payload for a WebSocket query, containing the SQL query, start time, and end
/// time.
///
/// This struct is used to encapsulate the details of a search query that can be sent from a
/// WebSocket client to the server.
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Default)]
pub struct WSQueryPayload {
    pub sql: String,
    pub start_time: i64,
    pub end_time: i64,
}
