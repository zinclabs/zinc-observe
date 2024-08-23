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
    io::{SeekFrom, Write},
    sync::Arc,
};

use anyhow::{anyhow, ensure, Result};
use fst::MapBuilder;
use futures::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use serde::{Deserialize, Serialize};

use super::puffin::writer::PuffinBytesWriter;
use crate::{
    meta::bitvec::BitVec,
    utils::inverted_index::{pack_u32_pair, unpack_u32_pair},
};

const INDEX_FILE_METAS_SIZE_SIZE: u64 = 4;

type Bytes = Vec<u8>;
type BytesRef<'a> = &'a [u8];

/// Tracks and consumes [`ColumnIndexMeta`] after each selected column within a parquet file
/// is indexed via [`ColumnIndexer`].
/// The aggregated metas is then compressed and written to buffer for writing to file system.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IndexFileMetas {
    #[serde(default)]
    pub metas: HashMap<String, ColumnIndexMeta>,
}

impl IndexFileMetas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes aggregated [`ColumnIndexMeta`] into writer and compresses all written bytes.
    /// Returns compressed data to write into file system
    pub fn finish(&self, mut writer: Vec<u8>) -> Result<Option<(Vec<u8>, u64)>> {
        if self.metas.is_empty() {
            return Ok(None);
        }
        let meta_bytes = serde_json::to_vec(&self)?;
        writer.write_all(&meta_bytes)?;
        let metas_size = meta_bytes.len() as u32;
        writer.write_all(&metas_size.to_le_bytes())?;
        writer.flush()?;
        let original_size = writer.len() as u64;

        let mut puffin_buf: Vec<u8> = Vec::new();
        let mut puffin_writer = PuffinBytesWriter::new(&mut puffin_buf);
        puffin_writer.add_blob(writer)?;
        puffin_writer.finish()?;

        Ok(Some((puffin_buf, original_size)))
    }
}

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
    // the minimum term length indexed in this column
    #[serde(default = "default_min_len")]
    pub min_len: usize,
    // the maximum term length indexed in this column
    #[serde(default)]
    pub max_len: usize,
}

const fn default_min_len() -> usize {
    usize::MAX
}

/// Reader to parse decompressed raw bytes read from file system into memory for the search
/// interface
pub struct IndexReader<R> {
    source: R,
}

impl<R: AsyncRead + AsyncSeek + Unpin + Send> IndexReader<R> {
    pub fn new(source: R) -> Self {
        Self { source }
    }

    /// Reads and parse the bytes read from source to construct [`IndexFileMetas`].
    /// IndexFileMetas is used to find and read ColumnIndex for a particular column.
    pub async fn metadata(&mut self) -> Result<Arc<IndexFileMetas>> {
        let end_offset = self.source.seek(SeekFrom::End(0)).await?;

        // read index_size
        let index_file_metas_size_offset = SeekFrom::Start(end_offset - INDEX_FILE_METAS_SIZE_SIZE);
        self.source.seek(index_file_metas_size_offset).await?;
        let index_file_metas_size_buf = &mut [0u8; INDEX_FILE_METAS_SIZE_SIZE as usize];
        self.source.read_exact(index_file_metas_size_buf).await?;
        let index_file_metas_size = u32::from_le_bytes(*index_file_metas_size_buf) as u64;

        // read index_file_metas
        let index_file_metas_offset =
            SeekFrom::Start(end_offset - INDEX_FILE_METAS_SIZE_SIZE - index_file_metas_size);
        self.source.seek(index_file_metas_offset).await?;
        let index_file_metas_buf = &mut vec![0u8; index_file_metas_size as usize];
        self.source.read_exact(index_file_metas_buf).await?;

        let index_file_metas: IndexFileMetas = serde_json::from_slice(index_file_metas_buf)?;
        Self::validate_meta(&index_file_metas, index_file_metas_size, end_offset)?;

        Ok(Arc::new(index_file_metas))
    }

    pub async fn fst(&mut self, offset: u64, size: u32) -> Result<fst::Map<Vec<u8>>> {
        let fst_buf = self.seek_read(offset, size).await?;
        fst::Map::new(fst_buf).map_err(|e| {
            anyhow!(
                "Error constructing FST Map from read bytes {}",
                e.to_string()
            )
        })
    }

    pub async fn get_bitmap(
        &mut self,
        column_index_meta: &ColumnIndexMeta,
        fst_val: u64,
    ) -> Result<BitVec> {
        let (relative_offset, size) = unpack_u32_pair(fst_val);
        self.bitmap(column_index_meta.base_offset + relative_offset as u64, size)
            .await
    }

    async fn bitmap(&mut self, offset: u64, size: u32) -> Result<BitVec> {
        self.seek_read(offset, size).await.map(BitVec::from_vec)
    }

    async fn seek_read(&mut self, offset: u64, size: u32) -> Result<Vec<u8>> {
        self.source.seek(SeekFrom::Start(offset)).await?;
        let mut buf = vec![0u8; size as usize];
        self.source.read_exact(&mut buf).await?;
        Ok(buf)
    }
}

impl<R> IndexReader<R> {
    fn validate_meta(
        index_file_metas: &IndexFileMetas,
        index_file_metas_size: u64,
        end_offset: u64,
    ) -> Result<()> {
        for col_meta in index_file_metas.metas.values() {
            let ColumnIndexMeta {
                base_offset,
                index_size,
                ..
            } = col_meta;

            let limit = end_offset - INDEX_FILE_METAS_SIZE_SIZE - index_file_metas_size;
            ensure!(
                *base_offset + *index_size <= limit,
                anyhow!(
                    "ColumnIndexMeta unexpected offset: {} and size: {}. IndexFileMetas size {}",
                    base_offset,
                    index_size,
                    index_file_metas_size
                )
            );
        }
        Ok(())
    }
}

/// An automaton that matches if the input contains to a specific string.
///
/// ```rust
/// extern crate fst;
///
/// use fst::{automaton::Contains, Automaton, IntoStreamer, Set, Streamer};
///
/// # fn main() { example().unwrap(); }
/// fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let paths = vec!["/home/projects/bar", "/home/projects/foo", "/tmp/foo"];
///     let set = Set::from_iter(paths)?;
///
///     // Build our contains query.
///     let keyword = Contains::new("/projects");
///
///     // Apply our query to the set we built.
///     let mut stream = set.search(keyword).into_stream();
///
///     let matches = stream.into_strs()?;
///     assert_eq!(matches, vec!["/home/projects/bar", "/home/projects/foo"]);
///     Ok(())
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Contains<'a> {
    string: &'a [u8],
}

impl<'a> Contains<'a> {
    /// Constructs automaton that matches a part of string.
    #[inline]
    pub fn new(string: &'a str) -> Contains<'a> {
        Self {
            string: string.as_bytes(),
        }
    }
}

impl<'a> fst::automaton::Automaton for Contains<'a> {
    type State = Option<usize>;

    #[inline]
    fn start(&self) -> Option<usize> {
        Some(0)
    }

    #[inline]
    fn is_match(&self, pos: &Option<usize>) -> bool {
        pos.is_some() && pos.unwrap() >= self.string.len()
    }

    #[inline]
    fn can_match(&self, pos: &Option<usize>) -> bool {
        pos.is_some()
    }

    #[inline]
    fn accept(&self, pos: &Option<usize>, byte: u8) -> Option<usize> {
        // if we aren't already past the end...
        if let Some(pos) = *pos {
            // and there is still a matching byte at the current position...
            if self.string.get(pos).cloned() == Some(byte) {
                // then move forward
                return Some(pos + 1);
            } else if pos >= self.string.len() {
                // if we're past the end, then we're done
                return Some(i32::MAX as usize);
            } else {
                return Some(0);
            }
        }
        // otherwise we're either past the end or didn't match the byte
        None
    }
}

/// Currently supports two InvertedIndexFormat
/// Parquet -> v2
/// FST     -> v3
/// BOTH    -> use both
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum InvertedIndexFormat {
    #[default]
    Parquet,
    FST,
    Both,
}

impl From<&String> for InvertedIndexFormat {
    fn from(s: &String) -> Self {
        match s.to_lowercase().as_str() {
            "fst" => InvertedIndexFormat::FST,
            "both" => InvertedIndexFormat::Both,
            _ => InvertedIndexFormat::Parquet,
        }
    }
}

impl std::fmt::Display for InvertedIndexFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvertedIndexFormat::Parquet => write!(f, "parquet"),
            InvertedIndexFormat::FST => write!(f, "fst"),
            InvertedIndexFormat::Both => write!(f, "both"),
        }
    }
}
