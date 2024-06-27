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

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct WebSocketMessage {
    pub user_id: String,
    pub content: WebSocketMessageType,
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

static WS_SESSIONS: Lazy<Mutex<HashMap<String, actix_ws::Session>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn remove_from_ws_session(user_id: String) {
    WS_SESSIONS.lock().await.remove(&user_id);
}

pub async fn insert_in_ws_session(user_id: String, session: actix_ws::Session) {
    WS_SESSIONS.lock().await.insert(user_id, session);
}

pub async fn get_ws_session(user_id: &str) -> Option<actix_ws::Session> {
    WS_SESSIONS.lock().await.get(user_id).cloned()
}
