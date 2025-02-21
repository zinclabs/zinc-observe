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

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind};

pub struct MemoryStats {
    pub total_memory: usize,
    pub used_memory: usize,
    pub free_memory: usize,
}

/// Get memory stats
pub fn get_memory_stats() -> MemoryStats {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    MemoryStats {
        total_memory: system.total_memory() as usize,
        used_memory: system.used_memory() as usize,
        free_memory: system.free_memory() as usize,
    }
}

// Get total memory in bytes
pub fn get_total_memory() -> usize {
    get_memory_stats().total_memory
}

// Get used memory in bytes
pub fn get_memory_usage() -> usize {
    get_memory_stats().used_memory
}

// Get free memory in bytes
pub fn get_free_memory() -> usize {
    get_memory_stats().free_memory
}

// Get process memory usage in bytes
#[allow(dead_code)]
pub fn get_process_memory_usage() -> usize {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0;
    };
    let mut system = sysinfo::System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_memory()),
    );
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    system
        .process(pid)
        .map(|p| p.memory() as usize)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysinfo_get_total_memory() {
        assert!(get_total_memory() > 0);
    }

    #[test]
    fn test_sysinfo_get_memory_usage() {
        assert!(get_memory_usage() > 0);
    }

    #[test]
    fn test_sysinfo_get_process_memory_usage() {
        assert!(get_process_memory_usage() > 0);
    }
}
