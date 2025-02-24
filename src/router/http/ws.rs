// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{future::Future, sync::Arc};

use actix_web::{rt, web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use fastwebsockets::{handshake, FragmentCollector, Frame, OpCode, Payload};
use futures_util::StreamExt;
use http_body_util::Empty;
use hyper::{
    body::Bytes,
    header::{CONNECTION, UPGRADE},
};
use tokio::{net::TcpStream, sync::Mutex};
use url::Url;

/// WebSocket proxy that manages bidirectional communication using two concurrent tasks.
///
/// # Architecture Overview
/// ```text
/// +--------+     +-----------------Router Process----------------+     +---------+
/// |        |     |  +-------------+        +-------------+       |     |         |
/// |        |     |  |    Task 1   |        |    Task 2   |       |     |         |
/// |Client  |<--->|  |client_to_bkd|  <-->  |backend_to_cl|       |<--->| Backend |
/// |        |     |  +-------------+        +-------------+       |     |         |
/// +--------+     +-----------------------------------------------+     +---------+
/// ```
///
/// # Close Sequence Flows
/// ```text
/// 1. Client Initiates Close:
///    Client -> Task1 -> Backend
///                    -> Send Close Frame
///                    -> Break Task1
///    Backend -> Task2 -> Client
///                    -> Send Close Ack to Backend
///                    -> Close Sink
///                    -> Break Task2
///
/// 2. Backend Initiates Close:
///    Backend -> Task2 -> Client
///                    -> Send Close Ack to Backend
///                    -> Close Sink
///                    -> Break Task2
///    Client -> Task1 -> Break Task1
/// ```
///
/// # Implementation Details
/// - Uses Arc<Mutex> for shared sink access between tasks
/// - Each task handles one direction of message flow
/// - Close sequence ensures proper WebSocket protocol shutdown
/// - Automatic resource cleanup when tasks complete
///
/// # Error Handling
/// - Connection errors trigger cleanup in both directions
/// - Timeouts prevent resource leaks
/// - Automatic task termination on connection close
///
/// # Message Flow Example
/// ```text
/// Normal Message:
///   Client -> Task1 -> convert format -> send to Backend
///   Backend -> Task2 -> convert format -> send to Client
///
/// Close Message:
///   1. Receive close frame
///   2. Forward to other endpoint
///   3. Send acknowledgment
///   4. Clean up resources
/// ```

struct WsProxySession {
    inner: Option<actix_ws::Session>,
}

impl WsProxySession {
    fn new(session: actix_ws::Session) -> Self {
        Self {
            inner: Some(session),
        }
    }

    async fn text<T: Into<String>>(&mut self, msg: T) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.text(msg).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    async fn binary<T: Into<bytes::Bytes>>(&mut self, msg: T) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.binary(msg).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    async fn ping(&mut self, payload: &[u8]) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.ping(payload).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    async fn pong(&mut self, payload: &[u8]) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.pong(payload).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    async fn close(
        &mut self,
        reason: Option<actix_ws::CloseReason>,
    ) -> Result<(), actix_ws::Closed> {
        if let Some(session) = self.inner.take() {
            session.close(reason).await
        } else {
            Err(actix_ws::Closed)
        }
    }
}

pub fn convert_to_websocket_url(url: &str) -> Result<String, Error> {
    let mut parsed_url = Url::parse(url)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Invalid URL: {}", e)))?;

    // Set the scheme
    parsed_url
        .set_scheme(if parsed_url.scheme() == "https" {
            "wss"
        } else {
            "ws"
        })
        .map_err(|_| {
            actix_web::error::ErrorInternalServerError("Failed to set WebSocket scheme")
        })?;

    // Ensure we have a port
    if parsed_url.port().is_none() {
        parsed_url
            .set_port(Some(if parsed_url.scheme() == "wss" {
                443
            } else {
                80
            }))
            .map_err(|_| {
                actix_web::error::ErrorInternalServerError("Failed to set default port")
            })?;
    }

    Ok(parsed_url.to_string())
}

pub async fn ws_proxy(
    req: HttpRequest,
    payload: web::Payload,
    ws_base_url: &str,
) -> Result<HttpResponse, Error> {
    let ws_url = convert_to_websocket_url(ws_base_url)?;
    let parsed_url = Url::parse(&ws_url).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Invalid WS URL: {}", e))
    })?;

    // Copy path and query from original request
    let req_uri = req.uri();
    let full_path = if let Some(query) = req_uri.query() {
        format!("{}?{}", req_uri.path(), query)
    } else {
        req_uri.path().to_string()
    };

    // Extract host and port for TCP connection
    let host = parsed_url
        .host_str()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Missing host"))?;
    let port = parsed_url
        .port()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Missing port"))?;

    // Get auth headers from original request
    let cookie_header = req
        .headers()
        .get("Cookie")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Client connection (actix-ws)
    let (response, client_session, mut client_stream) = match actix_ws::handle(&req, payload) {
        Ok(result) => result,
        Err(e) => {
            log::error!("[WS_PROXY] Error handling client: {:?}", e);
            return Err(e);
        }
    };
    let client_session = Arc::new(Mutex::new(WsProxySession::new(client_session)));
    let client_session2 = client_session.clone();

    // Backend connection (fastwebsockets) - SINGLE connection
    let stream = TcpStream::connect((host, port))
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    // Create handshake request with auth
    let key = handshake::generate_key();
    let request = hyper::Request::builder()
        .method("GET")
        .uri(full_path)  // Use the constructed full path with query
        .header("Host", format!("{}:{}", host, port))
        .header(UPGRADE, "websocket")
        .header(CONNECTION, "upgrade")
        .header("Sec-WebSocket-Key", &key)
        .header("Sec-WebSocket-Version", "13")
        .header("Cookie", cookie_header)
        .header("Authorization", auth_header)
        .body(Empty::<Bytes>::new())
        .unwrap();

    let (ws, _response) = handshake::client(&SpawnExecutor, request, stream)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    let ws = Arc::new(Mutex::new(FragmentCollector::new(ws)));
    let ws2 = ws.clone();

    // Client to Backend
    let client_to_backend = async move {
        while let Some(Ok(msg)) = client_stream.next().await {
            log::info!("[WS_PROXY] Client -> Backend: {:?}", msg);
            let mut backend = ws.lock().await;
            match msg {
                Message::Close(reason) => {
                    log::info!("[WS_PROXY] Client initiated close");
                    let _ = backend
                        .write_frame(Frame::close(1000, b"Client closed"))
                        .await;
                    break;
                }
                Message::Text(text) => {
                    log::info!("[WS_PROXY] Client ->>> Backend text: {}", text);
                    let frame =
                        Frame::new(true, OpCode::Text, None, Payload::Borrowed(text.as_bytes()));
                    if let Err(e) = backend.write_frame(frame).await {
                        log::error!("[WS_PROXY] Error writing frame: {:?}", e);
                        break;
                    }
                    log::info!("[WS_PROXY] Successfully sent frame to backend");
                }
                Message::Binary(bin) => {
                    let _ = backend
                        .write_frame(Frame::binary(Payload::Borrowed(&bin)))
                        .await;
                }
                Message::Ping(bytes) => {
                    let _ = backend
                        .write_frame(Frame::new(
                            true,
                            OpCode::Ping,
                            None,
                            Payload::Borrowed(&bytes),
                        ))
                        .await;
                }
                Message::Pong(bytes) => {
                    let _ = backend
                        .write_frame(Frame::new(
                            true,
                            OpCode::Pong,
                            None,
                            Payload::Borrowed(&bytes),
                        ))
                        .await;
                }
                _ => {
                    log::info!("[WS_PROXY] Client -> Backend binary: {:?}", msg);
                }
            }
        }
        log::info!("[WS_PROXY] Client to backend loop ended");
    };

    // Backend to Client
    let backend_to_client = async move {
        loop {
            // First get the frame while holding backend lock
            let frame = {
                let mut backend = ws2.lock().await;
                match backend.read_frame().await {
                    Ok(frame) => frame,
                    Err(e) => {
                        log::error!("[WS_PROXY] Backend error: {:?}", e);
                        break;
                    }
                }
            }; // backend lock is dropped here

            // Then process the frame with client lock
            let mut client = client_session2.lock().await;
            match frame.opcode {
                OpCode::Close => {
                    log::info!("[WS_PROXY] Backend initiated close");
                    let mut backend = ws2.lock().await; // Get backend lock again for close
                    let _ = backend.write_frame(Frame::close(1000, b"Closing")).await;
                    let _ = client
                        .close(Some(actix_ws::CloseReason {
                            code: actix_ws::CloseCode::Normal,
                            description: Some("Closing".into()),
                        }))
                        .await;
                    break;
                }
                OpCode::Text => {
                    let text = String::from_utf8_lossy(frame.payload.as_ref());
                    log::info!("[WS_PROXY] Backend -> Client text: {}", text);
                    let _ = client.text(text).await;
                }
                OpCode::Binary => {
                    let bytes = bytes::Bytes::copy_from_slice(frame.payload.as_ref());
                    let _ = client.binary(bytes).await;
                }
                OpCode::Ping => {
                    let _ = client.ping(frame.payload.as_ref()).await;
                }
                OpCode::Pong => {
                    let _ = client.pong(frame.payload.as_ref()).await;
                }
                _ => {}
            }
        }
    };

    rt::spawn(async move {
        let _ = tokio::join!(rt::spawn(client_to_backend), rt::spawn(backend_to_client));
        log::info!("[WS_PROXY] WebSocket proxy completed");
    });

    Ok(response)
}

// Helper function to convert an Actix HttpRequest to a Tungstenite WebSocket Request
// pub fn convert_actix_to_websocket_request(
//     actix_req: &HttpRequest,
//     ws_base_url: &str,
// ) -> Result<Request<()>, Box<dyn std::error::Error>> {
//     let mut url = Url::parse(ws_base_url)?;
//     if !actix_req.query_string().is_empty() {
//         url.set_query(Some(actix_req.query_string()));
//     }

//     let uri: Uri = url.as_str().parse()?;
//     let method = actix_req.method().clone();
//     let mut headers = HeaderMap::new();

//     for (key, value) in actix_req.headers().iter() {
//         if let Ok(header_str) = value.to_str() {
//             if let Ok(header_name) = HeaderName::from_str(key.as_str()) {
//                 if let Ok(header_value) = HeaderValue::from_str(header_str) {
//                     headers.insert(header_name, header_value);
//                 }
//             }
//         }
//     }

//     // insert headers for websockets, connection upgrade and upgrade to websocket
//     headers.insert(
//         HeaderName::from_static("connection"),
//         HeaderValue::from_static("upgrade"),
//     );
//     headers.insert(
//         HeaderName::from_static("upgrade"),
//         HeaderValue::from_static("websocket"),
//     );

//     Request::builder()
//         .method(method)
//         .uri(uri)
//         .body(())
//         .map_err(|e| e.into())
// }

// Add executor implementation
struct SpawnExecutor;
impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    fn execute(&self, fut: Fut) {
        tokio::task::spawn(fut);
    }
}
