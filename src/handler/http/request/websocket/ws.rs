use std::{
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_ws::{Message, Session};
use futures::stream::StreamExt;
use once_cell::sync::Lazy;
use tokio::sync::{broadcast, Mutex};

pub(crate) static WEBSOCKET_MSG_CHAN: Lazy<(
    broadcast::Sender<String>,
    broadcast::Receiver<String>,
)> = Lazy::new(|| {
    let (tx, rx) = broadcast::channel(100);
    (tx, rx)
});

async fn aliveness_check(mut session: Session, alive: Arc<Mutex<Instant>>) {
    actix_web::rt::spawn(async move {
        let mut interval = actix_web::rt::time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if session.ping(b"").await.is_err() {
                break;
            }

            if Instant::now().duration_since(*alive.lock().await) > Duration::from_secs(10) {
                tracing::info!("Client is not responding, closing connection");
                let _ = session.close(None).await;
                break;
            }
        }
    });
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
                            tracing::info!("Failed to send pong, bailing");
                            return;
                        }
                    }
                    Ok(Message::Text(msg)) => {
                        tracing::info!("Relaying msg: {msg}");
                        // Process incoming message from client later.
                        // chat.send(msg).await;
                    }
                    Ok(Message::Close(reason)) => {
                        let _ = session.close(reason).await;
                        tracing::info!("Got close, bailing");
                        return;
                    }
                    Ok(Message::Continuation(_)) => {
                        let _ = session.close(None).await;
                        tracing::info!("Got continuation, bailing");
                        return;
                    }
                    Ok(Message::Pong(_)) => {
                        *alive.lock().await = Instant::now();
                    }
                    _ => (),
                };
            }
            Ok(msg) = receiver.recv() => {
                if let Err(e) = session.text(msg).await {
                    tracing::error!("Error sending message: {}", e);
                    break;
                }
            }
            else =>{
                tracing::info!("Break the look because no message received");
                 break
                },
        }
    }

    tracing::info!("Break the loop, everything is done");
    let _ = session.close(None).await;
}
