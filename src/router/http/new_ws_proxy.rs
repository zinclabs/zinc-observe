use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite};

use crate::router::http::ws_proxy::convert_actix_to_tungstenite_request;

/// WebSocket proxy handler
pub async fn ws_proxy(
    req: HttpRequest,
    payload: web::Payload,
    ws_base_url: String,
) -> Result<HttpResponse, Error> {
    // Upgrade the client connection to a WebSocket
    let (response, mut session, mut client_msg_stream) = actix_ws::handle(&req, payload)?;

    dbg!(&response);

    let ws_req = match convert_actix_to_tungstenite_request(&req, &ws_base_url) {
        Ok(req) => req,
        Err(e) => {
            log::error!(
                "[WebSocketProxy] Failed to convert Actix request to Tungstenite request: {:?}",
                e
            );
            return Err(actix_web::error::ErrorInternalServerError(format!(
                "Failed to convert Actix request to Tungstenite request: {:?}",
                e
            )))?;
        }
    };

    dbg!(&ws_req);

    // Connect to the backend WebSocket service
    let (backend_ws_stream, _) = connect_async(ws_req).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to connect to backend: {}", e))
    })?;

    // Split backend Websocket stream into sink and stream
    let (mut backend_ws_sink, mut backend_ws_stream) = backend_ws_stream.split();

    // Spawn tasks to forward messages between client and backend WebSocket
    let client_to_backend = async move {
        while let Some(Ok(msg)) = client_msg_stream.next().await {
            if backend_ws_sink.send(from_actix_message(msg)).await.is_err() {
                return;
            }
        }
    };

    let backend_to_client = async move {
        while let Some(Ok(msg)) = backend_ws_stream.next().await {
            match from_tungstenite_msg_to_actix_msg(msg) {
                Message::Text(text) => {
                    if session.text(text).await.is_err() {
                        return;
                    }
                }
                Message::Binary(bin) => {
                    if session.binary(bin).await.is_err() {
                        return;
                    }
                }
                Message::Ping(ping) => {
                    if session.ping(&ping).await.is_err() {
                        return;
                    }
                }
                Message::Pong(pong) => {
                    if session.pong(&pong).await.is_err() {
                        return;
                    }
                }
                Message::Close(reason) => {
                    let _ = session.close(reason).await;
                    break;
                }
                _ => {
                    log::warn!("Unsupported message type from backend");
                }
            }
        }
    };

    // Spawn both tasks
    actix_web::rt::spawn(client_to_backend);
    actix_web::rt::spawn(backend_to_client);

    // Return the WebSocket handshake response
    Ok(response)
}

fn from_actix_message(msg: Message) -> tungstenite::protocol::Message {
    match msg {
        Message::Text(text) => tungstenite::protocol::Message::Text(text.to_string()),
        Message::Binary(bin) => tungstenite::protocol::Message::Binary(bin.to_vec()),
        Message::Ping(msg) => tungstenite::protocol::Message::Ping(msg.to_vec()),
        Message::Pong(msg) => tungstenite::protocol::Message::Pong(msg.to_vec()),
        Message::Close(None) => {
            log::info!(
                "[WebSocketProxy] Received a Message::Close from internal client, closing connection to proxied server"
            );
            tungstenite::protocol::Message::Close(None)
        }
        _ => {
            log::info!("[WebSocketProxy] Unsupported message type");
            tungstenite::protocol::Message::Close(None)
        }
    }
}

fn from_tungstenite_msg_to_actix_msg(msg: tungstenite::protocol::Message) -> Message {
    match msg {
        tungstenite::protocol::Message::Text(text) => Message::Text(text.into()),
        tungstenite::protocol::Message::Binary(bin) => Message::Binary(bin.into()),
        tungstenite::protocol::Message::Ping(msg) => Message::Ping(msg.into()),
        tungstenite::protocol::Message::Pong(msg) => Message::Pong(msg.into()),
        tungstenite::protocol::Message::Close(None) => Message::Close(None),
        _ => {
            log::info!("[WebSocketProxy] Unsupported message type");
            Message::Close(None)
        }
    }
}
