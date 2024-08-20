// Copyright 2024 Zinc Labs Inc.
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

use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
};

use anyhow::Result;
use fst::MapBuilder;
use serde::{Deserialize, Serialize};

use crate::{
    meta::bitvec::BitVec,
    utils::inverted_index::{pack_u32_pair, unpack_u32_pair},
};

type Bytes = Vec<u8>;
type BytesRef<'a> = &'a [u8];

/// Tracks and consumes ColumnIndexMeta after selected columns within a parquet file
/// is indexed via ColumnIndexer.
/// The aggregated metas is then written to file
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FileIndexer {
    #[serde(default)]
    pub metas: HashMap<String, ColumnIndexMeta>,
}

impl FileIndexer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes aggregated ColumnIndexMeta into writer and records total length
    pub fn finish(&self, writer: &mut Vec<u8>) -> Result<()> {
        let meta_bytes = serde_json::to_vec(&self.metas)?;
        writer.write_all(&meta_bytes)?;
        let total_size = meta_bytes.len() as u32;
        writer.write_all(&total_size.to_le_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

/// Given the column data within a parquet file, ColumnIndexer indexes a `term`
/// to the `SegmentIDs` that term appears in within the file.
/// 1. Maps all terms to their corresponding SegmentIDs:
///   a. terms are lexicographically sorted via a BTreeMap
///   b. SegmentIDs(= row_id / SEGMENT_LENGTH) are represented as a BitVec<u8>
///   c. {term: bitmap} is mapped through an FSTMap, (Finite State Transducers)
/// 2. Writes bitmaps, fst_map, and meta into in-memory buffer
///
/// ```text
/// components of the resulting buffer of one ColumnIndexer
/// ┌───────────────────────────────────────────────────────────────┐
/// │ bitvec0 | bitvec1 | ... | fst_bytes | column_index_meta_bytes │
/// └───────────────────────────────────────────────────────────────┘
/// ```
pub struct ColumnIndexer {
    pub sorter: BTreeMap<Bytes, BitVec>,
    pub fst: MapBuilder<Vec<u8>>,
    pub meta: ColumnIndexMeta,
}

impl ColumnIndexer {
    pub fn new() -> Self {
        Self {
            sorter: BTreeMap::new(),
            fst: MapBuilder::memory(),
            meta: ColumnIndexMeta::default(),
        }
    }

    /// Pushes the bytes of a term that is being indexed onto the BTreeMap and its segment_id
    /// for sorting purposes.
    /// It term always in the map, update its bitmap with the new segment_id
    pub fn push(&mut self, value: BytesRef<'_>, segment_id: usize) {
        let bitmap = self.sorter.entry(value.into()).or_default();
        if segment_id >= bitmap.len() {
            bitmap.resize(segment_id + 1, false);
        }
        bitmap.set(segment_id, true);
    }

    /// Appends all inserted value-bitmap pairs into buffer and constructs fst_map and
    /// writes constructed fst_map bytes into buffer. Return finalized column_index_meta data
    pub fn write(mut self, writer: &mut Vec<u8>) -> Result<ColumnIndexMeta> {
        // 1. write bitmaps to writer
        for (value, bitmap) in std::mem::take(&mut self.sorter) {
            self.append_value(value, bitmap, writer)?;
        }

        // 2. writes fst into writer buffer
        let fst_bytes = self.fst.into_inner()?;
        writer.write_all(&fst_bytes)?;

        // update meta
        self.meta.relative_fst_offset = self.meta.index_size as _;
        self.meta.fst_size = fst_bytes.len() as _;
        self.meta.index_size += self.meta.fst_size as u64;

        Ok(self.meta)
    }

    /// 1. writes all inserted value-bitmap pairs into buffer
    /// 2. maps each inserted value to their BitVec's (offset, size) within writer buffer to the
    ///    FSTMap
    fn append_value(&mut self, value: Bytes, bitmap: BitVec, writer: &mut Vec<u8>) -> Result<()> {
        // 1
        let bitmap_bytes = bitmap.into_vec();
        writer.write_all(&bitmap_bytes)?;

        // 2
        let offset = self.meta.index_size as u32;
        let size = bitmap_bytes.len() as u32;

        let packed = pack_u32_pair(offset, size);
        self.fst.insert(value, packed)?;

        // update meta
        self.meta.index_size += size as u64;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ColumnIndexMeta {
    // base byte offset for this column index date within the index file (multiple column indices)
    #[serde(default)]
    pub base_offset: u64,
    // total byte size of this column index date
    #[serde(default)]
    pub index_size: u64,
    // fst bytes offset relative to the `base_offset`
    #[serde(default)]
    pub relative_fst_offset: u32,
    // total byte size of fst bytes
    #[serde(default)]
    pub fst_size: u32,
    // the minimum timestamp found within this column
    #[serde(default)]
    pub min_ts: u32,
    // the maximum timestamp found within this column
    #[serde(default)]
    pub max_ts: u32,
}
