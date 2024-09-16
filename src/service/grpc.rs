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

use config::{utils::rand::get_rand_element, RwAHashMap};
use once_cell::sync::Lazy;
use tonic::{transport::Channel, Status};

use crate::common::infra::cluster;

static CHANNELS: Lazy<RwAHashMap<String, Channel>> = Lazy::new(Default::default);

pub(crate) async fn get_ingester_channel() -> Result<(String, Channel), tonic::Status> {
    let grpc_addr = get_rand_ingester_addr().await?;
    get_cached_channel(&grpc_addr)
        .await
        .map(|channel| (grpc_addr, channel))
}

async fn get_rand_ingester_addr() -> Result<String, tonic::Status> {
    let cfg = config::get_config();
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
    let channel = Channel::from_shared(grpc_addr.to_string())
        .unwrap()
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
