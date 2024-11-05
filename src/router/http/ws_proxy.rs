use std::str::FromStr;

use actix::prelude::*;
use actix_web::HttpRequest;
use actix_web_actors::ws;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite, tungstenite::client::IntoClientRequest};
use tungstenite::http::{HeaderName, HeaderValue, Request};
use url::Url;

#[derive(Message)]
#[rtype(result = "()")]
pub struct WebSocketMessageWrapper(ws::Message);

// Define your WebSocket actor
pub struct CustomWebSocketHandlers {
    pub url: String,
    pub req: HttpRequest,
    pub tx: mpsc::UnboundedSender<ws::Message>,
}

fn from_actix_message(msg: ws::Message) -> tungstenite::Message {
    match msg {
        ws::Message::Text(text) => tungstenite::Message::Text(text.to_string()),
        ws::Message::Binary(bin) => tungstenite::Message::Binary(bin.to_vec()),
        ws::Message::Ping(msg) => tungstenite::Message::Ping(msg.to_vec()),
        ws::Message::Pong(msg) => tungstenite::Message::Pong(msg.to_vec()),
        ws::Message::Close(None) => {
            log::info!(
                "[WebSocketProxy] Received a Message::Close from internal client, closing connection to proxied server"
            );
            tungstenite::Message::Close(None)
        }
        _ => {
            log::info!("[WebSocketProxy] Unsupported message type");
            tungstenite::Message::Close(None)
        }
    }
}

fn from_tungstenite_msg_to_actix_msg(msg: tungstenite::Message) -> ws::Message {
    match msg {
        tungstenite::Message::Text(text) => ws::Message::Text(text.into()),
        tungstenite::Message::Binary(bin) => ws::Message::Binary(bin.into()),
        tungstenite::Message::Ping(msg) => ws::Message::Ping(msg.into()),
        tungstenite::Message::Pong(msg) => ws::Message::Pong(msg.into()),
        tungstenite::Message::Close(None) => ws::Message::Close(None),
        _ => {
            log::info!("[WebSocketProxy] Unsupported message type");
            ws::Message::Close(None)
        }
    }
}

impl Actor for CustomWebSocketHandlers {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!(
            "[WebSocketProxy] Started WebSocket connection to {}",
            self.url
        );

        let (tx, mut rx) = mpsc::unbounded_channel();
        self.tx = tx;
        let addr = ctx.address();
        let ws_req = match convert_actix_to_tungstenite_request(&self.req, &self.url) {
            Ok(req) => req,
            Err(e) => {
                log::error!(
                    "[WebSocketProxy] Failed to convert Actix request to Tungstenite request: {:?}",
                    e
                );
                return;
            }
        };

        tokio::spawn(async move {
            match connect_async(ws_req).await {
                Ok((ws_stream, _)) => {
                    let (mut ws_sink, mut ws_stream) = ws_stream.split();

                    tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            log::info!(
                                "[WebSocketProxy] Received message from the original websocket actor: {msg:?}"
                            );
                            ws_sink
                                .send(from_actix_message(msg))
                                .await
                                .expect("Failed to send message");
                        }
                    });

                    while let Some(Ok(msg)) = ws_stream.next().await {
                        log::info!(
                            "[WebSocketProxy] Should have sent to the original websocket actor to send back to client: {msg:?}"
                        );
                        addr.do_send(WebSocketMessageWrapper(from_tungstenite_msg_to_actix_msg(
                            msg,
                        )));
                    }
                }
                Err(e) => {
                    log::error!(
                        "[WebSocketProxy] Failed to connect to backend WebSocket: {:?}",
                        e
                    );
                    addr.do_send(WebSocketMessageWrapper(ws::Message::Close(None)));
                }
            }
        });
    }
}

impl Handler<WebSocketMessageWrapper> for CustomWebSocketHandlers {
    type Result = ();

    fn handle(&mut self, msg: WebSocketMessageWrapper, ctx: &mut Self::Context) {
        log::info!(
            "[WebSocketProxy] Received message from the client: {:?}",
            &msg.0
        );
        match msg.0 {
            ws::Message::Text(text) => {
                log::info!("[WebSocketProxy] Text message received: {}", text);
                ctx.text(text);
            }
            ws::Message::Binary(bin) => {
                log::info!("[WebSocketProxy] Binary message received: {:?}", bin);
                ctx.binary(bin);
            }
            ws::Message::Ping(msg) => {
                log::info!("[WebSocketProxy] Ping message received: {:?}", msg);
                ctx.ping(&msg);
            }
            ws::Message::Pong(msg) => {
                log::info!("[WebSocketProxy] Pong message received: {:?}", msg);
                ctx.pong(&msg);
            }
            ws::Message::Close(reason) => {
                log::info!("[WebSocketProxy] Close message received: {:?}", reason);
                ctx.close(reason);
            }
            _ => {
                log::error!("Unsupported message type");
            }
        }
    }
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for CustomWebSocketHandlers {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                self.tx
                    .send(ws::Message::Text(text))
                    .expect("Failed to forward message");
            }
            Ok(ws::Message::Binary(bin)) => {
                self.tx
                    .send(ws::Message::Binary(bin))
                    .expect("Failed to forward message");
            }
            _ => (),
        }
    }
}

pub fn convert_to_websocket_url(url_str: &str) -> Result<String, String> {
    let mut parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(e) => {
            return Err(format!("Failed to parse URL: {}", e));
        }
    };

    // Check the scheme and update it accordingly
    match parsed_url.scheme() {
        "http" => {
            parsed_url
                .set_scheme("ws")
                .map_err(|_| "Failed to set scheme to ws")?;
        }
        "https" => {
            parsed_url
                .set_scheme("wss")
                .map_err(|_| "Failed to set scheme to wss")?;
        }
        _ => {
            return Err(format!("Unsupported URL scheme: {}", parsed_url.scheme()));
        }
    }

    Ok(parsed_url.to_string())
}

/// Helper function to convert an Actix HttpRequest to a Tungstenite WebSocket Request
pub fn convert_actix_to_tungstenite_request(
    actix_req: &HttpRequest,
    ws_base_url: &str,
) -> Result<tungstenite::http::Request<()>, Box<dyn std::error::Error>> {
    let mut url = Url::parse(ws_base_url)?;
    let query_string = actix_req.query_string();
    if !query_string.is_empty() {
        url.set_query(Some(query_string));
    }

    let uri: tungstenite::http::Uri = url.as_str().parse()?;

    let method = reqwest::Method::from_str(actix_req.method().as_str()).unwrap();
    let mut headers = tokio_tungstenite::tungstenite::http::HeaderMap::new();
    for (key, value) in actix_req.headers().iter() {
        let key = HeaderName::from_str(key.as_str()).unwrap();
        let value = HeaderValue::from_str(value.to_str().unwrap()).unwrap();
        headers.insert(key, value);
    }

    // insert headers for websockets, connection upgrade and upgrade to websocket
    headers.insert(
        HeaderName::from_static("connection"),
        HeaderValue::from_static("upgrade"),
    );
    headers.insert(
        HeaderName::from_static("upgrade"),
        HeaderValue::from_static("websocket"),
    );

    // Build the WebSocket request using the extracted method, URI, and headers
    let mut request_builder = tungstenite::http::Request::builder()
        .method(method)
        .uri(uri);

    for (key, value) in headers.iter() {
        request_builder = request_builder.header(key, value);
    }

    let ws_request = request_builder.body(())?;

    Ok(ws_request)
}
