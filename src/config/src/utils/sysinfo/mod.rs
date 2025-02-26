// Copyright 2025 OpenObserve Inc.
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

use serde::{Deserialize, Serialize};

pub mod cgroup;
pub mod cpu;
pub mod disk;
pub mod mem;
pub mod net;
pub mod os;

pub use net::TcpConnState;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub cpu_total: usize,
    pub cpu_usage: f32,
    pub memory_total: usize,
    pub memory_usage: usize,
    pub tcp_conns: usize,
    pub tcp_conns_established: usize,
    pub tcp_conns_close_wait: usize,
    pub tcp_conns_time_wait: usize,
}

/// Get the node running metrics
pub fn get_node_metrics() -> NodeMetrics {
    let cpu_total = get_cpu_limit();
    let cpu_usage = get_cpu_usage();
    let memory_total = get_memory_limit();
    let memory_usage = get_memory_usage();
    let tcp_conns = get_tcp_connections();
    let tcp_conns_established = net::get_tcp_connections(Some(TcpConnState::Established));
    let tcp_conns_close_wait = net::get_tcp_connections(Some(TcpConnState::CloseWait));
    let tcp_conns_time_wait = net::get_tcp_connections(Some(TcpConnState::TimeWait));

    NodeMetrics {
        cpu_total,
        cpu_usage,
        memory_total,
        memory_usage,
        tcp_conns,
        tcp_conns_established,
        tcp_conns_close_wait,
        tcp_conns_time_wait,
    }
}

pub fn get_cpu_limit() -> usize {
    cgroup::get_cpu_limit()
}

pub fn get_memory_limit() -> usize {
    cgroup::get_memory_limit()
}

pub fn get_cpu_usage() -> f32 {
    cpu::get_process_cpu_usage()
}

pub fn get_memory_usage() -> usize {
    mem::get_process_memory_usage()
}

pub fn get_tcp_connections() -> usize {
    net::get_tcp_connections(None)
}
