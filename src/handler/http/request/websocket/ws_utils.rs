use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case")
)]
pub enum WSMessageType {
    QueryEnqueued { trace_id: String },
    QueryCanceled { trace_id: String },
    QueryProcessingStarted { trace_id: String },
}

impl WSInternalMessage {
    pub fn trace_id(&self) -> &str {
        match &self.payload {
            WSMessageType::QueryEnqueued { trace_id, .. } => trace_id,
            WSMessageType::QueryCanceled { trace_id } => trace_id,
            WSMessageType::QueryProcessingStarted { trace_id } => trace_id,
        }
    }
}

/// Represents an internal WebSocket message that can be sent between the different jobs on backend.
///
/// The `WSInternalMessage` struct contains two fields:
///
/// - `user_id`: A string representing the ID of the user associated with the message.
/// - `payload`: The actual content of the message, represented by the `WSMessageType` enum.
///
/// This struct is used internally within the application to represent and transmit WebSocket
/// messages between different components.
#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct WSInternalMessage {
    pub user_id: String,
    pub payload: WSMessageType,
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
#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize = "snake_case")
)]
pub enum WSClientMessage {
    Search {
        trace_id: String,
        query: WSQueryPayload,
        #[serde(rename = "type")]
        query_type: String,
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

impl WSClientMessage {
    pub fn trace_id(&self) -> &str {
        match self {
            WSClientMessage::Search { trace_id, .. } => trace_id,
            WSClientMessage::Cancel { trace_id } => trace_id,
        }
    }
}

/// Represents a WebSocket server response message, which can be either a "QueryEnqueued" or
/// "QueryCanceled" variant.
///
/// The "QueryEnqueued" variant contains the trace ID, the query payload, and the query type.
/// The "QueryCanceled" variant contains the trace ID of the canceled query.
///
/// These messages are serialized and deserialized using the `serde` crate, with the `type` field
/// indicating the variant and the `content` field containing the associated data.
#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize = "snake_case")
)]
pub enum WSServerResponseMessage {
    QueryEnqueued {
        trace_id: String,
        query: WSQueryPayload,
        #[serde(rename = "type")]
        query_type: String,
    },
    QueryCanceled {
        trace_id: String,
    },
}

/// A lazy-initialized global channel for broadcasting WebSocket messages.
///
/// The channel has a capacity of 100 messages. The `WEBSOCKET_MSG_CHAN` static variable
/// contains the sender and receiver ends of the channel, which can be used to send and
/// receive WebSocket messages throughout the application.
pub static WEBSOCKET_MSG_CHAN: Lazy<(
    broadcast::Sender<WSInternalMessage>,
    broadcast::Receiver<WSInternalMessage>,
)> = Lazy::new(|| {
    let (tx, rx) = broadcast::channel(100);
    (tx, rx)
});

/// A lazy-initialized global HashMap that maps WebSocket request IDs to their corresponding
/// WebSocket session trace IDs.
///
/// This HashMap is used to store and retrieve WebSocket session trace IDs based on the request ID.
/// The `WS_TRACE_ID_TO_REQ_ID` static variable contains the HashMap, which can be used to
/// manage WebSocket session trace IDs throughout the application.
static WS_TRACE_ID_TO_REQ_ID: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn insert_trace_id_to_req_id(trace_id: String, request_id: String) {
    WS_TRACE_ID_TO_REQ_ID
        .lock()
        .await
        .insert(trace_id, request_id);
}

pub async fn print_req_id_to_trace_id() {
    log::debug!("WS_TRACE_ID_TO_REQ_ID:");
    for (trace_id, req_id) in WS_TRACE_ID_TO_REQ_ID.lock().await.iter() {
        log::debug!("{} -> {}", trace_id, req_id);
    }
}

pub async fn get_req_id_from_trace_id(trace_id: &str) -> Option<String> {
    WS_TRACE_ID_TO_REQ_ID.lock().await.get(trace_id).cloned()
}

pub async fn remove_trace_id_from_cache(trace_id: &str) {
    WS_TRACE_ID_TO_REQ_ID.lock().await.remove(trace_id);
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

pub async fn print_sessions() {
    log::debug!("WS_SESSIONS_BY_REQ_ID:");
    for (req_id, _session) in WS_SESSIONS_BY_REQ_ID.lock().await.iter() {
        log::debug!("Request id found {}", req_id);
    }
}

static WS_TRACE_ID_QUERY_OBJECT: Lazy<Mutex<HashMap<String, WSClientMessage>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn insert_in_ws_trace_id_query_object(trace_id: String, query_object: WSClientMessage) {
    WS_TRACE_ID_QUERY_OBJECT
        .lock()
        .await
        .insert(trace_id, query_object);
}

pub async fn get_ws_trace_id_query_object(trace_id: &str) -> Option<WSClientMessage> {
    WS_TRACE_ID_QUERY_OBJECT.lock().await.get(trace_id).cloned()
}

pub async fn remove_from_ws_trace_id_query_object(trace_id: &str) {
    WS_TRACE_ID_QUERY_OBJECT.lock().await.remove(trace_id);
}

pub async fn print_ws_trace_id_query_object() {
    log::debug!("WS_TRACE_ID_QUERY_OBJECT:");
    for (trace_id, query_object) in WS_TRACE_ID_QUERY_OBJECT.lock().await.iter() {
        log::debug!("{} -> {:?}", trace_id, query_object);
    }
}
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_ws_client_message_search() {
        let trace_id = "123".to_string();
        let query = WSQueryPayload {
            sql: "SELECT * FROM table".to_string(),
            start_time: 1234567890,
            end_time: 1234567900,
        };
        let query_type = "search".to_string();

        let message = WSClientMessage::Search {
            trace_id: trace_id.clone(),
            query: query.clone(),
            query_type: query_type.clone(),
        };

        assert_eq!(message.trace_id(), trace_id);
    }

    #[test]
    fn test_ws_client_message_cancel() {
        let trace_id = "456".to_string();

        let message = WSClientMessage::Cancel {
            trace_id: trace_id.clone(),
        };

        assert_eq!(message.trace_id(), trace_id);
    }

    #[test]
    fn test_ws_query_payload() {
        let sql = "SELECT * FROM table".to_string();
        let start_time = 1234567890;
        let end_time = 1234567900;

        let payload = WSQueryPayload {
            sql: sql.clone(),
            start_time,
            end_time,
        };

        assert_eq!(payload.sql, sql);
        assert_eq!(payload.start_time, start_time);
        assert_eq!(payload.end_time, end_time);
    }

    #[test]
    fn test_query_deserialization() {
        let json_payload = r#"{
            "type": "search",
            "content": {
                "type": "search_logs_histogram",
                "trace_id": "0ec50ae6ecad4483b7502103cdaa764d",
                "query":  {
                    "sql": "select histogram(_timestamp, '1 hour') AS zo_sql_key, count(*) AS zo_sql_num from \"e2e_automate\"  GROUP BY zo_sql_key ORDER BY zo_sql_key",
                    "start_time": 1718630409649000,
                    "end_time": 1719840009649000,
                    "size": -1,
                    "sql_mode": "full"
                }
            }
            }"#;

        let _payload: WSClientMessage = serde_json::from_str(json_payload).unwrap();
        assert!(true);
    }
}
