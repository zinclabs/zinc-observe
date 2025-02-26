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

use std::sync::Arc;

use ::datafusion::datasource::TableProvider;
use config::{
    get_config,
    meta::{search::ScanStats, stream::StreamType},
};
use infra::errors::{Error, Result};

pub mod flight;
pub mod storage;
pub mod wal;

pub type SearchTable = Result<(Vec<Arc<dyn TableProvider>>, ScanStats)>;

#[derive(Debug)]
pub struct QueryParams {
    pub trace_id: String,
    pub org_id: String,
    pub stream_type: StreamType,
    pub stream_name: String,
    pub time_range: Option<(i64, i64)>,
    pub work_group: Option<String>,
    pub use_inverted_index: bool,
}

fn check_memory_circuit_breaker(trace_id: &str, scan_stats: &ScanStats) -> Result<()> {
    let cfg = get_config();
    let scan_size = if scan_stats.compressed_size > 0 {
        scan_stats.compressed_size
    } else {
        scan_stats.original_size
    };
    let cur_memory = config::utils::sysinfo::get_memory_usage();
    if cur_memory > 0 {
        // left memory < datafusion * breaker_ratio and scan_size >=  left memory
        let left_mem = cfg.limit.mem_total - cur_memory;
        if (left_mem
            < (cfg.memory_cache.datafusion_max_size * cfg.common.memory_circuit_breaker_ratio
                / 100))
            && (scan_size >= left_mem as i64)
        {
            let err = format!(
                "fire memory_circuit_breaker, try to alloc {} bytes, now current memory usage is {} bytes, left memory {} bytes, left memory more than limit of [{} bytes] or scan_size more than left memory , please submit a new query with a short time range",
                scan_size,
                cur_memory,
                left_mem,
                cfg.memory_cache.datafusion_max_size * cfg.common.memory_circuit_breaker_ratio
                    / 100
            );
            log::warn!("[circuit_breaker {trace_id}] {}", err);
            return Err(Error::Message(err.to_string()));
        }
    }
    Ok(())
}
