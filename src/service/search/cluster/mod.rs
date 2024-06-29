// Copyright 2024 Zinc Labs Inc.
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

use std::{cmp::min, io::Cursor, sync::Arc};

use ::datafusion::arrow::{datatypes::Schema, ipc, record_batch::RecordBatch};
use async_recursion::async_recursion;
use config::{
    cluster::{is_ingester, is_querier},
    get_config,
    meta::{
        cluster::{Node, Role},
        search::{self, ScanStats},
        stream::{
            FileKey, PartitionTimeLevel, QueryPartitionStrategy, StreamPartition, StreamType,
        },
    },
    metrics,
    utils::{inverted_index::split_token, json},
};
use hashbrown::{HashMap, HashSet};
use infra::{
    dist_lock,
    errors::{Error, ErrorCodes, Result},
    schema::{unwrap_partition_time_level, unwrap_stream_settings},
};
use itertools::Itertools;
use proto::cluster_rpc;
use tonic::{
    codec::CompressionEncoding,
    metadata::{MetadataKey, MetadataValue},
    transport::Channel,
    Request,
};
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
    common::infra::cluster as infra_cluster,
    handler::http::request::websocket::ws_utils::{
        WebSocketMessage, WebSocketMessageType, WEBSOCKET_MSG_CHAN,
    },
    service::file_list,
};

pub mod cacher;
pub mod grpc;
pub mod http;
#[cfg(feature = "enterprise")]
pub mod super_cluster;

#[async_recursion]
#[tracing::instrument(
    name = "service:search:cluster:run",
    skip_all,
    fields(trace_id = req.job.as_ref().unwrap().trace_id, org_id = req.org_id)
)]
pub async fn search(
    trace_id: &str,
    meta: Arc<super::sql::Sql>,
    mut req: cluster_rpc::SearchRequest,
) -> Result<(HashMap<String, Vec<RecordBatch>>, ScanStats, usize, bool)> {
    let start = std::time::Instant::now();

    let cfg = get_config();
    // if the request is a super cluster request, then forward it to the super cluster service
    let is_final_phase = req.stype != cluster_rpc::SearchType::SuperCluster as i32;
    let stream_type = StreamType::from(req.stream_type.as_str());
    if req.timeout == 0 {
        req.timeout = cfg.limit.query_timeout as i64;
    }

    // get user_id
    #[cfg(feature = "enterprise")]
    let user_id = req.user_id.clone();
    #[cfg(feature = "enterprise")]
    let user_id = user_id.as_deref();

    // get nodes from cluster
    let mut nodes = infra_cluster::get_cached_online_query_nodes()
        .await
        .unwrap();
    // sort nodes by node_id this will improve hit cache ratio
    nodes.sort_by(|a, b| a.grpc_addr.cmp(&b.grpc_addr));
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    nodes.sort_by_key(|x| x.id);
    let nodes = nodes;

    let querier_num = nodes.iter().filter(|node| is_querier(&node.role)).count();
    if querier_num == 0 {
        log::error!("no querier node online");
        return Err(Error::Message("no querier node online".to_string()));
    }

    // partition request, here plus 1 second, because division is integer, maybe
    // lose some precision
    let job = cluster_rpc::Job {
        trace_id: trace_id.to_string(),
        job: trace_id[0..6].to_string(), // take the frist 6 characters as job id
        stage: 0,
        partition: 0,
    };

    let is_inverted_index = cfg.common.inverted_index_enabled && !meta.fts_terms.is_empty();

    log::info!(
        "[trace_id {trace_id}] search: is_agg_query {:?} is_inverted_index {:?}",
        !req.aggs.is_empty(),
        is_inverted_index
    );

    // stream settings
    let stream_settings = unwrap_stream_settings(&meta.schema).unwrap_or_default();
    let partition_time_level =
        unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);

    // If the query is of type inverted index and this is not an aggregations request
    let file_list = if is_inverted_index && req.aggs.is_empty() {
        let mut idx_req = req.clone();

        // Get all the unique terms which the user has searched.
        let terms = meta
            .fts_terms
            .iter()
            .map(|t| {
                let tokens = split_token(t, &cfg.common.inverted_index_split_chars);
                tokens
                    .into_iter()
                    .max_by_key(|key| key.len())
                    .unwrap_or_default()
            })
            .collect::<HashSet<String>>();

        let search_condition = terms
            .iter()
            .map(|v| format!("term LIKE '%{v}%'"))
            .collect::<Vec<_>>()
            .join(" OR ");

        let query = format!(
            "SELECT file_name, term, _count, _timestamp, deleted FROM \"{}\" WHERE {}",
            meta.stream_name, search_condition
        );

        // fast_mode is for 1st page optimization
        //  1. single WHERE clause of `match_all()`
        //  2. size > 0: hits equal to size (https://github.com/openobserve/openobserve/pull/3658)
        let fast_mode = (matches!(meta.meta.selection, Some(sqlparser::ast::Expr::Function(_)))
            && idx_req.query.as_ref().unwrap().size > 0);
        idx_req.stream_type = StreamType::Index.to_string();
        idx_req.query.as_mut().unwrap().sql = query;
        idx_req.query.as_mut().unwrap().sql_mode = "full".to_string();
        idx_req.query.as_mut().unwrap().from = 0; // from 0 to get all the results from index anyway.
        idx_req.query.as_mut().unwrap().size = 99999;
        idx_req.query.as_mut().unwrap().uses_zo_fn = false;
        idx_req.query.as_mut().unwrap().track_total_hits = false;
        idx_req.query.as_mut().unwrap().query_context = "".to_string();
        idx_req.query.as_mut().unwrap().query_fn = "".to_string();
        idx_req.aggs.clear();

        let idx_resp: search::Response = http::search(idx_req).await?;
        // get deleted file
        let deleted_files = idx_resp
            .hits
            .iter()
            .filter_map(|hit| {
                if hit.get("deleted").unwrap().as_bool().unwrap() {
                    Some(hit.get("file_name").unwrap().as_str().unwrap().to_string())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        let unique_files = if fast_mode {
            let limit_count = (meta.meta.limit + meta.meta.offset) as u64;
            let mut total_count = 0;
            let sorted_data = idx_resp
                .hits
                .iter()
                .filter_map(|hit| {
                    let term = hit.get("term").unwrap().as_str().unwrap().to_string();
                    let file_name = hit.get("file_name").unwrap().as_str().unwrap().to_string();
                    let timestamp = hit.get("_timestamp").unwrap().as_i64().unwrap();
                    let count = hit.get("_count").unwrap().as_u64().unwrap();
                    if deleted_files.contains(&file_name) {
                        None
                    } else {
                        total_count += count;
                        Some((term, file_name, count, timestamp))
                    }
                })
                .sorted_by(|a, b| Ord::cmp(&b.3, &a.3)); // Descending order of timestamp
            let mut term_map: HashMap<String, Vec<String>> = HashMap::new();
            let mut term_counts: HashMap<String, u64> = HashMap::new();

            for (term, filename, count, _timestamp) in sorted_data {
                let term = term.as_str();
                for search_term in terms.iter() {
                    if term.contains(search_term) {
                        let current_count = term_counts.entry(search_term.to_string()).or_insert(0);
                        if *current_count < limit_count {
                            *current_count += count;
                            term_map
                                .entry(search_term.to_string())
                                .or_insert_with(Vec::new)
                                .push(filename.to_string());
                        }
                    }
                }
            }
            term_map
                .into_iter()
                .flat_map(|(_, filenames)| filenames)
                .collect::<HashSet<_>>()
        } else {
            idx_resp
                .hits
                .iter()
                .filter_map(|hit| {
                    hit.get("file_name")
                        .and_then(|value| value.as_str())
                        .filter(|&name| !deleted_files.contains(name))
                        .map(String::from)
                })
                .collect::<HashSet<_>>()
        };

        let mut idx_file_list: Vec<FileKey> = vec![];
        for filename in unique_files {
            let prefixed_filename = format!(
                "files/{}/{}/{}/{}",
                meta.org_id, stream_type, meta.stream_name, filename
            );
            if let Ok(file_meta) = file_list::get_file_meta(&prefixed_filename).await {
                idx_file_list.push(FileKey {
                    key: prefixed_filename.to_string(),
                    meta: file_meta,
                    deleted: false,
                });
            }
        }
        // sorted by _timestamp
        idx_file_list.sort_by(|a, b| a.meta.min_ts.cmp(&b.meta.min_ts));
        idx_file_list
    } else {
        get_file_list(
            trace_id,
            &meta,
            stream_type,
            partition_time_level,
            &stream_settings.partition_keys,
        )
        .await
    };

    let file_list_took = start.elapsed().as_millis() as usize;
    log::info!(
        "[trace_id {trace_id}] search: get file_list time_range: {:?}, num: {}, took: {} ms",
        meta.meta.time_range,
        file_list.len(),
        file_list_took,
    );

    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&req.org_id])
        .inc();

    #[cfg(not(feature = "enterprise"))]
    let work_group: Option<String> = None;
    // 1. get work group
    #[cfg(feature = "enterprise")]
    let work_group: Option<o2_enterprise::enterprise::search::WorkGroup> = Some(
        o2_enterprise::enterprise::search::work_group::predict(&nodes, &file_list),
    );
    // 2. check concurrency
    let work_group_str = if let Some(wg) = &work_group {
        wg.to_string()
    } else {
        "global".to_string()
    };

    let locker_key = format!("/search/cluster_queue/{}", work_group_str);
    // get a cluster search queue lock
    let locker = if cfg.common.local_mode || !cfg.common.feature_query_queue_enabled {
        None
    } else {
        dist_lock::lock(&locker_key, req.timeout as u64)
            .await
            .map_err(|e| {
                metrics::QUERY_PENDING_NUMS
                    .with_label_values(&[&req.org_id])
                    .dec();
                Error::Message(e.to_string())
            })?
    };

    // check global concurrency
    #[cfg(feature = "enterprise")]
    work_group_checking(trace_id, start, &req, &work_group, &locker, None).await?;

    // check user concurrency
    #[cfg(feature = "enterprise")]
    if user_id.is_some() {
        work_group_checking(trace_id, start, &req, &work_group, &locker, user_id).await?;
    }

    // 3. process the search in the work group
    #[cfg(feature = "enterprise")]
    if let Err(e) = work_group
        .as_ref()
        .unwrap()
        .process(trace_id, user_id,) //TODO(ansrivas): Query started processing
        .await
    {
        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[&req.org_id])
            .dec();
        let _ = WEBSOCKET_MSG_CHAN.0.send(WebSocketMessage {
            user_id: user_id.unwrap().to_string(),
            payload: WebSocketMessageType::QueryProcessingStarted {
                trace_id: trace_id.to_string(),
            },
        });

        dist_lock::unlock(&locker).await?;
        return Err(Error::Message(e.to_string()));
    }
    #[cfg(feature = "enterprise")]
    if let Err(e) = dist_lock::unlock(&locker).await {
        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[&req.org_id])
            .dec();
        work_group
            .as_ref()
            .unwrap()
            .done(trace_id, user_id)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        return Err(e);
    }
    // done in the queue
    let took_wait = start.elapsed().as_millis() as usize - file_list_took;
    log::info!(
        "[trace_id {trace_id}] search: wait in queue took: {} ms",
        took_wait,
    );

    // set work_group
    req.work_group = work_group_str;

    let mut partition_files = Vec::new();
    let mut file_num = file_list.len();
    let mut partition_strategy =
        QueryPartitionStrategy::from(&cfg.common.feature_query_partition_strategy);
    if cfg.memory_cache.cache_latest_files {
        partition_strategy = QueryPartitionStrategy::FileHash;
    }
    let offset = match partition_strategy {
        QueryPartitionStrategy::FileNum => {
            if querier_num >= file_num {
                1
            } else {
                (file_num / querier_num) + 1
            }
        }
        QueryPartitionStrategy::FileSize => {
            let files = partition_file_by_bytes(&file_list, querier_num);
            file_num = files.len();
            partition_files = files;
            1
        }
        QueryPartitionStrategy::FileHash => {
            let files = partition_file_by_hash(&file_list, &nodes).await;
            file_num = files.len();
            partition_files = files;
            1
        }
    };

    log::info!(
        "[trace_id {trace_id}] search: file_list partition, time_range: {:?}, num: {file_num}, offset: {offset}",
        meta.meta.time_range
    );

    #[cfg(feature = "enterprise")]
    {
        let mut records = 0;
        let mut original_size = 0;
        let mut compressed_size = 0;
        for file in file_list.iter() {
            let file_meta = &file.meta;
            records += file_meta.records;
            original_size += file_meta.original_size;
            compressed_size += file_meta.compressed_size;
        }
        super::SEARCH_SERVER
            .add_file_stats(
                trace_id,
                file_list.len() as i64,
                records,
                original_size,
                compressed_size,
            )
            .await;
    }

    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&req.org_id])
        .dec();
    metrics::QUERY_RUNNING_NUMS
        .with_label_values(&[&req.org_id])
        .inc();

    // make cluster request
    let mut tasks = Vec::new();
    let mut offset_start: usize = 0;
    for (partition_no, node) in nodes.iter().cloned().enumerate() {
        let trace_id = trace_id.to_string();
        let mut req = req.clone();
        let mut job = job.clone();
        job.partition = partition_no as i32;
        req.job = Some(job);
        req.stype = cluster_rpc::SearchType::WalOnly as _;
        let is_querier = is_querier(&node.role);
        if is_querier {
            if offset_start < file_num {
                req.stype = cluster_rpc::SearchType::Cluster as _;
                match partition_strategy {
                    QueryPartitionStrategy::FileNum => {
                        req.file_list = file_list
                            [offset_start..min(offset_start + offset, file_num)]
                            .to_vec()
                            .iter()
                            .map(cluster_rpc::FileKey::from)
                            .collect();
                        offset_start += offset;
                    }
                    QueryPartitionStrategy::FileSize | QueryPartitionStrategy::FileHash => {
                        req.file_list = partition_files
                            .get(offset_start)
                            .unwrap()
                            .iter()
                            .map(|f| cluster_rpc::FileKey::from(*f))
                            .collect();
                        offset_start += offset;
                        if req.file_list.is_empty() {
                            if is_ingester(&node.role) {
                                req.stype = cluster_rpc::SearchType::WalOnly as _;
                            } else {
                                continue; // no need more querier
                            }
                        }
                    }
                };
            } else if !is_ingester(&node.role) {
                continue; // no need more querier
            }
        }

        let req_files = req.file_list.len();
        let node_addr = node.grpc_addr.clone();
        let grpc_span = info_span!(
            "service:search:cluster:grpc_search",
            trace_id,
            org_id = req.org_id,
            node_id = node.id,
            node_addr = node_addr.as_str(),
        );

        #[cfg(feature = "enterprise")]
        let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
        #[cfg(feature = "enterprise")]
        if super::SEARCH_SERVER
            .insert_sender(&trace_id, abort_sender)
            .await
            .is_err()
        {
            log::info!(
                "[trace_id {trace_id}] search->grpc: search canceled before call search->grpc"
            );
            work_group
                .as_ref()
                .unwrap()
                .done(&trace_id, user_id)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
            return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!(
                "[trace_id {trace_id}] search->grpc: search canceled before call search->grpc"
            ))));
        }

        let task = tokio::task::spawn(
            async move {
                let cfg = config::get_config();
                let org_id: MetadataValue<_> = req
                    .org_id
                    .parse()
                    .map_err(|_| Error::Message("invalid org_id".to_string()))?;
                let org_id_string = req.org_id.clone();
                let mut request = tonic::Request::new(req);
                // request.set_timeout(Duration::from_secs(cfg.grpc.timeout));

                opentelemetry::global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(
                        &tracing::Span::current().context(),
                        &mut super::MetadataMap(request.metadata_mut()),
                    )
                });

                log::info!("[trace_id {trace_id}] search->grpc: request node: {}, is_querier: {}, files: {req_files}", &node_addr, is_querier);

                let org_header_key: MetadataKey<_> = cfg
                .grpc
                .org_header_key
                .parse()
                .map_err(|_| Error::Message("invalid org_header_key".to_string()))?;
                let token: MetadataValue<_> = infra_cluster::get_internal_grpc_token()
                    .parse()
                    .map_err(|_| Error::Message("invalid token".to_string()))?;
                let channel = Channel::from_shared(node_addr)
                    .unwrap()
                    .connect_timeout(std::time::Duration::from_secs(cfg.grpc.connect_timeout))
                    .connect()
                    .await
                    .map_err(|err| {
                        log::error!("[trace_id {trace_id}] search->grpc: node: {}, connect err: {:?}", &node.grpc_addr, err);
                        super::server_internal_error("connect search node error")
                    })?;
                let mut client = cluster_rpc::search_client::SearchClient::with_interceptor(
                    channel,
                    move |mut req: Request<()>| {
                        req.metadata_mut().insert("authorization", token.clone());
                        req.metadata_mut()
                            .insert(org_header_key.clone(), org_id.clone());
                        Ok(req)
                    },
                );
                client = client
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip)
                    .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
                    .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);
                let response;
                tokio::select! {
                    result = client.search(request) => {
                        match result {
                            Ok(res) => response = res.into_inner(),
                            Err(err) => {
                                log::error!("[trace_id {trace_id}] search->grpc: node: {}, search err: {:?}", &node.grpc_addr, err);
                                if err.code() == tonic::Code::DeadlineExceeded {
                                    metrics::QUERY_TIMEOUT_NUMS
                                        .with_label_values(&[&org_id_string])
                                        .inc();
                                }
                                if err.code() == tonic::Code::Internal {
                                    let err = ErrorCodes::from_json(err.message())?;
                                    return Err(Error::ErrorCode(err));
                                }
                                return Err(super::server_internal_error("search node error"));
                            }
                        }
                    }
                    _ = async {
                        #[cfg(feature = "enterprise")]
                        let _ = abort_receiver.await;
                        #[cfg(not(feature = "enterprise"))]
                        futures::future::pending::<()>().await;
                    } => {
                        log::info!("[trace_id {trace_id}] search->grpc: cancel search in node: {:?}", &node.grpc_addr);
                        return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!("[trace_id {trace_id}] search->grpc: search canceled"))));
                    }
                }

                log::info!(
                    "[trace_id {trace_id}] search->grpc: response node: {}, is_querier: {}, total: {}, took: {} ms, files: {}, scan_size: {}",
                    &node.grpc_addr,
                    is_querier,
                    response.total,
                    response.took,
                    response.scan_stats.as_ref().unwrap().files,
                    response.scan_stats.as_ref().unwrap().original_size,
                );
                Ok((node.clone(),response))
            }
            .instrument(grpc_span),
        );
        tasks.push(task);
    }

    let mut results = Vec::new();
    let mut succeed = 0;
    let mut last_error = None;
    for task in tasks {
        match task.await {
            Ok(res) => match res {
                Ok(res) => {
                    succeed += 1;
                    results.push(res);
                }
                Err(err) => {
                    results.push((
                        Node::default(),
                        cluster_rpc::SearchResponse {
                            is_partial: true,
                            ..Default::default()
                        },
                    ));
                    log::error!("[trace_id {trace_id}] search->grpc: node search error: {err}");
                    last_error = Some(err);
                }
            },
            Err(e) => {
                // search done, release lock
                #[cfg(not(feature = "enterprise"))]
                dist_lock::unlock(&locker).await?;
                #[cfg(feature = "enterprise")]
                {
                    work_group
                        .as_ref()
                        .unwrap()
                        .done(trace_id, user_id)
                        .await
                        .map_err(|e| Error::Message(e.to_string()))?;
                }
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    e.to_string(),
                )));
            }
        }
    }
    if succeed == 0 || results.iter().map(|(_, v)| v.total).sum::<i64>() == 0 {
        if let Some(err) = last_error {
            #[cfg(not(feature = "enterprise"))]
            dist_lock::unlock(&locker).await?;
            #[cfg(feature = "enterprise")]
            {
                work_group
                    .as_ref()
                    .unwrap()
                    .done(trace_id, user_id)
                    .await
                    .map_err(|e| Error::Message(e.to_string()))?;
            }
            return Err(err);
        }
    }

    let (merge_batches, scan_stats, is_partial) =
        match merge_grpc_result(trace_id, meta.clone(), results, is_final_phase).await {
            Ok(v) => v,
            Err(e) => {
                // search done, release lock
                #[cfg(not(feature = "enterprise"))]
                dist_lock::unlock(&locker).await?;
                #[cfg(feature = "enterprise")]
                {
                    work_group
                        .as_ref()
                        .unwrap()
                        .done(trace_id, user_id)
                        .await
                        .map_err(|e| Error::Message(e.to_string()))?;
                }
                return Err(e);
            }
        };
    log::info!("[trace_id {trace_id}] final merge task finish");

    // search done, release lock
    #[cfg(not(feature = "enterprise"))]
    dist_lock::unlock(&locker).await?;
    #[cfg(feature = "enterprise")]
    {
        work_group
            .as_ref()
            .unwrap()
            .done(trace_id, user_id)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok((merge_batches, scan_stats, took_wait, is_partial))
}

#[cfg(feature = "enterprise")]
#[tracing::instrument(name = "work_group:checking", skip_all, fields(user_id = user_id))]
async fn work_group_checking(
    trace_id: &str,
    start: std::time::Instant,
    req: &cluster_rpc::SearchRequest,
    work_group: &Option<o2_enterprise::enterprise::search::WorkGroup>,
    locker: &Option<dist_lock::Locker>,
    user_id: Option<&str>,
) -> Result<()> {
    let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
    if super::SEARCH_SERVER
        .insert_sender(trace_id, abort_sender)
        .await
        .is_err()
    {
        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[&req.org_id])
            .dec();
        dist_lock::unlock(locker).await?;
        log::warn!("[trace_id {trace_id}] search->cluster: request canceled before enter queue");
        return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!(
            "[trace_id {trace_id}] search->cluster: request canceled before enter queue"
        ))));
    }
    tokio::select! {
        res = work_group_need_wait(trace_id, start, req, work_group, user_id) => {
            match res {
                Ok(_) => {
                    return Ok(());
                },
                Err(e) => {
                    metrics::QUERY_PENDING_NUMS
                        .with_label_values(&[&req.org_id])
                        .dec();
                    dist_lock::unlock(locker).await?;
                    return Err(e);
                }
            }
        }
        _ = async {
            let _ = abort_receiver.await;
        } => {
            metrics::QUERY_PENDING_NUMS
                .with_label_values(&[&req.org_id])
                .dec();
            dist_lock::unlock(locker).await?;
            log::warn!("[trace_id {trace_id}] search->cluster: waiting in queue was canceled");
            return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!("[trace_id {trace_id}] search->cluster: waiting in queue was canceled"))));
        }
    }
}

#[cfg(feature = "enterprise")]
#[tracing::instrument(name = "work_group:need_wait", skip_all, fields(user_id = user_id))]
async fn work_group_need_wait(
    trace_id: &str,
    start: std::time::Instant,
    req: &cluster_rpc::SearchRequest,
    work_group: &Option<o2_enterprise::enterprise::search::WorkGroup>,
    user_id: Option<&str>,
) -> Result<()> {
    loop {
        if start.elapsed().as_millis() as u64 >= req.timeout as u64 * 1000 {
            metrics::QUERY_TIMEOUT_NUMS
                .with_label_values(&[&req.org_id])
                .inc();
            return Err(Error::Message(format!(
                "[trace_id {trace_id}] search: request timeout in queue"
            )));
        }
        match work_group.as_ref().unwrap().need_wait(user_id).await {
            Ok(true) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            Ok(false) => {
                return Ok(());
            }
            Err(e) => {
                return Err(Error::Message(e.to_string()));
            }
        }
    }
}

async fn merge_grpc_result(
    trace_id: &str,
    sql: Arc<super::sql::Sql>,
    results: Vec<(Node, cluster_rpc::SearchResponse)>,
    is_final_phase: bool,
) -> Result<(HashMap<String, Vec<RecordBatch>>, ScanStats, bool)> {
    // merge multiple instances data
    let mut scan_stats = search::ScanStats::new();
    let mut batches: HashMap<String, Vec<RecordBatch>> = HashMap::new();
    let mut is_partial = false;
    for (_, resp) in results {
        if resp.is_partial {
            is_partial = true;
            continue;
        }
        scan_stats.add(&resp.scan_stats.as_ref().unwrap().into());
        // handle hits
        let value = batches.entry("query".to_string()).or_default();
        if !resp.hits.is_empty() {
            let buf = Cursor::new(resp.hits);
            let reader = ipc::reader::FileReader::try_new(buf, None).unwrap();
            let batch = reader
                .into_iter()
                .map(std::result::Result::unwrap)
                .collect::<Vec<_>>();
            value.extend(batch);
        }
        // handle aggs
        for agg in resp.aggs {
            if !agg.hits.is_empty() {
                let buf = Cursor::new(agg.hits);
                let reader = ipc::reader::FileReader::try_new(buf, None).unwrap();
                let batch = reader
                    .into_iter()
                    .map(std::result::Result::unwrap)
                    .collect::<Vec<_>>();
                let value = batches.entry(format!("agg_{}", agg.name)).or_default();
                value.extend(batch);
            }
        }
    }

    // convert select field to schema::Field
    let select_fields = sql
        .meta
        .fields
        .iter()
        .filter_map(|f| {
            sql.schema
                .field_with_name(f)
                .ok()
                .map(|f| Arc::new(f.clone()))
        })
        .collect::<Vec<_>>();

    // merge all batches
    let mut merge_batches = HashMap::new();
    for (name, batch) in batches {
        let (merge_sql, select_fields) = if name == "query" {
            (sql.origin_sql.clone(), select_fields.clone())
        } else {
            let agg_name = name.strip_prefix("agg_").unwrap();
            (sql.aggs.get(agg_name).unwrap().0.clone(), vec![])
        };

        #[cfg(feature = "enterprise")]
        let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
        #[cfg(feature = "enterprise")]
        if super::SEARCH_SERVER
            .insert_sender(trace_id, abort_sender)
            .await
            .is_err()
        {
            let err = format!(
                "[trace_id {trace_id}] search->grpc: search canceled after get result from remote node"
            );
            log::error!("{}", err);
            return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(err)));
        }

        let merge_batch;
        tokio::select! {
            res = super::datafusion::exec::merge(
                &sql.org_id,
                sql.meta.offset,
                sql.meta.limit,
                &merge_sql,
                &batch,
                &select_fields,
                is_final_phase,
                sql.meta.subquery.is_some()
            ) => {
                match res {
                    Ok(res) => merge_batch = res,
                    Err(err) => {
                        return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                            err.to_string(),
                        )));
                    }
                }
            }
            _ = async {
                #[cfg(feature = "enterprise")]
                let _ = abort_receiver.await;
                #[cfg(not(feature = "enterprise"))]
                futures::future::pending::<()>().await;
            } => {
                log::info!("[trace_id {trace_id}] search->cluster: final merge task is cancel");
                return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!("[trace_id {trace_id}] search->cluster: final merge task is cancel"))));
            }
        }

        merge_batches.insert(name, merge_batch);
    }
    Ok((merge_batches, scan_stats, is_partial))
}

#[tracing::instrument(skip(sql), fields(org_id = sql.org_id, stream_name = sql.stream_name))]
pub(crate) async fn get_file_list(
    _trace_id: &str,
    sql: &super::sql::Sql,
    stream_type: StreamType,
    time_level: PartitionTimeLevel,
    partition_keys: &[StreamPartition],
) -> Vec<FileKey> {
    let is_local = get_config().common.meta_store_external
        || infra_cluster::get_cached_online_querier_nodes()
            .await
            .unwrap_or_default()
            .len()
            <= 1;
    let (time_min, time_max) = sql.meta.time_range.unwrap();
    let file_list = match file_list::query(
        &sql.org_id,
        &sql.stream_name,
        stream_type,
        time_level,
        time_min,
        time_max,
        is_local,
    )
    .await
    {
        Ok(file_list) => file_list,
        Err(_) => vec![],
    };

    let mut files = Vec::with_capacity(file_list.len());
    for file in file_list {
        if sql
            .match_source(&file, false, false, stream_type, partition_keys)
            .await
        {
            files.push(file.to_owned());
        }
    }
    files.sort_by(|a, b| a.key.cmp(&b.key));
    files.dedup_by(|a, b| a.key == b.key);
    files
}

pub(crate) fn partition_file_by_bytes(
    file_keys: &[FileKey],
    num_nodes: usize,
) -> Vec<Vec<&FileKey>> {
    let mut partitions: Vec<Vec<&FileKey>> = vec![Vec::new(); num_nodes];
    let sum_original_size = file_keys
        .iter()
        .map(|fk| fk.meta.original_size)
        .sum::<i64>();
    let avg_size = sum_original_size / num_nodes as i64;
    let mut node_size = 0;
    let mut node_k = 0;
    for fk in file_keys {
        node_size += fk.meta.original_size;
        if node_size >= avg_size && node_k != num_nodes - 1 && !partitions[node_k].is_empty() {
            node_size = fk.meta.original_size;
            node_k += 1;
            partitions[node_k].push(fk);
            continue;
        }
        partitions[node_k].push(fk);
    }
    partitions
}

pub(crate) async fn partition_file_by_hash<'a>(
    file_keys: &'a [FileKey],
    nodes: &'a [Node],
) -> Vec<Vec<&'a FileKey>> {
    let mut node_idx = HashMap::with_capacity(nodes.len());
    let mut idx = 0;
    for node in nodes {
        if !is_querier(&node.role) {
            continue;
        }
        node_idx.insert(&node.uuid, idx);
        idx += 1;
    }
    let mut partitions: Vec<Vec<&FileKey>> = vec![Vec::new(); idx];
    for fk in file_keys {
        let node_uuid = infra_cluster::get_node_from_consistent_hash(&fk.key, &Role::Querier)
            .await
            .expect("there is no querier node in consistent hash ring");
        let idx = node_idx.get(&node_uuid).unwrap_or(&0);
        partitions[*idx].push(fk);
    }
    partitions
}

fn handle_table_response(
    schema: Arc<Schema>,
    sources: Vec<json::Value>,
) -> (Vec<String>, Vec<json::Value>) {
    let columns = schema
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect::<Vec<_>>();
    let mut table = Vec::with_capacity(sources.len());
    for row in &sources {
        let mut new_row = Vec::with_capacity(columns.len());
        let row = row.as_object().unwrap();
        for column in &columns {
            let value = match row.get(column) {
                Some(v) => v.to_owned(),
                None => json::Value::Null,
            };
            new_row.push(value);
        }
        table.push(json::Value::Array(new_row));
    }
    (columns, table)
}

fn handle_metrics_response(sources: Vec<json::Value>) -> Vec<json::Value> {
    // handle metrics response
    let mut results_metrics: HashMap<String, json::Value> = HashMap::with_capacity(16);
    let mut results_values: HashMap<String, Vec<[json::Value; 2]>> = HashMap::with_capacity(16);
    let cfg = get_config();
    for source in sources {
        let fields = source.as_object().unwrap();
        let mut key = Vec::with_capacity(fields.len());
        fields.iter().for_each(|(k, v)| {
            if *k != cfg.common.column_timestamp && k != "value" {
                key.push(format!("{k}_{v}"));
            }
        });
        let key = key.join("_");
        if !results_metrics.contains_key(&key) {
            let mut fields = fields.clone();
            fields.remove(&cfg.common.column_timestamp);
            fields.remove("value");
            results_metrics.insert(key.clone(), json::Value::Object(fields));
        }
        let entry = results_values.entry(key).or_default();
        let value = [
            fields.get(&cfg.common.column_timestamp).unwrap().to_owned(),
            json::Value::String(fields.get("value").unwrap().to_string()),
        ];
        entry.push(value);
    }

    let mut new_sources = Vec::with_capacity(results_metrics.len());
    for (key, metrics) in results_metrics {
        new_sources.push(json::json!({
            "metrics": metrics,
            "values": results_values.get(&key).unwrap(),
        }));
    }

    new_sources
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_file_by_bytes() {
        use config::meta::stream::FileMeta;

        let vec = vec![
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 256,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 256,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 100,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 256,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 1,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 256,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 200,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 30,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 90,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 256,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 5,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
            FileKey::new(
                "",
                FileMeta {
                    min_ts: -1,
                    max_ts: -1,
                    records: -1,
                    original_size: 150,
                    compressed_size: -1,
                    flattened: false,
                },
                false,
            ),
        ];
        let expected: Vec<Vec<i64>> = vec![
            vec![256, 256, 100],
            vec![256, 1, 256],
            vec![200, 30, 90, 256, 5, 150],
        ];
        let byte = partition_file_by_bytes(&vec, 3);
        for value in byte
            .iter()
            .map(|x| x.iter().map(|v| v.meta.original_size).collect::<Vec<i64>>())
            .enumerate()
        {
            assert_eq!(value.1, expected.get(value.0).unwrap().clone());
        }
    }
}
