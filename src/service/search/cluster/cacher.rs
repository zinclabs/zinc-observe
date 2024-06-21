use config::cluster::{is_querier, LOCAL_NODE_UUID};
use infra::errors::{Error, ErrorCodes};
use proto::cluster_rpc::{self, QueryCacheRequest};
use tonic::{codec::CompressionEncoding, metadata::MetadataValue, transport::Channel, Request};
use tracing::{info_span, Instrument};

use crate::{common::meta::search::CachedQueryResponse, service::search::infra_cluster};

pub async fn get_cached_results(
    start_time: i64,
    end_time: i64,
    is_aggregate: bool,
    query_key: String,
    file_path: String,
    trace_id: String,
    result_ts_column: String,
) -> Option<CachedQueryResponse> {
    let start = std::time::Instant::now();
    // get nodes from cluster
    let mut nodes = infra_cluster::get_cached_online_query_nodes()
        .await
        .unwrap();
    nodes.sort_by(|a, b| a.grpc_addr.cmp(&b.grpc_addr));
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    
    nodes.sort_by_key(|x| x.id);

    let local_node = infra_cluster::get_node_by_uuid(LOCAL_NODE_UUID.as_str()).await;
    nodes.retain(|node| is_querier(&node.role) && !node.uuid.eq(LOCAL_NODE_UUID.as_str()));  

    
    let querier_num = nodes.len();
    if querier_num == 0 && local_node.is_none() {
        log::error!("no querier node online");
        return None;
    };

    let mut tasks = Vec::new();
    for node in nodes {
        let cfg = config::get_config();
        let node_addr = node.grpc_addr.clone();
        let grpc_span = info_span!(
            "service:search:cluster:cacher:get_cached_results",
            trace_id,
            node_id = node.id,
            node_addr = node_addr.as_str(),
        );
        let query_key = query_key.clone();
        let file_path = file_path.clone();
        let trace_id = trace_id.clone();
        let result_ts_column = result_ts_column.clone();
        let task = tokio::task::spawn(
            async move {
                let req = QueryCacheRequest {
                    start_time,
                    end_time,
                    is_aggregate,
                    query_key,
                    file_path,
                    timestamp_col: result_ts_column,
                };

                let request = tonic::Request::new(req);

                log::info!(
                    "[trace_id {trace_id}] get_cached_results->grpc: request node: {}",
                    &node_addr
                );

                let token: MetadataValue<_> = infra_cluster::get_internal_grpc_token()
                    .parse()
                    .map_err(|_| Error::Message("invalid token".to_string()))?;
                let channel = Channel::from_shared(node_addr)
                    .unwrap()
                    .connect_timeout(std::time::Duration::from_secs(cfg.grpc.connect_timeout))
                    .connect()
                    .await
                    .map_err(|err| {
                        log::error!(
                            "[trace_id {trace_id}] get_cached_results->grpc: node: {}, connect err: {:?}",
                            &node.grpc_addr,
                            err
                        );
                        super::super::server_internal_error("connect search node error")
                    })?;
                let mut client =
                    cluster_rpc::query_cache_client::QueryCacheClient::with_interceptor(
                        channel,
                        move |mut req: Request<()>| {
                            req.metadata_mut().insert("authorization", token.clone());

                            Ok(req)
                        },
                    );
                client = client
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip)
                    .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
                    .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);
                

                let response = match client.get_cached_result(request).await {
                    Ok(res) => res.into_inner(),
                    Err(err) => {
                        log::error!(
                            "[trace_id {trace_id}] get_cached_results->grpc: node: {}, get_cached_results err: {:?}",
                            &node.grpc_addr,
                            err
                        );
                        if err.code() == tonic::Code::Internal {
                            let err = ErrorCodes::from_json(err.message())?;
                            return Err(Error::ErrorCode(err));
                        }
                        return Err(super::super::server_internal_error("querier node error"));
                    }
                };

                Ok((node.clone(), response))
            }
            .instrument(grpc_span),
        );
        tasks.push(task);
    }

    let mut results = Vec::new();

    for task in tasks {
        match task.await {
            Ok(res) => match res {
                Ok((node, node_res)) => match node_res.response {
                    Some(res) => {
                        let deltas = res
                            .deltas
                            .iter()
                            .map(|d| crate::common::meta::search::QueryDelta {
                                delta_start_time: d.delta_start_time,
                                delta_end_time: d.delta_end_time,
                                delta_removed_hits: d.delta_removed_hits,
                            })
                            .collect();
                        let cached_res:config::meta::search::Response  =  match res.cached_response {
                            Some(cached_response) => {
                                match serde_json::from_slice(&cached_response.data){
                                    Ok(v) => v,
                                    Err(e) => {
                                        log::error!(
                                            "[trace_id {trace_id}] get_cached_results->grpc: node: {}, cached_response parse error: {:?}",
                                            &node.grpc_addr,
                                            e
                                        );
                                        config::meta::search::Response::default()
                                }
                                    
                            }
                        },
                            None => {
                                log::error!(
                                    "[trace_id {trace_id}] get_cached_results->grpc: node: {}, no cached_response",
                                    &node.grpc_addr
                                );
                                config::meta::search::Response::default()
                            }
                        };

                        results.push((
                            node,
                            CachedQueryResponse {
                                cached_response: cached_res,
                                deltas,
                                has_pre_cache_delta: res.has_pre_cache_delta,
                                has_cached_data: res.has_cached_data,
                                cache_query_response: res.cache_query_response,
                                response_start_time: res.cache_start_time,
                                response_end_time: res.cache_end_time,
                                file_path: format!("{}_{}",file_path,result_ts_column),
                            },
                        ));
                    }
                    None => {
                        log::info!(
                            "[trace_id {trace_id}] get_cached_results->grpc: node: {}, no cached data",
                            &node.grpc_addr
                        );
                    }
                },
                Err(err) => {
                    log::error!(
                        "[trace_id {trace_id}] get_cached_results->grpc: node search error: {err}"
                    );
                }
            },
            Err(e) => {
                log::error!(
                    "[trace_id {trace_id}] get_cached_results->grpc: node search error: {e}"
                );
            }
        }
    }

    if let Some(local_resp) = crate::service::search::cache::cacher::get_cached_results(
        start_time,
        end_time,
        is_aggregate,
        &query_key,
        &file_path,
        result_ts_column,
    )
    .await
    {
        results.push((local_node.unwrap(), local_resp));
    }

    let cfg = config::get_config();
    match results
        .iter()
        .filter(|(_, cache_meta)| {
            // to make sure there is overlap between cache time range and query time range &
            // cache can at least serve query_cache_min_contribution

            let cached_duration = cache_meta.response_end_time - cache_meta.response_start_time;
            let query_duration = end_time - start_time;

            cached_duration > query_duration / cfg.limit.query_cache_min_contribution
                && cache_meta.response_start_time <= end_time
                && cache_meta.response_end_time >= start_time
        })
        .max_by_key(|(_, result)| {
            result.response_end_time.min(end_time) - result.response_start_time.max(start_time)
        }) {
        Some((node, result)) => {
            log::info!(
                "[trace_id {trace_id}] get_cached_results->grpc: node: {}, get cached result took {} ms",
                &node.grpc_addr,
                start.elapsed().as_millis()
            );
            Some(result.clone())
        }
        None => None,
    }
}
