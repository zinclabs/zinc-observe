use std::sync::Arc;

use config::{
    meta::{search, stream::StreamType},
    utils::json,
};
use infra::errors::{Error, ErrorCodes};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use tracing::{info_span, Instrument};

use crate::{
    common::infra::cluster as infra_cluster,
    service::{grpc::make_grpc_search_client, search::server_internal_error},
};

#[tracing::instrument(name = "service:search:grpc_search", skip_all)]
pub async fn grpc_search(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    user_id: Option<String>,
    in_req: &search::Request,
) -> Result<search::Response, Error> {
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("search->grpc: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let mut rng = StdRng::from_entropy();
    let node = nodes.choose(&mut rng).unwrap().clone();

    // make cluster request
    let node_addr = node.grpc_addr.clone();
    let grpc_span = info_span!(
        "service:search:cluster:grpc_search",
        node_id = node.id,
        node_addr = node_addr.as_str(),
    );

    let trace_id = trace_id.to_string();
    let org_id = org_id.to_string();
    let stream_type = stream_type.to_string();
    let in_req = in_req.clone();
    let task = tokio::task::spawn(
        async move {
            let req = bytes::Bytes::from(json::to_string(&in_req)?).to_vec();
            let mut request = tonic::Request::new(proto::cluster_rpc::SearchRequest {
                trace_id: trace_id.to_string(),
                org_id: org_id.to_string(),
                stream_type: stream_type.to_string(),
                user_id: user_id.clone(),
                request: req,
            });
            let node = Arc::new(node) as _;
            let mut client = make_grpc_search_client(&mut request, &node).await?;
            let response = match client.search(request).await {
                Ok(res) => res.into_inner(),
                Err(err) => {
                    log::error!(
                        "search->grpc: node: {}, search err: {err:?}",
                        &node.get_grpc_addr(),
                    );
                    if err.code() == tonic::Code::Internal {
                        let err = ErrorCodes::from_json(err.message())?;
                        return Err(Error::ErrorCode(err));
                    }
                    return Err(server_internal_error("search node error"));
                }
            };
            Ok(response)
        }
        .instrument(grpc_span),
    );

    let response = task
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))??;
    let response = json::from_slice::<search::Response>(&response.response)?;

    Ok(response)
}

#[tracing::instrument(name = "service:search:grpc_search_partition", skip_all)]
pub async fn grpc_search_partition(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    in_req: &search::SearchPartitionRequest,
    skip_max_query_range: bool,
) -> Result<search::SearchPartitionResponse, Error> {
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("search->grpc: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let mut rng = StdRng::from_entropy();
    let node = nodes.choose(&mut rng).unwrap().clone();

    // make cluster request
    let node_addr = node.grpc_addr.clone();
    let grpc_span = info_span!(
        "service:search:cluster:grpc_search_partition",
        node_id = node.id,
        node_addr = node_addr.as_str(),
    );

    let trace_id = trace_id.to_string();
    let org_id = org_id.to_string();
    let stream_type = stream_type.to_string();
    let in_req = in_req.clone();
    let task = tokio::task::spawn(
        async move {
            let req = bytes::Bytes::from(json::to_string(&in_req)?).to_vec();
            let mut request = tonic::Request::new(proto::cluster_rpc::SearchPartitionRequest {
                trace_id: trace_id.to_string(),
                org_id: org_id.to_string(),
                stream_type: stream_type.to_string(),
                request: req,
                skip_max_query_range,
            });
            let node = Arc::new(node) as _;
            let mut client = make_grpc_search_client(&mut request, &node).await?;
            let response = match client.search_partition(request).await {
                Ok(res) => res.into_inner(),
                Err(err) => {
                    log::error!(
                        "search->grpc: node: {}, search err: {err:?}",
                        &node.get_grpc_addr(),
                    );
                    if err.code() == tonic::Code::Internal {
                        let err = ErrorCodes::from_json(err.message())?;
                        return Err(Error::ErrorCode(err));
                    }
                    return Err(server_internal_error("search node error"));
                }
            };
            Ok(response)
        }
        .instrument(grpc_span),
    );

    let response = task
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))??;
    let response = json::from_slice::<search::SearchPartitionResponse>(&response.response)?;

    Ok(response)
}
