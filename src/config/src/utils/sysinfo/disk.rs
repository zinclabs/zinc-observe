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

pub struct DiskUsage {
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
}

pub fn get_disk_usage() -> Vec<DiskUsage> {
    let mut disks: Vec<_> = sysinfo::Disks::new_with_refreshed_list()
        .iter()
        .map(|d| DiskUsage {
            mount_point: d.mount_point().to_str().unwrap().to_string(),
            total_space: d.total_space(),
            available_space: d.available_space(),
        })
        .collect();
    disks.sort_by(|a, b| b.mount_point.cmp(&a.mount_point));
    disks
}
