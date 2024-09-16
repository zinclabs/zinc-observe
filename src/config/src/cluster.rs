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

use std::net::IpAddr;

use once_cell::sync::Lazy;

use crate::{
    get_config, ider,
    meta::cluster::{Node, NodeStatus, Role, RoleGroup},
};

pub static mut LOCAL_NODE_ID: i32 = 0;
pub static mut LOCAL_NODE_KEY_LEASE_ID: i64 = 0;
pub static mut LOCAL_NODE_STATUS: NodeStatus = NodeStatus::Prepare;
pub static LOCAL_NODE: Lazy<Node> = Lazy::new(load_local_node);

pub fn load_local_node() -> Node {
    let cfg = get_config();
    Node {
        id: 1,
        uuid: load_local_node_uuid(),
        role: load_local_node_role(),
        role_group: load_role_group(),
        name: cfg.common.instance_name.clone(),
        http_addr: format!("http://127.0.0.1:{}", cfg.http.port),
        grpc_addr: format!("http://127.0.0.1:{}", cfg.grpc.port),
        cpu_num: cfg.limit.cpu_num as u64,
        status: NodeStatus::Online,
        scheduled: true,
        broadcasted: false,
    }
}

fn load_local_node_uuid() -> String {
    ider::uuid()
}

fn load_local_node_role() -> Vec<Role> {
    get_config()
        .common
        .node_role
        .clone()
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect()
}

fn load_role_group() -> RoleGroup {
    RoleGroup::from(get_config().common.node_role_group.as_str())
}

pub fn get_local_http_ip() -> String {
    let cfg = get_config();
    if !cfg.http.addr.is_empty() {
        cfg.http.addr.clone()
    } else {
        get_local_node_ip()
    }
}

pub fn get_local_grpc_ip() -> String {
    let cfg = get_config();
    if !cfg.grpc.addr.is_empty() {
        cfg.grpc.addr.clone()
    } else {
        get_local_node_ip()
    }
}

pub fn get_local_node_ip() -> String {
    for adapter in get_if_addrs::get_if_addrs().unwrap() {
        if !adapter.is_loopback() && matches!(adapter.ip(), IpAddr::V4(_)) {
            return adapter.ip().to_string();
        }
    }
    String::new()
}

#[inline(always)]
pub fn is_offline() -> bool {
    unsafe { LOCAL_NODE_STATUS == NodeStatus::Offline }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::cluster::Role;

    #[test]
    fn test_convert_role() {
        let parse = |s: &str| s.parse::<Role>().unwrap();

        assert_eq!(parse("all"), Role::All);
        assert_eq!(parse("ingester"), Role::Ingester);
        assert_eq!(parse("querier"), Role::Querier);
        assert_eq!(parse("compactor"), Role::Compactor);
        assert_eq!(parse("router"), Role::Router);
        assert_eq!(parse("alertmanager"), Role::AlertManager);
        assert_eq!(parse("alertManager"), Role::AlertManager);
        assert_eq!(parse("AlertManager"), Role::AlertManager);
        assert!("alert_manager".parse::<Role>().is_ok());
    }

    #[test]
    fn test_load_local_node_uuid() {
        assert!(!load_local_node_uuid().is_empty());
    }

    #[test]
    fn test_get_node_ip() {
        assert!(!get_local_node_ip().is_empty());
    }
}
