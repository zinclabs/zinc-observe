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

use actix_ws::{MessageStream, Session};
use config::{
    get_config,
    meta::websocket::{ErrorType, SearchResultType},
};
use futures::StreamExt;
use infra::errors::Error;
use rand::prelude::SliceRandom;

use crate::handler::http::request::websocket::{
    search,
    utils::{sessions_cache_utils, WsClientEvents, WsServerEvents},
};

pub async fn run(
    mut session: Session,
    mut msg_stream: MessageStream,
    user_id: String,
    req_id: String,
    org_id: String,
) {
    let cfg = get_config();
    let mut accumulated_results: Vec<SearchResultType> = Vec::new();

    loop {
        tokio::select! {
            Some(msg) = msg_stream.next() => {
                match msg {
                    Ok(actix_ws::Message::Ping(bytes)) => {
                        if session.pong(&bytes).await.is_err() {
                            log::error!("[WS_HANDLER]: Pong failed for request_id: {}", req_id);
                            break;
                        }
                    }
                    Ok(actix_ws::Message::Text(msg)) => {
                        log::info!("[WS_HANDLER]: Request Id: {} Node Role: {} Received message: {}",
                            req_id,
                            cfg.common.node_role,
                            msg
                        );
                        handle_text_message(&mut session, &mut accumulated_results, &org_id, &user_id, &req_id, msg.to_string()).await;
                    }
                    Ok(actix_ws::Message::Close(reason)) => {
                        log::info!("[WS_HANDLER]: Request Id: {} Node Role: {} Closing connection with reason: {:?}",
                            req_id,
                            cfg.common.node_role,
                            reason
                        );
                        break;
                    }
                    _ => ()
                }
            }
        }
    }

    // Remove the session from the cache and close the session
    sessions_cache_utils::remove_session(&req_id);
    log::info!(
        "[WS_HANDLER]: Request Id: {} Node Role: {} Session cache utils len: {}",
        req_id,
        cfg.common.node_role,
        sessions_cache_utils::len_sessions()
    );
}

pub async fn handle_text_message(
    session: &mut Session,
    accumulated_results: &mut Vec<SearchResultType>,
    org_id: &str,
    user_id: &str,
    req_id: &str,
    msg: String,
) {
    match serde_json::from_str::<WsClientEvents>(&msg) {
        Ok(client_msg) => {
            match client_msg {
                WsClientEvents::Search(search_req) => {
                    match search::handle_search_request(
                        session,
                        accumulated_results,
                        org_id,
                        user_id,
                        *search_req.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            // close the session
                            let session = session.clone();
                            let _ = session.close(None).await;
                        }
                        Err(e) => {
                            log::error!(
                                    "[WS_HANDLER]: Failed to get search result for trace_id: {}, error: {:?}",
                                    search_req.trace_id,
                                    e
                                );
                            let err_res = WsServerEvents::Error {
                                error_type: ErrorType::SearchError {
                                    trace_id: search_req.trace_id.clone(),
                                    error: e.to_string(),
                                },
                            };
                            let _ = send_message(session, err_res.to_json().to_string()).await;
                            let session = session.clone();
                            let _ = session.close(None).await;
                        }
                    }
                }
                #[cfg(feature = "enterprise")]
                WsClientEvents::Cancel { trace_id } => {
                    let res = search::handle_cancel(&trace_id, org_id).await;
                    // close the session if send_message failed
                    let _ = send_message(session, res.to_json().to_string()).await;
                    let session = session.clone();
                    let _ = session.close(None).await;
                }
                WsClientEvents::Benchmark { id } => {
                    // simulate random delay for benchmarking by sleep for 10/20/30/60/90
                    // seconds
                    let delay: Vec<u64> = vec![10, 20, 30, 60, 90];
                    let delay = delay.choose(&mut rand::thread_rng()).unwrap();
                    log::info!(
                        "[WS_HANDLER]: Sleeping for benchmark, id: {}, delay: {}",
                        id,
                        delay
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(*delay)).await;

                    let response = serde_json::json!({
                        "id": id,
                        "took": delay,
                    });
                    let _ = send_message(session, response.to_string()).await;
                    let session = session.clone();
                    let _ = session.close(None).await;
                }
            }
        }
        Err(e) => {
            log::error!(
                "[WS_HANDLER]: Request id: {} Failed to parse message: {:?}, error: {:?}",
                req_id,
                msg,
                e
            );
            let err_res = WsServerEvents::Error {
                error_type: ErrorType::RequestError {
                    request_id: req_id.to_string(),
                    error: e.to_string(),
                },
            };
            let _ = send_message(session, err_res.to_json().to_string()).await;
            let session = session.clone();
            let _ = session.close(None).await;
        }
    }
}

pub async fn send_message(session: &mut Session, msg: String) -> Result<(), Error> {
    session.text(msg).await.map_err(|e| {
        log::error!("[WS_HANDLER]: Failed to send message: {:?}", e);
        Error::Message(e.to_string())
    })
}
