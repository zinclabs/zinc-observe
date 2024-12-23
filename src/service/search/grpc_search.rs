use config::{
    get_config,
    meta::{cluster::get_internal_grpc_token, search, stream::StreamType},
    utils::json,
};
use infra::errors::{Error, ErrorCodes};
use proto::cluster_rpc;
use rand::{seq::SliceRandom, thread_rng};
use tonic::{codec::CompressionEncoding, metadata::MetadataValue, Request};
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
    common::infra::cluster as infra_cluster,
    service::{
        grpc::get_cached_channel,
        search::{server_internal_error, MetadataMap},
    },
};

#[tracing::instrument(name = "service:search:grpc_search", skip_all)]
pub async fn grpc_search(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    user_id: Option<String>,
    in_req: &search::Request,
) -> Result<search::Response, Error> {
    let cfg = get_config();
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("search->grpc: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let mut rng = thread_rng();
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
            request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));

            opentelemetry::global::get_text_map_propagator(|propagator| {
                propagator.inject_context(
                    &tracing::Span::current().context(),
                    &mut MetadataMap(request.metadata_mut()),
                )
            });

            let token: MetadataValue<_> = get_internal_grpc_token()
                .parse()
                .map_err(|_| Error::Message("invalid token".to_string()))?;
            let channel = get_cached_channel(&node_addr).await.map_err(|err| {
                log::error!(
                    "search->grpc: node: {}, connect err: {:?}",
                    &node.grpc_addr,
                    err
                );
                server_internal_error("connect search node error")
            })?;
            let mut client = cluster_rpc::search_client::SearchClient::with_interceptor(
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
            let response = match client.search(request).await {
                Ok(res) => res.into_inner(),
                Err(err) => {
                    log::error!(
                        "search->grpc: node: {}, search err: {:?}",
                        &node.grpc_addr,
                        err
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
