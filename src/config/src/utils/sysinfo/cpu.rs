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

use sysinfo::{CpuRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind};

// Get number of CPUs
pub fn get_cpu_num() -> usize {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_list(CpuRefreshKind::nothing());
    system.cpus().len()
}

// Get average CPU usage
pub fn get_cpu_usage() -> f32 {
    let mut system = sysinfo::System::new_with_specifics(
        RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
    );
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();
    let cpus = system.cpus();
    if cpus.is_empty() {
        return 0.0;
    }
    let total_usage = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>();
    total_usage / cpus.len() as f32
}

// Get process CPU usage
#[allow(dead_code)]
pub fn get_process_cpu_usage() -> f32 {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0.0;
    };
    let mut system = sysinfo::System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_cpu()),
    );
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), false);
    system
        .process(pid)
        .map(|p| p.cpu_usage())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysinfo_get_cpu_num() {
        assert!(get_cpu_num() > 0);
    }

    #[test]
    fn test_sysinfo_get_cpu_usage() {
        assert!(get_cpu_usage() > 0.0);
    }

    #[test]
    fn test_sysinfo_get_process_cpu_usage() {
        assert!(get_process_cpu_usage() >= 0.0);
    }
}
