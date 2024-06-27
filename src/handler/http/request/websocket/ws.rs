use std::{
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
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "t", content = "c")]
pub enum WebSocketMessage {
    QueryEnqueued {
        user_id: String,
        trace_id: String,
    },
    QueryCanceled {
        user_id: String,
        trace_ids: Vec<String>,
    },
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

/// Spawns a background task that periodically checks the aliveness of the WebSocket session.
///
/// The task will ping the session every 5 seconds. If the session does not respond with a pong
/// within 10 seconds, the connection is considered dead and the session is closed.
///
/// The `alive` parameter is a shared mutex that tracks the last time a pong was received from the
/// client. This is used to determine if the client is still responsive.
async fn aliveness_check(mut session: Session, alive: Arc<Mutex<Instant>>) {
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if session.ping(b"").await.is_err() {
                break;
            }

            if Instant::now().duration_since(*alive.lock().await) > Duration::from_secs(10) {
                log::info!("Client is not responding, closing connection");
                let _ = session.close(None).await;
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

#[get("/ws")]
pub async fn websocket(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let alive = Arc::new(Mutex::new(Instant::now()));
    let alive1 = alive.clone();
    let session1 = session.clone();
    actix_web::rt::spawn(async move { aliveness_check(session1, alive1).await });

    // Spawn the handler
    actix_web::rt::spawn(websocket_handler(
        session.clone(),
        alive.clone(),
        msg_stream,
    ));

    // Return the response
    Ok(res)
}
