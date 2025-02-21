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

pub mod cgroup;
pub mod cpu;
pub mod disk;
pub mod mem;
pub mod net;
pub mod os;

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
