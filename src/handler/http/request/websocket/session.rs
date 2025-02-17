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

use actix_http::ws::{CloseCode, CloseReason};
use actix_ws::{MessageStream, Session};
use config::{
    get_config,
    meta::websocket::{SearchEventReq, SearchResultType},
};
use dashmap::DashMap;
use futures::StreamExt;
use infra::errors::{self, Error};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::{
    auditor::{AuditMessage, Protocol, WsMeta},
    infra::config::get_config as get_o2_config,
};
use once_cell::sync::Lazy;
use rand::prelude::SliceRandom;
use tokio::sync::mpsc;

use super::utils::search_registry_utils::SearchState;
#[cfg(feature = "enterprise")]
use crate::handler::http::request::websocket::utils::search_registry_utils;
use crate::handler::http::request::websocket::{
    search,
    utils::{sessions_cache_utils, WsClientEvents, WsServerEvents},
};
#[cfg(feature = "enterprise")]
use crate::service::self_reporting::audit;

// Global registry for search requests by `trace_id`
pub static SEARCH_REGISTRY: Lazy<DashMap<String, SearchState>> = Lazy::new(DashMap::new);

// Do not clone the session, instead use a reference to the session
pub struct WsSession {
    inner: Option<Session>,
}

impl WsSession {
    pub fn new(inner: Session) -> Self {
        Self { inner: Some(inner) }
    }

    /// Send a text message to the client
    pub async fn text(&mut self, msg: String) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.text(msg).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    /// Close the session with a reason
    pub async fn close(&mut self, reason: Option<CloseReason>) -> Result<(), actix_ws::Closed> {
        if let Some(session) = self.inner.take() {
            session.close(reason).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    /// Send a pong response
    pub async fn pong(&mut self, payload: &[u8]) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.pong(payload).await
        } else {
            Err(actix_ws::Closed)
        }
    }

    pub async fn ping(&mut self, payload: &[u8]) -> Result<(), actix_ws::Closed> {
        if let Some(ref mut session) = self.inner {
            session.ping(payload).await
        } else {
            Err(actix_ws::Closed)
        }
    }
}

pub async fn run(
    mut msg_stream: MessageStream,
    user_id: String,
    req_id: String,
    org_id: String,
    path: String,
) {
    let cfg = get_config();
    let mut close_reason: Option<CloseReason> = None;

    loop {
        tokio::select! {
            Some(msg) = msg_stream.next() => {
                match msg {
                    Ok(actix_ws::Message::Ping(bytes)) => {
                        let mut session = if let Some(session) =
                                    sessions_cache_utils::get_mut_session(&req_id)
                                {
                                    session
                                } else {
                                    log::error!(
                                        "[WS_HANDLER]: req_id: {} session not found",
                                        req_id
                                    );
                                    return;
                                };
                        if session.pong(&bytes).await.is_err() {
                            log::error!("[WS_HANDLER]: Pong failed for request_id: {}", req_id);
                            break;
                        }
                    }
                    // handle pong as well
                    Ok(actix_ws::Message::Pong(bytes)) => {
                        log::info!("[WS_HANDLER]: Request Id: {} Node Role: {} Received pong {:?}",
                            req_id,
                            cfg.common.node_role,
                            bytes
                        );
                    }
                    Ok(actix_ws::Message::Text(msg)) => {
                        log::info!("[WS_HANDLER]: Request Id: {} Node Role: {} Received message: {}",
                            req_id,
                            cfg.common.node_role,
                            msg
                        );
                        handle_text_message(&org_id, &user_id, &req_id, msg.to_string(), path.clone()).await;
                    }
                    Ok(actix_ws::Message::Close(reason)) => {
                        if let Some(reason) = reason.as_ref() {
                            match reason.code {
                                CloseCode::Normal | CloseCode::Error => {
                                    log::info!("[WS_HANDLER]: Request Id: {} Node Role: {} Closing connection with reason: {:?}",
                                        req_id,
                                        cfg.common.node_role,
                                        reason
                                    );
                                },
                                _ => {
                                    log::error!("[WS_HANDLER]: Request Id: {} Node Role: {} Abnormal closure with reason: {:?}",req_id,cfg.common.node_role,reason);
                                },
                            }
                        }
                        close_reason = reason;
                        break;
                    }
                    _ => ()
                }
            }
        }
    }
    cleanup_and_close_session(&req_id, close_reason).await;
}

/// Handle the incoming text message
/// Text message is parsed into `WsClientEvents` and processed accordingly
/// Depending on each event type, audit must be done
/// Currently audit is done only for the search event
pub async fn handle_text_message(
    org_id: &str,
    user_id: &str,
    req_id: &str,
    msg: String,
    path: String,
) {
    match serde_json::from_str::<WsClientEvents>(&msg) {
        Ok(client_msg) => {
            match client_msg {
                WsClientEvents::Search(ref search_req) => {
                    handle_search_event(search_req, org_id, user_id, req_id, path.clone()).await;
                }
                #[cfg(feature = "enterprise")]
                WsClientEvents::Cancel { trace_id } => {
                    // First handle the cancel event
                    // send a cancel flag to the search task
                    handle_cancel_event(&trace_id).await;

                    log::info!(
                        "[WS_HANDLER]: trace_id: {}, Cancellation flag set to: {}",
                        trace_id,
                        search_registry_utils::is_cancelled(&trace_id)
                    );

                    let res = search::handle_cancel(&trace_id, org_id).await;
                    let _ = send_message(req_id, res.to_json().to_string()).await;
                    let close_reason = Some(CloseReason {
                        code: CloseCode::Normal,
                        description: Some(format!("trace_id {} Search canceled", trace_id)),
                    });

                    #[cfg(feature = "enterprise")]
                    let client_msg = WsClientEvents::Cancel { trace_id };

                    // Add audit before closing
                    #[cfg(feature = "enterprise")]
                    let is_audit_enabled = get_o2_config().common.audit_enabled;

                    #[cfg(feature = "enterprise")]
                    if is_audit_enabled {
                        audit(AuditMessage {
                            user_email: user_id.to_string(),
                            org_id: org_id.to_string(),
                            _timestamp: chrono::Utc::now().timestamp(),
                            protocol: Protocol::Ws(WsMeta {
                                path: path.clone(),
                                message_type: client_msg.get_type(),
                                content: client_msg.to_json(),
                                close_reason: format!("{:#?}", close_reason),
                            }),
                        })
                        .await;
                    }

                    cleanup_and_close_session(req_id, close_reason).await;
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
                    let _ = send_message(req_id, response.to_string()).await;
                    let close_reason = Some(CloseReason {
                        code: CloseCode::Normal,
                        description: Some(format!("id {} benchmark completed", id)),
                    });
                    cleanup_and_close_session(req_id, close_reason).await;
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
            let err_res = WsServerEvents::error_response(e.into(), Some(req_id.to_string()), None);
            let _ = send_message(req_id, err_res.to_json().to_string()).await;
            let close_reason = Some(CloseReason {
                code: CloseCode::Error,
                description: Some(format!("req_id {} Request Error", req_id)),
            });
            let mut session = if let Some(session) = sessions_cache_utils::get_mut_session(req_id) {
                session
            } else {
                log::error!("[WS_HANDLER]: req_id: {} session not found", req_id);
                return;
            };
            let _ = session.close(close_reason).await;
        }
    }
}

pub async fn send_message(req_id: &str, msg: String) -> Result<(), Error> {
    let mut session = if let Some(session) = sessions_cache_utils::get_mut_session(req_id) {
        session
    } else {
        log::error!("[WS_HANDLER]: req_id: {} session not found", req_id);
        return Err(Error::Message(format!(
            "[req_id {}] session not found",
            req_id
        )));
    };

    log::debug!("[WS_HANDLER]: req_id: {} sending message: {}", req_id, msg);
    session.text(msg).await.map_err(|e| {
        log::error!("[WS_HANDLER]: Failed to send message: {:?}", e);
        Error::Message(e.to_string())
    })
}

async fn cleanup_and_close_session(req_id: &str, close_reason: Option<CloseReason>) {
    if let Some(mut session) = sessions_cache_utils::get_mut_session(req_id) {
        let mut close_reason = close_reason;
        if let Some(reason) = close_reason.as_mut() {
            log::info!(
                "[WS_HANDLER]: req_id: {} Closing session with reason: {:?}",
                req_id,
                reason
            );
        } else {
            log::info!(
                "[WS_HANDLER]: req_id: {} Closing session with no specific reason",
                req_id
            );
        }

        // Experiment: sleep for the interval duration in milliseconds to avoid race condition
        // where the close frame (control frame) is treated as a data frame
        // and mal forms the data frame
        let cfg = get_config();
        let interval = cfg.common.websocket_close_frame_delay;
        if interval > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
        }

        // Attempt to close the session
        if let Err(e) = session.close(close_reason).await {
            log::error!(
                "[WS_HANDLER]: req_id: {} Failed to close session gracefully. Connection may have been closed prematurely: {:?}",
                req_id,
                e
            );
        } else {
            log::info!(
                "[WS_HANDLER]: req_id: {} Close frame sent successfully. Waiting for acknowledgment...",
                req_id
            );
        }
    } else {
        log::error!(
            "[WS_HANDLER]: req_id: {} Session not found during cleanup",
            req_id
        );
    }

    // Remove the session from the cache
    sessions_cache_utils::remove_session(req_id);
    log::info!(
        "[WS_HANDLER]: req_id: {} Session removed from cache. Remaining sessions: {}",
        req_id,
        sessions_cache_utils::len_sessions()
    );
}

// Main search handler
async fn handle_search_event(
    search_req: &SearchEventReq,
    org_id: &str,
    user_id: &str,
    req_id: &str,
    #[allow(unused_variables)] path: String,
) {
    let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
    let mut accumulated_results: Vec<SearchResultType> = Vec::new();

    let org_id = org_id.to_string();
    let user_id = user_id.to_string();
    let req_id = req_id.to_string();
    let trace_id = search_req.trace_id.clone();
    let trace_id_for_task = trace_id.clone();
    let search_req = search_req.clone();

    #[cfg(feature = "enterprise")]
    let is_audit_enabled = get_o2_config().common.audit_enabled;

    #[cfg(feature = "enterprise")]
    let client_msg = WsClientEvents::Search(Box::new(search_req.clone()));

    // Register running search BEFORE spawning task
    SEARCH_REGISTRY.insert(trace_id.clone(), SearchState::Running { cancel_tx });

    // Spawn the search task
    tokio::spawn(async move {
        // Handle the search request
        // If search is cancelled, the task will exit
        // Otherwise, the task will complete and the results will be sent to the client
        // The task will also update the search state to completed
        // The task will also close the session
        // The task will also cleanup the search resources
        tokio::select! {
            search_result = search::handle_search_request(
                &req_id,
                &mut accumulated_results,
                &org_id,
                &user_id,
                search_req.clone(),
            ) => {
                match search_result {
                    Ok(_) => {
                        if let Some(mut state) = SEARCH_REGISTRY.get_mut(&trace_id_for_task) {
                            *state = SearchState::Completed;
                        }

                        let close_reason = CloseReason {
                            code: CloseCode::Normal,
                            description: Some(format!("trace_id {} Search completed", trace_id_for_task)),
                        };

                        // Add audit before closing
                        #[cfg(feature = "enterprise")]
                        if is_audit_enabled {
                            audit(AuditMessage {
                                user_email: user_id,
                                org_id,
                                _timestamp: chrono::Utc::now().timestamp(),
                                protocol: Protocol::Ws(WsMeta {
                                    path: path.clone(),
                                    message_type: client_msg.get_type(),
                                    content: client_msg.to_json(),
                                    close_reason: format!("{:#?}", close_reason),
                                }),
                            })
                            .await;
                        }

                        cleanup_and_close_session(&req_id, Some(close_reason)).await;
                        cleanup_search_resources(&trace_id_for_task).await;
                    }
                    Err(e) => {
                        if let Some(close_reason) = handle_search_error(e, &req_id, &trace_id_for_task).await {
                            // Add audit before closing
                            #[cfg(feature = "enterprise")]
                            if is_audit_enabled {
                                audit(AuditMessage {
                                    user_email: user_id,
                                    org_id,
                                    _timestamp: chrono::Utc::now().timestamp(),
                                    protocol: Protocol::Ws(WsMeta {
                                        path: path.clone(),
                                        message_type: client_msg.get_type(),
                                        content: client_msg.to_json(),
                                        close_reason: format!("{:#?}", close_reason),
                                    }),
                                })
                                .await;
                            }


                            cleanup_and_close_session(&req_id, Some(close_reason)).await;
                        }
                        // Even if the search is cancelled, we need to cleanup the resources
                        cleanup_search_resources(&trace_id_for_task).await;
                    }
                }
            }
            _ = cancel_rx.recv() => {
                // if search is cancelled, update the state
                // the cancel handler will close the session

                // Just cleanup resources when cancelled
                cleanup_search_resources(&trace_id_for_task).await;
            }
        }
    });
}

// Cancel handler
#[cfg(feature = "enterprise")]
async fn handle_cancel_event(trace_id: &str) {
    if let Some(mut entry) = SEARCH_REGISTRY.get_mut(trace_id) {
        let state = entry.value_mut();
        let cancel_tx = match state {
            SearchState::Running { cancel_tx } => cancel_tx.clone(),
            state => {
                log::info!("[WS_HANDLER]: Cannot cancel search in state: {:?}", state);
                return;
            }
        };

        *entry.value_mut() = SearchState::Cancelled;

        if let Err(e) = cancel_tx.send(()).await {
            log::error!("[WS_HANDLER]: Failed to send cancel signal: {}", e);
        }

        log::info!("[WS_HANDLER]: Search cancelled for trace_id: {}", trace_id);
    }
}

async fn handle_search_error(e: Error, req_id: &str, trace_id: &str) -> Option<CloseReason> {
    // if the error is due to search cancellation, return.
    // the cancel handler will close the session
    if let errors::Error::ErrorCode(errors::ErrorCodes::SearchCancelQuery(_)) = e {
        log::info!(
            "[WS_HANDLER]: trace_id: {}, Return from search handler, search canceled",
            trace_id
        );
        // Update state to cancelled before returning
        if let Some(mut state) = SEARCH_REGISTRY.get_mut(trace_id) {
            *state = SearchState::Cancelled;
        }
        return None;
    }

    log::error!("[WS_HANDLER]: trace_id: {} Search error: {}", trace_id, e);
    // Send error response
    let err_res =
        WsServerEvents::error_response(e, Some(trace_id.to_string()), Some(req_id.to_string()));
    let _ = send_message(req_id, err_res.to_json().to_string()).await;

    // Close with error
    let close_reason = CloseReason {
        code: CloseCode::Error,
        description: Some(format!("trace_id {} Search Error", trace_id)),
    };

    // Update registry state
    if let Some(mut state) = SEARCH_REGISTRY.get_mut(trace_id) {
        *state = SearchState::Completed;
    }

    Some(close_reason)
}

// Add cleanup function
async fn cleanup_search_resources(trace_id: &str) {
    SEARCH_REGISTRY.remove(trace_id);
    log::debug!("[WS_HANDLER]: trace_id: {}, Resources cleaned up", trace_id);
}
