use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use actix_ws::{Message, Session};
use futures::stream::StreamExt;
use tokio::sync::Mutex;

use super::ws_utils::{
    get_ws_session, insert_in_ws_session, remove_from_ws_session, WEBSOCKET_MSG_CHAN,
};

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
                if let Some(mut session) = get_ws_session(&ws_msg.user_id).await{
                    let payload = serde_json::to_string(&ws_msg).unwrap();
                    if let Err(e) = session.text(payload).await {
                        log::error!("Error sending message: {}", e);
                        break;
                    }
                }else{
                    log::info!("No websocket session found for user_id: {}", ws_msg.user_id);
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
