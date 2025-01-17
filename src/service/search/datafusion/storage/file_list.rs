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

use chrono::{TimeZone, Utc};
use config::meta::{bitvec::BitVec, stream::FileKey};
use hashbrown::HashMap;
use object_store::ObjectMeta;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

type SegmentData = HashMap<String, BitVec>;

static FILES: Lazy<RwLock<HashMap<String, Vec<ObjectMeta>>>> = Lazy::new(Default::default);
static SEGMENTS: Lazy<RwLock<HashMap<String, SegmentData>>> = Lazy::new(Default::default);

pub fn get(trace_id: &str) -> Result<Vec<ObjectMeta>, anyhow::Error> {
    let data = match FILES.read().get(trace_id) {
        Some(data) => data.clone(),
        None => return Err(anyhow::anyhow!("trace_id not found: {}", trace_id)),
    };
    Ok(data)
}

pub async fn set(trace_id: &str, schema_key: &str, files: &[FileKey]) {
    let key = format!("{}/schema={}", trace_id, schema_key);
    let mut values = Vec::with_capacity(files.len());
    let mut segment_data = HashMap::new();
    for file in files {
        let modified = Utc.timestamp_nanos(file.meta.max_ts * 1000);
        let file_name = format!("/{}/$$/{}", key, file.key);
        values.push(ObjectMeta {
            location: file_name.into(),
            last_modified: modified,
            size: file.meta.compressed_size as usize,
            e_tag: None,
            version: None,
        });
        if let Some(bin_data) = file.segment_ids.as_ref() {
            segment_data.insert(file.key.clone(), bin_data.clone());
        }
    }
    FILES.write().insert(key.clone(), values);
    SEGMENTS.write().insert(key, segment_data);
}

pub fn clear(trace_id: &str) {
    // Remove all files for the given trace_id
    let r = FILES.read();
    let keys = r
        .keys()
        .filter(|x| x.starts_with(trace_id))
        .cloned()
        .collect::<Vec<_>>();
    drop(r);
    let mut w = FILES.write();
    for key in keys.iter() {
        w.remove(key);
    }
    w.shrink_to_fit();
    drop(w);

    // Remove all segment data for the given trace_id
    // here we can reuse the keys, because they are the same
    let mut w = SEGMENTS.write();
    for key in keys.iter() {
        w.remove(key);
    }
    w.shrink_to_fit();
    drop(w);
}

pub fn get_segment_ids(file_key: &str) -> Option<BitVec> {
    let (trace_id, filename) = file_key.split_once("/$$/")?;
    let r = SEGMENTS.read();
    let data = r.get(trace_id)?;
    data.get(filename).cloned()
}
