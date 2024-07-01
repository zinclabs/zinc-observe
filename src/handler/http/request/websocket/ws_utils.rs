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
    QueryEnqueued {
        trace_id: String,
        query: WSQueryPayload,
    },
    QueryCanceled {
        trace_id: String,
    },
    QueryProcessingStarted {
        trace_id: String,
    },
}

impl WebSocketMessage {
    pub fn trace_id(&self) -> &str {
        match &self.payload {
            WebSocketMessageType::QueryEnqueued { trace_id, .. } => trace_id,
            WebSocketMessageType::QueryCanceled { trace_id } => trace_id,
            WebSocketMessageType::QueryProcessingStarted { trace_id } => trace_id,
        }
    }

    pub fn update_payload(&mut self, updated_payload: WSQueryPayload) {
        match &mut self.payload {
            WebSocketMessageType::QueryEnqueued { query, .. } => *query = updated_payload.clone(),
            WebSocketMessageType::QueryCanceled { .. } => {}
            WebSocketMessageType::QueryProcessingStarted { .. } => {}
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

pub async fn print_req_id_to_trace_id() {
    println!("Traceid -> request_id");
    for (trace_id, req_id) in WS_REQ_ID_TO_TRACE_ID.lock().await.iter() {
        println!("{} -> {}", trace_id, req_id);
    }
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
    log::info!("Removing ws session for request id {}", request_id);
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
    println!("Sessions:");
    for (req_id, _session) in WS_SESSIONS_BY_REQ_ID.lock().await.iter() {
        println!("Request id found {}", req_id);
    }
}

/// A lazy-initialized global HashMap that maps WebSocket trace IDs to their corresponding
/// WebSocket query payloads.
///
/// This HashMap is used to store and retrieve WebSocket query payloads based on the trace ID.
/// The `WS_TRACE_ID_QUERY_OBJECT` static variable contains the HashMap, which can be used to
/// manage WebSocket query payloads throughout the application.
static WS_TRACE_ID_QUERY_OBJECT: Lazy<Mutex<HashMap<String, WSQueryPayload>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn insert_in_ws_trace_id_query_object(trace_id: String, query_object: WSQueryPayload) {
    WS_TRACE_ID_QUERY_OBJECT
        .lock()
        .await
        .insert(trace_id, query_object);
}

pub async fn get_ws_trace_id_query_object(trace_id: &str) -> Option<WSQueryPayload> {
    WS_TRACE_ID_QUERY_OBJECT.lock().await.get(trace_id).cloned()
}

pub async fn remove_from_ws_trace_id_query_object(trace_id: &str) {
    WS_TRACE_ID_QUERY_OBJECT.lock().await.remove(trace_id);
}

pub async fn print_ws_trace_id_query_object() {
    println!("Traceid -> query_object");
    for (trace_id, query_object) in WS_TRACE_ID_QUERY_OBJECT.lock().await.iter() {
        println!("{} -> {:?}", trace_id, query_object);
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
