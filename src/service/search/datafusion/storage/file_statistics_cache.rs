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

use std::{collections::VecDeque, sync::Arc};

use dashmap::DashMap;
use datafusion::{common::Statistics, execution::cache::CacheAccessor};
use object_store::{path::Path, ObjectMeta};
use once_cell::sync::Lazy;

pub static GLOBAL_CACHE: Lazy<Arc<FileStatisticsCache>> =
    Lazy::new(|| Arc::new(FileStatisticsCache::default()));

/// Collected statistics for files
/// Cache is invalided when file size or last modification has changed
pub struct FileStatisticsCache {
    statistics: DashMap<String, (ObjectMeta, Arc<Statistics>)>,
    cacher: parking_lot::Mutex<VecDeque<String>>,
    max_entries: usize,
}

impl FileStatisticsCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            statistics: DashMap::new(),
            cacher: parking_lot::Mutex::new(VecDeque::new()),
            max_entries,
        }
    }

    pub fn len(&self) -> usize {
        self.statistics.len()
    }

    fn format_key(&self, k: &Path) -> String {
        if let Some(mut p) = k.as_ref().find("/$$/") {
            if let Some(pp) = k.as_ref()[..p].find("/schema=") {
                p = pp;
            }
            k.as_ref()[p..].to_string()
        } else {
            k.to_string()
        }
    }
}

impl Default for FileStatisticsCache {
    fn default() -> Self {
        Self::new(
            config::get_config()
                .limit
                .datafusion_file_stat_cache_max_entries,
        )
    }
}

impl CacheAccessor<Path, Arc<Statistics>> for FileStatisticsCache {
    type Extra = ObjectMeta;

    /// Get `Statistics` for file location.
    fn get(&self, k: &Path) -> Option<Arc<Statistics>> {
        let k = self.format_key(k);
        self.statistics
            .get(&k)
            .map(|s| Some(s.value().1.clone()))
            .unwrap_or(None)
    }

    /// Get `Statistics` for file location. Returns None if file has changed or not found.
    fn get_with_extra(&self, k: &Path, e: &Self::Extra) -> Option<Arc<Statistics>> {
        let k = self.format_key(k);
        self.statistics
            .get(&k)
            .map(|s| {
                let (saved_meta, statistics) = s.value();
                if saved_meta.size != e.size || saved_meta.last_modified != e.last_modified {
                    // file has changed
                    None
                } else {
                    Some(statistics.clone())
                }
            })
            .unwrap_or(None)
    }

    /// Save collected file statistics
    fn put(&self, _key: &Path, _value: Arc<Statistics>) -> Option<Arc<Statistics>> {
        panic!("Put cache in FileStatisticsCache without Extra not supported.")
    }

    fn put_with_extra(
        &self,
        k: &Path,
        value: Arc<Statistics>,
        e: &Self::Extra,
    ) -> Option<Arc<Statistics>> {
        let k = self.format_key(k);
        let mut w = self.cacher.lock();
        if w.len() >= self.max_entries {
            // release 5% of the cache
            for _ in 0..(self.max_entries / 20) {
                if let Some(k) = w.pop_front() {
                    self.statistics.remove(&k);
                } else {
                    break;
                }
            }
        }
        w.push_back(k.clone());
        drop(w);
        self.statistics.insert(k, (e.clone(), value)).map(|x| x.1)
    }

    fn remove(&mut self, k: &Path) -> Option<Arc<Statistics>> {
        let k = self.format_key(k);
        self.statistics.remove(&k).map(|x| x.1 .1)
    }

    fn contains_key(&self, k: &Path) -> bool {
        let k = self.format_key(k);
        self.statistics.contains_key(&k)
    }

    fn len(&self) -> usize {
        self.statistics.len()
    }

    fn clear(&self) {
        self.statistics.clear()
    }
    fn name(&self) -> String {
        "FileStatisticsCache".to_string()
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};

    use super::*;

    #[test]
    fn test_file_statistics_cache() {
        let meta = ObjectMeta {
            location: Path::from("test"),
            last_modified: DateTime::parse_from_rfc3339("2022-09-27T22:36:00+02:00")
                .unwrap()
                .into(),
            size: 1024,
            e_tag: None,
            version: None,
        };
        let cache = FileStatisticsCache::default();
        assert!(cache.get_with_extra(&meta.location, &meta).is_none());

        cache.put_with_extra(
            &meta.location,
            Statistics::new_unknown(&Schema::new(vec![Field::new(
                "test_column",
                DataType::Timestamp(TimeUnit::Second, None),
                false,
            )]))
            .into(),
            &meta,
        );
        assert!(cache.get_with_extra(&meta.location, &meta).is_some());

        // file size changed
        let mut meta2 = meta.clone();
        meta2.size = 2048;
        assert!(cache.get_with_extra(&meta2.location, &meta2).is_none());

        // file last_modified changed
        let mut meta2 = meta.clone();
        meta2.last_modified = DateTime::parse_from_rfc3339("2022-09-27T22:40:00+02:00")
            .unwrap()
            .into();
        assert!(cache.get_with_extra(&meta2.location, &meta2).is_none());

        // different file
        let mut meta2 = meta;
        meta2.location = Path::from("test2");
        assert!(cache.get_with_extra(&meta2.location, &meta2).is_none());
    }
}
