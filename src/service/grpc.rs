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

use std::sync::Arc;

use config::{get_config, meta::cluster::NodeInfo, utils::rand::get_rand_element, RwAHashMap};
use infra::errors::{Error, ErrorCodes};
use once_cell::sync::Lazy;
use proto::cluster_rpc::{self, metrics_client::MetricsClient, search_client::SearchClient};
use tonic::{
    codec::CompressionEncoding,
    metadata::{MetadataKey, MetadataValue},
    service::interceptor::InterceptedService,
    transport::{Certificate, Channel, ClientTlsConfig},
    Request, Status,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::search::MetadataMap;
use crate::common::infra::cluster;

static CHANNELS: Lazy<RwAHashMap<String, Channel>> = Lazy::new(Default::default);

pub(crate) async fn get_ingester_channel() -> Result<(String, Channel), tonic::Status> {
    let grpc_addr = get_rand_ingester_addr().await?;
    get_cached_channel(&grpc_addr)
        .await
        .map(|channel| (grpc_addr, channel))
}

async fn get_rand_ingester_addr() -> Result<String, tonic::Status> {
    let cfg = get_config();
    let nodes = cluster::get_cached_online_ingester_nodes().await;
    if nodes.is_none() || nodes.as_ref().unwrap().is_empty() {
        if !cfg.route.ingester_srv_url.is_empty() {
            Ok(format!(
                "http://{}:{}",
                cfg.route.ingester_srv_url, cfg.grpc.port
            ))
        } else {
            Err(tonic::Status::internal(
                "No online ingester nodes".to_string(),
            ))
        }
    } else {
        let nodes = nodes.unwrap();
        let node = get_rand_element(&nodes);
        Ok(node.grpc_addr.to_string())
    }
}

pub(crate) async fn get_cached_channel(grpc_addr: &str) -> Result<Channel, tonic::Status> {
    // if channel cache is disabled, create a new channel for each request
    if get_config().grpc.channel_cache_disabled {
        return create_channel(grpc_addr).await;
    }

    // cache hit
    let r = CHANNELS.read().await;
    if let Some(channel) = r.get(grpc_addr) {
        return Ok(channel.clone());
    }
    drop(r);

    // cache miss, connect to ingester
    let channel = create_channel(grpc_addr).await?;
    let mut w = CHANNELS.write().await;
    w.insert(grpc_addr.to_string(), channel.clone());
    drop(w);

    Ok(channel.clone())
}

async fn create_channel(grpc_addr: &str) -> Result<Channel, tonic::Status> {
    let cfg = config::get_config();
    let mut channel = Channel::from_shared(grpc_addr.to_string()).map_err(|err| {
        log::error!("gRPC node: {}, parse err: {:?}", &grpc_addr, err);
        Status::internal("parse gRPC node error".to_string())
    })?;
    if cfg.grpc.tls_enabled {
        let pem = std::fs::read_to_string(&cfg.grpc.tls_cert_path)?;
        let cert = Certificate::from_pem(pem);
        let tls = ClientTlsConfig::new()
            .ca_certificate(cert)
            .domain_name(&cfg.grpc.tls_cert_domain);
        channel = channel.tls_config(tls).map_err(|err| {
            log::error!("gRPC node: {}, tls err: {:?}", &grpc_addr, err);
            Status::internal("tls gRPC node error".to_string())
        })?;
    }
    let channel = channel
        .connect_timeout(std::time::Duration::from_secs(
            config::get_config().grpc.connect_timeout,
        ))
        .connect()
        .await
        .map_err(|err| {
            log::error!("gRPC node: {}, connect err: {:?}", &grpc_addr, err);
            Status::internal("connect to gRPC node error".to_string())
        })?;
    Ok(channel)
}

#[tracing::instrument(name = "grpc:search::make_client", skip_all)]
pub async fn make_grpc_search_client<T>(
    request: &mut Request<T>,
    node: &Arc<dyn NodeInfo>,
) -> Result<
    SearchClient<InterceptedService<Channel, impl Fn(Request<()>) -> Result<Request<()>, Status>>>,
    Error,
> {
    let cfg = get_config();
    request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));

    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(
            &tracing::Span::current().context(),
            &mut MetadataMap(request.metadata_mut()),
        )
    });

    let token: MetadataValue<_> = node
        .get_auth_token()
        .parse()
        .map_err(|_| Error::Message("invalid token".to_string()))?;
    let channel = get_cached_channel(&node.get_grpc_addr())
        .await
        .map_err(|err| {
            log::error!(
                "search->grpc: node: {}, connect err: {:?}",
                &node.get_grpc_addr(),
                err
            );
            Error::ErrorCode(ErrorCodes::ServerInternalError(err.to_string()))
        })?;
    let client = cluster_rpc::search_client::SearchClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            Ok(req)
        },
    );
    Ok(client
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip)
        .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
        .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024))
}

#[tracing::instrument(name = "promql:search:grpc:metrics:make_client", skip_all)]
pub async fn make_grpc_metrics_client<T>(
    trace_id: &str,
    org_id: &str,
    request: &mut Request<T>,
    node: &Arc<dyn NodeInfo>,
) -> Result<
    MetricsClient<InterceptedService<Channel, impl Fn(Request<()>) -> Result<Request<()>, Status>>>,
    Error,
> {
    let cfg = get_config();
    let org_id: MetadataValue<_> = org_id
        .parse()
        .map_err(|_| Error::Message(format!("invalid org_id: {}", org_id)))?;
    request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));

    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(
            &tracing::Span::current().context(),
            &mut MetadataMap(request.metadata_mut()),
        )
    });

    let org_header_key: MetadataKey<_> = cfg
        .grpc
        .org_header_key
        .parse()
        .map_err(|_| Error::Message("invalid org_header_key".to_string()))?;
    let token: MetadataValue<_> = node
        .get_auth_token()
        .parse()
        .map_err(|_| Error::Message("invalid token".to_string()))?;
    let channel = get_cached_channel(&node.get_grpc_addr())
        .await
        .map_err(|err| {
            log::error!(
                "[trace_id {trace_id}] promql->search->grpc: node: {}, connect err: {:?}",
                &node.get_grpc_addr(),
                err
            );
            Error::ErrorCode(ErrorCodes::ServerInternalError(
                "connect search node error".to_string(),
            ))
        })?;
    let mut client = cluster_rpc::metrics_client::MetricsClient::with_interceptor(
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
    Ok(client)
}
