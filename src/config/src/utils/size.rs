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

/// Convert bytes to human readable format
pub fn bytes_to_human_readable(bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let mut bytes = bytes;
    let mut index = 0;

    if bytes == 0.0 {
        return "0 B".to_string();
    }

    while bytes >= 1024.0 && index < units.len() - 1 {
        bytes /= 1024.0;
        index += 1;
    }

    format!("{:.2} {}", bytes, units[index])
}
