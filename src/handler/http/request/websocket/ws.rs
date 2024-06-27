use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use futures::stream::StreamExt;
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
#[serde(tag = "t", content = "c")]
pub enum WebSocketMessageType {
    QueryEnqueued { trace_id: String },
    QueryCanceled { trace_id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct WebSocketMessage {
    user_id: String,
    content: WebSocketMessageType,
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

/// Spawns a background task that periodically checks the aliveness of the WebSocket session.
///
/// The task will ping the session every 5 seconds. If the session does not respond with a pong
/// within 10 seconds, the connection is considered dead and the session is closed.
///
/// The `alive` parameter is a shared mutex that tracks the last time a pong was received from the
/// client. This is used to determine if the client is still responsive.
async fn aliveness_check(user_id: String, mut session: Session, alive: Arc<Mutex<Instant>>) {
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if session.ping(b"").await.is_err() {
                remove_from_ws_session(user_id).await;
                break;
            }

            let client_timedout =
                Instant::now().duration_since(*alive.lock().await) > Duration::from_secs(10);
            if client_timedout {
                log::info!("{user_id} is not responding, closing connection");
                let _ = session.close(None).await;
                remove_from_ws_session(user_id).await;

                break;
            }
        }
    });
}

async fn websocket_handler(
    mut session: actix_ws::Session,
    alive: Arc<Mutex<Instant>>,
    mut msg_stream: actix_ws::MessageStream,
) {
    let mut receiver = WEBSOCKET_MSG_CHAN.1.resubscribe();

    loop {
        tokio::select! {
            Some(msg) = msg_stream.next() => {
                match msg {
                    Ok(Message::Ping(bytes)) => {
                        if session.pong(&bytes).await.is_err() {
                            log::info!("Failed to send pong, bailing");
                            return;
                        }
                    }
                    Ok(Message::Text(msg)) => {
                        log::info!("Relaying msg: {msg}");
                        // Process incoming message from client later.
                        // chat.send(msg).await;
                    }
                    Ok(Message::Close(reason)) => {
                        let _ = session.close(reason).await;
                        log::info!("Got close, bailing");
                        return;
                    }
                    Ok(Message::Continuation(_)) => {
                        let _ = session.close(None).await;
                        log::info!("Got continuation, bailing");
                        return;
                    }
                    Ok(Message::Pong(_)) => {
                        *alive.lock().await = Instant::now();
                    }
                    _ => (),
                };
            }
            Ok(ws_msg) = receiver.recv() => {
                let payload = serde_json::to_string(&ws_msg).unwrap();
                if let Err(e) = session.text(payload).await {
                    log::error!("Error sending message: {}", e);
                    break;
                }
            }
            else =>{
                log::info!("Break the look because no message received");
                 break
                },
        }
    }

    log::info!("Break the loop, everything is done");
    let _ = session.close(None).await;
}

#[get("/ws/{user_id}")]
pub async fn websocket(
    user_id: web::Path<String>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let user_id = user_id.into_inner();
    insert_in_ws_session(user_id.clone(), session.clone()).await;

    for (id, _sess) in WS_SESSIONS.lock().await.iter() {
        log::info!("User id: {id}, session found");
    }
    let alive = Arc::new(Mutex::new(Instant::now()));
    let alive1 = alive.clone();
    let session1 = session.clone();
    actix_web::rt::spawn(async move { aliveness_check(user_id, session1, alive1).await });

    // Spawn the handler
    actix_web::rt::spawn(websocket_handler(
        session.clone(),
        alive.clone(),
        msg_stream,
    ));

    // Return the response
    Ok(res)
}
