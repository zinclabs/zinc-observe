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

use std::{collections::BTreeMap, io::Write};

use fst::MapBuilder;

use super::{Bytes, BytesRef, ColumnIndexMeta, Result};
use crate::{meta::bitvec::BitVec, utils::inverted_index::pack_u32_pair};

/// Given the column data within a parquet file, [`ColumnIndexer`] indexes a `term`
/// to the `SegmentIDs` that term appears in within the file.
/// 1. Maps all terms to their corresponding SegmentIDs:
///    - terms are lexicographically sorted via a [`BTreeMap`]
///    - SegmentIDs(= row_id / SEGMENT_LENGTH) are represented as a BitVec<u8>
///    - {term: bitmap} is mapped through an [`FSTMap`], (Finite State Transducers)
/// 2. Writes bitmaps, fst_map, and meta into in-memory buffer
///
/// ```text
/// components of the resulting buffer of one ColumnIndexer
/// ┌───────────────────────────────────────────────────────────────┐
/// │ bitmap0 | bitmap1 | ... | fst_bytes | column_index_meta_bytes │
/// └───────────────────────────────────────────────────────────────┘
/// ```
pub struct ColumnIndexer {
    sorter: BTreeMap<Bytes, BitVec>,
    fst: MapBuilder<Vec<u8>>,
    meta: ColumnIndexMeta,
}

impl ColumnIndexer {
    pub fn new() -> Self {
        Self {
            sorter: BTreeMap::new(),
            fst: MapBuilder::memory(),
            meta: ColumnIndexMeta::default(),
        }
    }

    /// Pushes the bytes of a term that is being indexed onto the [`BTreeMap`] and its segment_id
    /// for sorting purposes.
    /// It term always in the map, update its bitmap with the new segment_id
    pub fn push(&mut self, value: BytesRef<'_>, segment_id: usize, term_len: usize) {
        let bitmap = self.sorter.entry(value.into()).or_default();
        if segment_id >= bitmap.len() {
            bitmap.resize(segment_id + 1, false);
        }
        bitmap.set(segment_id, true);

        // update min_max timestamp
        if term_len < self.meta.min_len {
            self.meta.min_len = term_len;
        }
        if term_len > self.meta.max_len {
            self.meta.max_len = term_len;
        }
    }

    /// Appends all inserted value-bitmap pairs into buffer and constructs fst_map and
    /// writes constructed fst_map bytes into buffer. Return finalized column_index_meta data
    pub fn write(mut self, writer: &mut Vec<u8>) -> Result<ColumnIndexMeta> {
        // Get the min and max value from the sorter, and update the meta
        let min_val = self.sorter.keys().next().cloned();
        let max_val = self.sorter.keys().next_back().cloned();
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
        self.meta.min_val = min_val.unwrap_or_default();
        self.meta.max_val = max_val.unwrap_or_default();

        Ok(self.meta)
    }

    pub fn is_empty(&self) -> bool {
        self.sorter.is_empty()
    }

    /// 1. writes all inserted value-bitmap pairs into buffer
    /// 2. maps each inserted value to their BitVec's (offset, size) within writer buffer to the
    ///    [`FSTMap`]
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

impl Default for ColumnIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn check_col_indexer_min_max() {
        let mut indexer = super::ColumnIndexer::new();
        indexer.meta.min_len = std::usize::MAX;
        indexer.push("a".as_bytes(), 0, 1);
        indexer.push("b".as_bytes(), 0, 1);
        indexer.push("c".as_bytes(), 0, 1);
        indexer.push("a".as_bytes(), 1, 1);
        indexer.push("b".as_bytes(), 1, 1);
        indexer.push("c".as_bytes(), 1, 1);
        indexer.push("a".as_bytes(), 2, 1);
        indexer.push("b".as_bytes(), 2, 1);
        indexer.push("c".as_bytes(), 2, 1);

        let mut writer = Vec::new();
        let meta = indexer.write(&mut writer).unwrap();
        assert_eq!(meta.min_len, 1);
        assert_eq!(meta.max_len, 1);
        assert_eq!(meta.min_val, "a".as_bytes());
        assert_eq!(meta.max_val, "c".as_bytes());
    }

    #[test]
    fn check_col_indexer_empty() {
        let indexer = super::ColumnIndexer::new();
        assert!(indexer.is_empty());
    }
}
