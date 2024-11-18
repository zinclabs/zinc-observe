use actix_ws::{MessageStream, Session};
use config::{
    get_config,
    meta::{
        search::{Response, SearchPartitionRequest},
        sql::resolve_stream_names,
        stream::StreamType,
    },
    utils::sql::is_aggregate_query,
};
use futures::StreamExt;
use infra::errors::Error;
use proto::cluster_rpc::SearchQuery;
use rand::prelude::SliceRandom;
use tracing::Instrument;

use crate::{
    common::meta::{self, search::MultiCachedQueryResponse},
    handler::http::request::websocket::utils::{
        sessions_cache_utils, SearchEventReq, SearchResponseType, WsClientEvents, WsServerEvents,
    },
    service::search::{self as SearchService, cache::cacher::get_ts_col_order_by, sql::Sql},
};

pub struct SessionHandler {
    session: Session,
    msg_stream: MessageStream,
    user_id: String,
    request_id: String,
    org_id: String,
}

impl SessionHandler {
    pub fn new(
        session: Session,
        msg_stream: MessageStream,
        user_id: &str,
        request_id: &str,
        org_id: &str,
    ) -> Self {
        Self {
            session,
            msg_stream,
            user_id: user_id.to_string(),
            request_id: request_id.to_string(),
            org_id: org_id.to_string(),
        }
    }

    // Main handler method to run the session
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.msg_stream.next() => {
                    match msg {
                        Ok(actix_ws::Message::Ping(bytes)) => {
                            if self.session.pong(&bytes).await.is_err() {
                                log::info!("[WS_HANDLER]: Failed to send pong, closing session for request_id: {}", self.request_id);
                                break;
                            }
                        }
                        Ok(actix_ws::Message::Text(msg)) => {
                            log::info!("[WS_HANDLER]: Got text message for request_id: {}: {}", self.request_id, msg);
                            self.handle_text_message(msg.into()).await;
                        }
                        Ok(actix_ws::Message::Close(reason)) => {
                            log::info!("[WS_HANDLER]: Session closed for request_id: {}, reason: {:?}", self.request_id, reason);
                            break;
                        }
                        Ok(actix_ws::Message::Continuation(_)) => {
                            log::info!("[WS_HANDLER]: Continuation message received, closing session for request_id: {}", self.request_id);
                            break;
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    async fn handle_text_message(&mut self, msg: String) {
        match serde_json::from_str::<WsClientEvents>(&msg) {
            Ok(client_msg) => {
                log::debug!(
                    "[WS_HANDLER]: Received trace registrations msg: {:?}",
                    client_msg
                );
                match client_msg {
                    WsClientEvents::Search(search_req) => {
                        match self.handle_search_request(search_req.clone()).await {
                            Ok(_) => {
                                // force close the session once search is complete
                                self.cleanup().await;
                            }
                            Err(e) => {
                                log::error!(
                                    "[WS_HANDLER]: Failed to get search result for trace_id: {}, error: {:?}",
                                    search_req.trace_id,
                                    e
                                );
                            }
                        };
                    }
                    #[cfg(feature = "enterprise")]
                    WsClientEvents::Cancel { trace_id } => {
                        match self.handle_cancel(&trace_id).await {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!(
                                    "[WS_HANDLER]: Failed to get cancel search for trace_id: {}, error: {:?}",
                                    trace_id,
                                    e
                                );
                            }
                        };
                    }
                    // Benchmark message
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
                        if self.session.text(response.to_string()).await.is_err() {
                            log::error!(
                                "[WS_HANDLER]: Failed to send benchmark response for request_id: {}",
                                self.request_id
                            );
                        }
                        // force close session after benchmark
                        self.cleanup().await;
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to parse maessage incoming from ws client: {:?}, {:?}",
                    msg,
                    e
                );
                let err_res = WsServerEvents::RequestError {
                    request_id: self.request_id.clone(),
                    error: format!("{e}"),
                };
                self.send_message(err_res.to_json().to_string(), None, None, 0, 0)
                    .await
                    .unwrap();

                self.cleanup().await;
            }
        }
    }

    // Cleanup the session when it ends
    async fn cleanup(&mut self) {
        let req_id = self.request_id.clone();
        sessions_cache_utils::remove_session(&req_id);

        // close the ws session
        let session = self.session.clone();
        if let Err(e) = session.close(None).await {
            log::error!(
                "[WS_HANDLER]: Error closing session for request_id {}: {:?}",
                self.request_id,
                e
            );
        }

        log::info!(
            "[WS_HANDLER]: Session closed for request_id: {}, session_cache_len: {}",
            self.request_id,
            sessions_cache_utils::len_sessions()
        );
    }

    async fn handle_search_request(&mut self, mut req: SearchEventReq) -> Result<(), Error> {
        let cfg = config::get_config();
        let user_id = self.user_id.clone();
        let org_id = self.org_id.clone();
        let trace_id = req.trace_id.clone();
        let stream_type = req.stream_type;
        let start_time = req.payload.query.start_time;
        let end_time = req.payload.query.end_time;
        println!("start_time: {}, end_time: {}", start_time, end_time);

        // check and append search event type
        if req.payload.search_type.is_none() {
            req.payload.search_type = Some(req.search_type);
        }

        // get stream name
        let stream_names = match resolve_stream_names(&req.payload.query.sql) {
            Ok(v) => v.clone(),
            Err(e) => {
                let err_res = WsServerEvents::SearchError {
                    trace_id: trace_id.clone(),
                    error: e.to_string(),
                };
                self.send_message(
                    err_res.to_json().to_string(),
                    None,
                    None,
                    start_time,
                    end_time,
                )
                .await?;
                return Ok(());
            }
        };

        // get stream settings
        for stream_name in stream_names {
            if let Some(settings) =
                infra::schema::get_settings(&org_id, &stream_name, stream_type).await
            {
                let max_query_range = settings.max_query_range;
                if max_query_range > 0
                    && (req.payload.query.end_time - req.payload.query.start_time)
                        > max_query_range * 3600 * 1_000_000
                {
                    req.payload.query.start_time =
                        req.payload.query.end_time - max_query_range * 3600 * 1_000_000;
                    // range_error = format!(
                    //     "Query duration is modified due to query range restriction of {} hours",
                    //     max_query_range
                    // );
                    log::info!(
                        "[WS_SEARCH]: Query duration is modified due to query range restriction of {} hours",
                        max_query_range
                    );
                }
            }

            // Check permissions on stream
            #[cfg(feature = "enterprise")]
            {
                use o2_enterprise::enterprise::openfga::meta::mapping::OFGA_MODELS;

                use crate::common::{
                    infra::config::USERS,
                    utils::auth::{is_root_user, AuthExtractor},
                };

                if !is_root_user(&user_id) {
                    let user: meta::user::User =
                        USERS.get(&format!("{org_id}/{}", user_id)).unwrap().clone();
                    let stream_type_str = stream_type.to_string();

                    if user.is_external
                        && !crate::handler::http::auth::validator::check_permissions(
                            &user_id,
                            AuthExtractor {
                                auth: "".to_string(),
                                method: "GET".to_string(),
                                o2_type: format!(
                                    "{}:{}",
                                    OFGA_MODELS
                                        .get(stream_type_str.as_str())
                                        .map_or(stream_type_str.as_str(), |model| model.key),
                                    stream_name
                                ),
                                org_id: org_id.clone(),
                                bypass_check: false,
                                parent_id: "".to_string(),
                            },
                            Some(user.role),
                        )
                        .await
                    {
                        // return forbidden on websockets
                        let err_res = WsServerEvents::SearchError {
                            trace_id: trace_id.clone(),
                            error: "Unauthorized Access".to_string(),
                        };
                        self.send_message(
                            err_res.to_json().to_string(),
                            None,
                            None,
                            start_time,
                            end_time,
                        )
                        .await?;
                        return Ok(());
                    }
                    // Check permissions on stream ends
                }
            }
        }

        // handle search result size
        let req_size = if req.payload.query.size == 0 {
            cfg.limit.query_default_limit
        } else {
            req.payload.query.size
        };

        // get partitions and call search for each
        if self
            .is_partition_request(&req.payload, req.stream_type)
            .await
        {
            let c_resp = crate::service::search::cache::check_cache_v2(
                &trace_id,
                &org_id,
                stream_type,
                &req.payload,
                req.use_cache,
            )
            .await;
            if let Ok(c_resp) = c_resp {
                let local_c_resp = c_resp.clone();
                let cached_resp = local_c_resp.cached_response;
                let mut deltas = local_c_resp.deltas;
                deltas.sort();
                deltas.dedup();

                if !cached_resp.is_empty() {
                    // Initialize iterators for deltas and cached responses
                    let mut delta_iter = deltas.iter().peekable();
                    let mut cached_resp_iter = cached_resp.iter().peekable();

                    // Process cached responses and deltas in sorted order
                    while cached_resp_iter.peek().is_some() || delta_iter.peek().is_some() {
                        if let (Some(&delta), Some(cached)) =
                            (delta_iter.peek(), cached_resp_iter.peek())
                        {
                            // If the delta is before the current cached response time, fetch
                            // partitions

                            if delta.delta_end_time < cached.response_end_time {
                                let partitions = self.get_partitions(&req).await;

                                if partitions.is_empty() {
                                    return Ok(());
                                }

                                // get start_index for pagination
                                let start_idx =
                                    self.find_start_partition_idx(&partitions, req.time_offset);
                                let mut curr_res_size = 0;

                                log::info!(
                                    "[WS_SEARCH] Found {} partitions for trace_id: {}",
                                    partitions.len(),
                                    trace_id
                                );

                                // search by handling pagination
                                for (idx, &[start_time, end_time]) in
                                    partitions.iter().enumerate().skip(start_idx)
                                {
                                    let mut req = req.clone();
                                    req.payload.query.start_time = start_time;
                                    req.payload.query.end_time = end_time;

                                    if req_size != -1 {
                                        req.payload.query.size -= curr_res_size;
                                    }

                                    let search_res = self.do_search(&req).await?;
                                    curr_res_size += search_res.hits.len() as i64;

                                    if !search_res.hits.is_empty() {
                                        let ws_search_res = WsServerEvents::SearchResponse {
                                            trace_id: trace_id.clone(),
                                            results: search_res.clone(),
                                            response_type: SearchResponseType::Partition {
                                                current: idx as u64 + 1,
                                                total: partitions.len() as u64,
                                            },
                                            time_offset: end_time,
                                        };
                                        self.send_message(
                                            ws_search_res.to_json().to_string(),
                                            Some(search_res),
                                            Some(c_resp.clone()),
                                            start_time,
                                            end_time,
                                        )
                                        .await?;
                                    }

                                    // Stop if we've reached the requested result size
                                    if req_size != -1 && curr_res_size >= req_size {
                                        log::info!(
                                            "[WS_SEARCH]: Reached requested result size ({}), stopping search",
                                            req_size
                                        );
                                        break;
                                    }
                                }
                                delta_iter.next(); // Move to the next delta after processing
                            } else {
                                // Send cached response if delta is not before the cached response
                                // time
                                if !cached.cached_response.hits.is_empty() {
                                    let ws_search_res = WsServerEvents::SearchResponse {
                                        trace_id: trace_id.clone(),
                                        results: cached.cached_response.clone(),
                                        response_type: SearchResponseType::Partition {
                                            current: 1000 as u64 + 1,
                                            total: 1000 as u64,
                                        },
                                        time_offset: cached.response_end_time,
                                    };
                                    self.send_message(
                                        ws_search_res.to_json().to_string(),
                                        Some(cached.cached_response.clone()),
                                        Some(c_resp.clone()),
                                        start_time,
                                        end_time,
                                    )
                                    .await?;
                                }
                                cached_resp_iter.next(); // Move to the next cached response
                            }
                        } else if let Some(&delta) = delta_iter.peek() {
                            // Process remaining deltas if no more cached responses
                            let partitions = self.get_partitions(&req).await;

                            if partitions.is_empty() {
                                return Ok(());
                            }

                            let start_idx =
                                self.find_start_partition_idx(&partitions, req.time_offset);
                            let mut curr_res_size = 0;

                            log::info!(
                                "[WS_SEARCH] Found {} partitions for trace_id: {}",
                                partitions.len(),
                                trace_id
                            );

                            for (idx, &[start_time, end_time]) in
                                partitions.iter().enumerate().skip(start_idx)
                            {
                                let mut req = req.clone();
                                req.payload.query.start_time = start_time;
                                req.payload.query.end_time = end_time;

                                if req_size != -1 {
                                    req.payload.query.size -= curr_res_size;
                                }

                                let search_res = self.do_search(&req).await?;
                                curr_res_size += search_res.hits.len() as i64;

                                log::info!(
                                    "[WS_SEARCH]: Found {} hits for trace_id: {}",
                                    search_res.hits.len(),
                                    trace_id
                                );

                                if !search_res.hits.is_empty() {
                                    log::info!(
                                        "[WS_SEARCH]: Sending search response for trace_id: {}",
                                        trace_id
                                    );
                                    let ws_search_res = WsServerEvents::SearchResponse {
                                        trace_id: trace_id.clone(),
                                        results: search_res.clone(),
                                        response_type: SearchResponseType::Partition {
                                            current: idx as u64 + 1,
                                            total: partitions.len() as u64,
                                        },
                                        time_offset: end_time,
                                    };
                                    self.send_message(
                                        ws_search_res.to_json().to_string(),
                                        Some(search_res),
                                        Some(c_resp.clone()),
                                        start_time,
                                        end_time,
                                    )
                                    .await?;
                                }

                                // Stop if we've reached the requested result size
                                if req_size != -1 && curr_res_size >= req_size {
                                    log::info!(
                                        "[WS_SEARCH]: Reached requested result size ({}), stopping search",
                                        req_size
                                    );
                                    break;
                                }
                            }
                            delta_iter.next(); // Move to the next delta after processing
                        } else if let Some(cached) = cached_resp_iter.next() {
                            // Process remaining cached responses if no more deltas
                            let ws_search_res = WsServerEvents::SearchResponse {
                                trace_id: trace_id.clone(),
                                results: cached.cached_response.clone(),
                                response_type: SearchResponseType::Partition {
                                    current: 1000 as u64 + 1,
                                    total: 1000 as u64,
                                },
                                time_offset: cached.response_end_time,
                            };
                            self.send_message(
                                ws_search_res.to_json().to_string(),
                                Some(cached.cached_response.clone()),
                                Some(c_resp.clone()),
                                start_time,
                                end_time,
                            )
                            .await?;
                        }
                    }
                } else {
                    let partitions = self.get_partitions(&req.clone()).await;

                    if partitions.is_empty() {
                        return Ok(());
                    }

                    let start_idx = self.find_start_partition_idx(&partitions, req.time_offset);
                    let mut curr_res_size = 0;

                    log::info!(
                        "[WS_SEARCH] Found {} partitions for trace_id: {}",
                        partitions.len(),
                        trace_id
                    );

                    for (idx, &[start_time, end_time]) in
                        partitions.iter().enumerate().skip(start_idx)
                    {
                        let mut req = req.clone();
                        req.payload.query.start_time = start_time;
                        req.payload.query.end_time = end_time;

                        if req_size != -1 {
                            req.payload.query.size -= curr_res_size;
                        }

                        let search_res = self.do_search(&req).await?;
                        curr_res_size += search_res.hits.len() as i64;

                        if !search_res.hits.is_empty() {
                            let ws_search_res = WsServerEvents::SearchResponse {
                                trace_id: trace_id.clone(),
                                results: search_res.clone(),
                                response_type: SearchResponseType::Partition {
                                    current: idx as u64 + 1,
                                    total: partitions.len() as u64,
                                },
                                time_offset: end_time,
                            };
                            self.send_message(
                                ws_search_res.to_json().to_string(),
                                Some(search_res),
                                Some(c_resp.clone()),
                                start_time,
                                end_time,
                            )
                            .await?;
                        }

                        // Stop if we've reached the requested result size
                        if req_size != -1 && curr_res_size >= req_size {
                            log::info!(
                                "[WS_SEARCH]: Reached requested result size ({}), stopping search",
                                req_size
                            );
                            break;
                        }
                    }
                }
            }
        } else {
            // call search directly
            log::info!("[WS_SEARCH]: Single search for trace_id: {}", trace_id);
            let end_time = req.payload.query.end_time;
            let search_res = self.do_search(&req).await?;
            let ws_search_res = WsServerEvents::SearchResponse {
                trace_id: trace_id.clone(),
                results: search_res.clone(),
                response_type: SearchResponseType::Single,
                time_offset: end_time,
            };
            self.send_message(
                ws_search_res.to_json().to_string(),
                None,
                None,
                start_time,
                end_time,
            )
            .await?;
        }

        Ok(())
    }

    async fn send_message(
        &mut self,
        message: String,
        search_res: Option<Response>,
        cached_resp: Option<MultiCachedQueryResponse>,
        start_time: i64,
        end_time: i64,
    ) -> Result<(), Error> {
        if self.session.text(message).await.is_err() {
            log::error!(
                "[WS_HANDLER]: Failed to send message for request_id: {}",
                self.request_id
            );
        }
        if let Some(search_res) = search_res {
            let c_resp = cached_resp.unwrap();
            crate::service::search::cache::write_results_v2(
                &c_resp.trace_id,
                &c_resp.ts_column,
                start_time,
                end_time,
                &search_res,
                c_resp.file_path,
                c_resp.is_aggregate,
                c_resp.is_descending,
            )
            .await;
        }

        Ok(())
    }

    async fn is_partition_request(
        &self,
        req: &config::meta::search::Request,
        stream_type: StreamType,
    ) -> bool {
        let cfg = get_config();

        let query = SearchQuery {
            start_time: req.query.start_time,
            end_time: req.query.end_time,
            sql: req.query.sql.clone(),
            ..Default::default()
        };

        let sql = match Sql::new(&query, &self.org_id, stream_type).await {
            Ok(s) => s,
            Err(e) => {
                log::error!("[WS_HANDLER] Failed to create SQL query: {:?}", e);
                return false;
            }
        };

        // if there is no _timestamp field in the query, return single partition
        let is_aggregate = is_aggregate_query(&req.query.sql).unwrap_or_default();
        let res_ts_column = get_ts_col_order_by(&sql, &cfg.common.column_timestamp, is_aggregate);
        let ts_column = res_ts_column.map(|(v, _)| v);

        ts_column.is_some()
    }

    async fn get_partitions(&self, req: &SearchEventReq) -> Vec<[i64; 2]> {
        let search_payload = req.payload.clone();
        let search_partition_req = SearchPartitionRequest {
            sql: search_payload.query.sql.clone(),
            start_time: search_payload.query.start_time,
            end_time: search_payload.query.end_time,
            encoding: search_payload.encoding.clone(),
            regions: search_payload.regions.clone(),
            clusters: search_payload.clusters.clone(),
            query_fn: search_payload.query.query_fn.clone(),
        };

        let res = SearchService::search_partition(
            &req.trace_id,
            &self.org_id,
            req.stream_type,
            &search_partition_req,
        )
        .instrument(tracing::info_span!("search_partition"))
        .await;

        // get the list of partitions
        let partitions = match res {
            Ok(res) => res.partitions,
            Err(e) => {
                log::error!(
                    "[WS_HANDLER]: Failed to get partitions for trace_id: {}, error: {:?}",
                    req.trace_id,
                    e
                );
                return vec![];
            }
        };

        partitions
    }

    async fn do_search(&mut self, req: &SearchEventReq) -> Result<Response, infra::errors::Error> {
        SearchService::cache::search(
            &req.trace_id,
            &self.org_id,
            req.stream_type,
            Some(self.user_id.clone()),
            &req.payload,
            req.use_cache,
        )
        .instrument(tracing::info_span!("search"))
        .await
    }

    #[cfg(feature = "enterprise")]
    async fn handle_cancel(&mut self, trace_id: &str) -> Result<(), Error> {
        match crate::service::search::cancel_query(&self.org_id, &trace_id).await {
            Ok(ret) => {
                let res = serde_json::to_string(&ret)?;
                self.send_message(res).await?;
            }
            Err(_) => {
                let res = WsServerEvents::CancelResponse {
                    trace_id: trace_id.to_string(),
                    is_success: false,
                };
                self.send_message(res.to_json().to_string()).await?;
            }
        }

        Ok(())
    }

    fn find_start_partition_idx(&self, partitions: &[[i64; 2]], time_offset: Option<i64>) -> usize {
        if let Some(offset) = time_offset {
            for (idx, [_, end_time]) in partitions.iter().enumerate() {
                if *end_time == offset {
                    log::info!(
                        "[WS_SEARCH]: Found matching partition for time_offset: {} at index: {}",
                        offset,
                        idx
                    );
                    return idx;
                }
            }
        }

        0
    }
}
