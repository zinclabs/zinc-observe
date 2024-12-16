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

use config::{
    meta::stream::{FileMeta, StreamStats, StreamType},
    RwHashMap,
};
use once_cell::sync::Lazy;

const STREAM_STATS_MEM_SIZE: usize = std::mem::size_of::<StreamStats>();
static STATS: Lazy<RwHashMap<String, StreamStats>> = Lazy::new(Default::default);

#[inline]
pub fn get_stats() -> RwHashMap<String, StreamStats> {
    STATS.clone()
}

#[inline]
pub fn get_stream_stats(org_id: &str, stream_name: &str, stream_type: StreamType) -> StreamStats {
    let key = format!("{org_id}/{stream_type}/{stream_name}");
    STATS
        .get(&key)
        .map(|v| v.value().clone())
        .unwrap_or_default()
}

#[inline]
pub fn remove_stream_stats(org_id: &str, stream_name: &str, stream_type: StreamType) {
    let key = format!("{org_id}/{stream_type}/{stream_name}");
    STATS.remove(&key);
}

#[inline]
pub fn set_stream_stats(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    val: StreamStats,
) {
    let key = format!("{org_id}/{stream_type}/{stream_name}");
    STATS.insert(key, val);
}

#[inline]
pub fn incr_stream_stats(key: &str, val: &FileMeta) -> Result<(), anyhow::Error> {
    if val.records == 0 {
        return Ok(());
    }

    // eg: files/default/logs/olympics/2022/10/03/10/6982652937134804993_1.parquet
    let columns = key.split('/').collect::<Vec<&str>>();
    if columns.len() < 9 {
        return Err(anyhow::anyhow!(
            "[incr_stream_stats] Invalid file path: {}",
            key
        ));
    }
    // let _ = columns[0];
    let org_id = columns[1];
    let stream_type = columns[2];
    let stream_name = columns[3];
    let key = format!("{org_id}/{stream_type}/{stream_name}");
    let mut stats = STATS.entry(key).or_default();
    if stats.doc_time_min > val.min_ts || stats.doc_time_min == 0 {
        stats.doc_time_min = val.min_ts;
    }
    if stats.doc_time_max < val.max_ts {
        stats.doc_time_max = val.max_ts;
    }
    stats.doc_num += val.records;
    stats.file_num += 1;
    stats.storage_size += val.original_size as f64;
    stats.compressed_size += val.compressed_size as f64;
    stats.index_size += val.index_size as f64;

    Ok(())
}

#[inline]
pub fn get_stream_stats_len() -> usize {
    STATS.len()
}

#[inline]
pub fn get_stream_stats_in_memory_size() -> usize {
    STATS
        .iter()
        .map(|v| v.key().len() + STREAM_STATS_MEM_SIZE)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stream_stats_len() {
        let stats = get_stats();
        assert_eq!(get_stream_stats_len(), stats.len());

        let val = StreamStats {
            created_at: 1667978841102,
            doc_time_min: 1667978841102,
            doc_time_max: 1667978845374,
            doc_num: 5000,
            file_num: 1,
            storage_size: 200.0,
            compressed_size: 3.0,
            index_size: 120000.0,
        };

        set_stream_stats("nexus", "default", StreamType::Logs, val.clone());
        let stats = get_stream_stats("nexus", "default", StreamType::Logs);
        assert_eq!(stats, val);

        let stats = get_stream_stats("nexus", "default", StreamType::Logs);
        assert_eq!(stats.doc_num, 5000);
    }
}
