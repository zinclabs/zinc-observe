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
use config::{get_config, meta::websocket::SearchResultType};
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

#[allow(unused_imports)]
use crate::handler::http::request::websocket::utils::cancellation_registry_cache_utils;
use crate::handler::http::request::websocket::{
    search,
    utils::{sessions_cache_utils, WsClientEvents, WsServerEvents},
};
#[cfg(feature = "enterprise")]
use crate::service::self_reporting::audit;

// Global cancellation registry for search requests by `trace_id`
pub static CANCELLATION_FLAGS: Lazy<DashMap<String, bool>> = Lazy::new(DashMap::new);

#[derive(Clone)]
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
                                    sessions_cache_utils::get_session(&req_id)
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
    #[cfg(feature = "enterprise")]
    let is_audit_enabled = get_o2_config().common.audit_enabled;

    match serde_json::from_str::<WsClientEvents>(&msg) {
        Ok(client_msg) => {
            match client_msg {
                WsClientEvents::Search(ref search_req) => {
                    let mut accumulated_results: Vec<SearchResultType> = Vec::new();
                    let org_id = org_id.to_string();
                    let user_id = user_id.to_string();
                    let req_id = req_id.to_string();
                    let search_req = search_req.clone();
                    #[cfg(feature = "enterprise")]
                    let client_msg = client_msg.clone();
                    #[allow(unused_variables)]
                    let path = path.to_string();
                    let trace_id = search_req.trace_id.clone();

                    let task = tokio::spawn(async move {
                        match search::handle_search_request(
                            &req_id,
                            &mut accumulated_results,
                            &org_id,
                            &user_id,
                            *search_req.clone(),
                        )
                        .await
                        {
                            Ok(_) => {
                                // close the session
                                let close_reason = Some(CloseReason {
                                    code: CloseCode::Normal,
                                    description: Some(format!(
                                        "[trace_id {}] Search completed",
                                        search_req.trace_id.clone()
                                    )),
                                });

                                // audit
                                #[cfg(feature = "enterprise")]
                                if is_audit_enabled {
                                    audit(AuditMessage {
                                        user_email: user_id,
                                        org_id,
                                        _timestamp: chrono::Utc::now().timestamp(),
                                        protocol: Protocol::Ws(WsMeta {
                                            path,
                                            message_type: client_msg.get_type(),
                                            content: client_msg.to_json(),
                                            close_reason: format!(
                                                "{:#?}",
                                                close_reason.clone().unwrap()
                                            ),
                                        }),
                                    })
                                    .await;
                                }

                                cleanup_and_close_session(&req_id, close_reason).await;
                            }
                            Err(e) => {
                                log::error!(
                                    "[WS_HANDLER]: Failed to get search result for trace_id: {}, error: {:?}",
                                    search_req.trace_id,
                                    e
                                );

                                // if the error is due to search cancellation, return
                                // the cancel handler will close the session
                                if let errors::Error::ErrorCode(
                                    errors::ErrorCodes::SearchCancelQuery(_),
                                ) = e
                                {
                                    log::info!(
                                        "[WS_HANDLER]: trace_id: {}, Return from search handler, search canceled",
                                        search_req.trace_id
                                    );
                                    return;
                                }

                                let err_msg =
                                    format!("[trace_id: {}, error: {}]", search_req.trace_id, &e);
                                #[allow(unused_variables)]
                                let close_reason = Some(CloseReason {
                                    code: CloseCode::Error,
                                    // sending the original error for audit
                                    description: Some(err_msg),
                                });

                                // audit
                                #[cfg(feature = "enterprise")]
                                if is_audit_enabled {
                                    audit(AuditMessage {
                                        user_email: user_id,
                                        org_id,
                                        _timestamp: chrono::Utc::now().timestamp(),
                                        protocol: Protocol::Ws(WsMeta {
                                            path,
                                            message_type: client_msg.get_type(),
                                            content: client_msg.to_json(),
                                            close_reason: format!(
                                                "{:#?}",
                                                close_reason.clone().unwrap()
                                            ),
                                        }),
                                    })
                                    .await;
                                }

                                let err_res = WsServerEvents::error_response(
                                    e,
                                    Some(search_req.trace_id.clone()),
                                    Some(req_id.to_string()),
                                );
                                let _ = send_message(&req_id, err_res.to_json().to_string()).await;
                                let close_reason = Some(CloseReason {
                                    code: CloseCode::Error,
                                    // Need to keep description short
                                    // `actix_ws` does not support long descriptions
                                    description: Some(format!(
                                        "[trace_id {}] Search Error",
                                        search_req.trace_id.clone()
                                    )),
                                });
                                cleanup_and_close_session(&req_id, close_reason).await;
                            }
                        }
                    });

                    // Remove the cancellation flag
                    cancellation_registry_cache_utils::remove_cancellation_flag(&trace_id);

                    drop(task);
                }
                #[cfg(feature = "enterprise")]
                WsClientEvents::Cancel { trace_id } => {
                    // Set to cancellation flag for the given trace_id
                    cancellation_registry_cache_utils::set_cancellation_flag(&trace_id);
                    log::info!(
                        "[WS_HANDLER]: trace_id: {}, Cancellation flag set to: {}",
                        trace_id,
                        cancellation_registry_cache_utils::is_cancelled(&trace_id)
                    );

                    let res = search::handle_cancel(&trace_id, org_id).await;
                    // close the session if send_message failed
                    let _ = send_message(req_id, res.to_json().to_string()).await;
                    let close_reason = Some(CloseReason {
                        code: CloseCode::Normal,
                        description: Some(format!("[trace_id {}] Search canceled", trace_id)),
                    });
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
                        description: Some(format!("[id {}] benchmark completed", id)),
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
                description: Some(format!("[req_id {}] Request Error", req_id)),
            });
            let mut session = if let Some(session) = sessions_cache_utils::get_session(req_id) {
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
    let mut session = if let Some(session) = sessions_cache_utils::get_session(req_id) {
        session
    } else {
        log::error!("[WS_HANDLER]: req_id: {} session not found", req_id);
        return Err(Error::Message(format!(
            "[req_id {}] session not found",
            req_id
        )));
    };
    session.text(msg).await.map_err(|e| {
        log::error!("[WS_HANDLER]: Failed to send message: {:?}", e);
        Error::Message(e.to_string())
    })
}

async fn cleanup_and_close_session(req_id: &str, close_reason: Option<CloseReason>) {
    if let Some(mut session) = sessions_cache_utils::get_session(req_id) {
        if let Some(reason) = &close_reason {
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
