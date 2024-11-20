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
use once_cell::sync::Lazy;

type IndexCache = (Arc<tantivy::Index>, Arc<tantivy::IndexReader>);

pub static GLOBAL_CACHE: Lazy<Arc<ReaderCache>> = Lazy::new(|| Arc::new(ReaderCache::default()));

/// Cache created for storing the inverted index readers
pub struct ReaderCache {
    readers: DashMap<String, IndexCache>,
    cacher: parking_lot::Mutex<VecDeque<String>>,
    max_entries: usize,
}

impl ReaderCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            readers: DashMap::new(),
            cacher: parking_lot::Mutex::new(VecDeque::new()),
            max_entries,
        }
    }

    pub fn len(&self) -> usize {
        self.readers.len()
    }

    pub fn get(&self, key: &str) -> Option<IndexCache> {
        self.readers.get(key).map(|r| r.value().clone())
    }

    pub fn put(&self, key: String, value: IndexCache) -> Option<IndexCache> {
        let mut w = self.cacher.lock();
        if w.len() >= self.max_entries {
            // release 5% of the cache
            for _ in 0..(self.max_entries / 20) {
                if let Some(k) = w.pop_front() {
                    self.readers.remove(&k);
                } else {
                    break;
                }
            }
        }
        w.push_back(key.clone());
        drop(w);
        self.readers.insert(key, value)
    }
}

impl Default for ReaderCache {
    fn default() -> Self {
        Self::new(config::get_config().limit.inverted_index_cache_max_entries)
    }
}
