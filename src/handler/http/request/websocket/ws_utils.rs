use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};

/// Represents different types of WebSocket messages that can be sent between the client and server.
///
/// The `t` field indicates the type of the message, and the `c` field contains the message content.
///
/// - `QueryEnqueued`: Indicates that a query has been enqueued, with the `user_id` and `trace_id`
///   fields providing additional context.
/// - `QueryCanceled`: Indicates that a query has been canceled, with the `user_id` and `trace_id`
///   fields providing additional context.
#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case")
)]
pub enum WebSocketMessageType {
    QueryEnqueued { trace_id: String },
    QueryCanceled { trace_id: String },
}

impl WebSocketMessage {
    pub fn trace_id(&self) -> &str {
        match &self.payload {
            WebSocketMessageType::QueryEnqueued { trace_id } => trace_id,
            WebSocketMessageType::QueryCanceled { trace_id } => trace_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct WebSocketMessage {
    pub user_id: String,
    pub payload: WebSocketMessageType,
}

/// Represents different types of WebSocket messages that can be sent from the client to the server.
///
/// - `Search`: Indicates a search query, with the `trace_id` and `query` fields providing
///   additional context.
/// - `Cancel`: Indicates a request to cancel a query, with the `trace_id` field providing
///   additional context.
#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize="snake_case")
)]
pub enum WSClientMessage {
    Search { trace_id: String, query: String },
    Cancel { trace_id: String },
}

impl WSClientMessage {
    pub fn trace_id(&self) -> &str {
        match self {
            WSClientMessage::Search { trace_id, .. } => trace_id,
            WSClientMessage::Cancel { trace_id } => trace_id,
        }
    }
}

/// A lazy-initialized global channel for broadcasting WebSocket messages.
///
/// The channel has a capacity of 100 messages. The `WEBSOCKET_MSG_CHAN` static variable
/// contains the sender and receiver ends of the channel, which can be used to send and
/// receive WebSocket messages throughout the application.
pub static WEBSOCKET_MSG_CHAN: Lazy<(
    broadcast::Sender<WebSocketMessage>,
    broadcast::Receiver<WebSocketMessage>,
)> = Lazy::new(|| {
    let (tx, rx) = broadcast::channel(100);
    (tx, rx)
});

/// A lazy-initialized global HashMap that maps WebSocket request IDs to their corresponding
/// WebSocket session trace IDs.
///
/// This HashMap is used to store and retrieve WebSocket session trace IDs based on the request ID.
/// The `WS_REQUEST_ID_TO_TRACE_ID` static variable contains the HashMap, which can be used to
/// manage WebSocket session trace IDs throughout the application.
static WS_REQ_ID_TO_TRACE_ID: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn insert_trace_id_to_req_id(trace_id: String, request_id: String) {
    WS_REQ_ID_TO_TRACE_ID
        .lock()
        .await
        .insert(trace_id, request_id);
}

pub async fn get_req_id_from_trace_id(trace_id: &str) -> Option<String> {
    WS_REQ_ID_TO_TRACE_ID.lock().await.get(trace_id).cloned()
}

pub async fn remove_trace_id_from_cache(trace_id: &str) {
    WS_REQ_ID_TO_TRACE_ID.lock().await.remove(trace_id);
}

/// A lazy-initialized global HashMap that maps WebSocket request IDs to their corresponding
/// WebSocket sessions.
///
/// This HashMap is used to store and retrieve WebSocket sessions based on the request ID.
/// The `WS_SESSIONS_BY_REQ_ID` static variable contains the HashMap, which can be used to
/// manage WebSocket sessions throughout the application.
static WS_SESSIONS_BY_REQ_ID: Lazy<Mutex<HashMap<String, actix_ws::Session>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn remove_from_ws_session_by_req_id(request_id: String) {
    WS_SESSIONS_BY_REQ_ID.lock().await.remove(&request_id);
}

pub async fn insert_in_ws_session_by_req_id(request_id: String, session: actix_ws::Session) {
    WS_SESSIONS_BY_REQ_ID
        .lock()
        .await
        .insert(request_id, session);
}

pub async fn get_ws_session_by_req_id(request_id: &str) -> Option<actix_ws::Session> {
    WS_SESSIONS_BY_REQ_ID.lock().await.get(request_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::WSClientMessage;

    #[test]
    fn test_search_message_serialization() {
        let trace_id = "abc123".to_string();
        let query = "test query".to_string();
        let message = WSClientMessage::Search { trace_id, query };

        let serialized = serde_json::to_string(&message).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"search","content":{"trace_id":"abc123","query":"test query"}}"#
        );
    }

    #[test]
    fn test_cancel_message_serialization() {
        let trace_id = "def456".to_string();
        let message = WSClientMessage::Cancel { trace_id };

        let serialized = serde_json::to_string(&message).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"cancel","content":{"trace_id":"def456"}}"#
        );
    }

    #[test]
    fn test_search_message_deserialization() {
        let json = r#"{"type":"search","content":{"trace_id":"abc123","query":"test query"}}"#;
        let message: WSClientMessage = serde_json::from_str(json).unwrap();

        if let WSClientMessage::Search { trace_id, query } = message {
            assert_eq!(trace_id, "abc123");
            assert_eq!(query, "test query");
        } else {
            panic!("Unexpected message type");
        }
    }
}
