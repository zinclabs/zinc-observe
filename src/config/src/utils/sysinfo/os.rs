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

pub fn get_os_name() -> String {
    sysinfo::System::name().unwrap_or("unknown".to_string())
}

pub fn get_os_version() -> String {
    sysinfo::System::os_version().unwrap_or("unknown".to_string())
}

pub fn get_hostname() -> String {
    sysinfo::System::host_name().unwrap_or("unknown".to_string())
}
