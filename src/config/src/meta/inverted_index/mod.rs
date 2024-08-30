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

pub mod reader;
pub mod search;
pub mod writer;

use std::{collections::HashMap, io::Write};

use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    /// Returns the length of bytes written to writer.
    pub fn finish(&self, writer: &mut Vec<u8>) -> Result<u64> {
        if self.metas.is_empty() {
            return Ok(0u64);
        }
        let original_size = writer.len() as u64;
        let meta_bytes = serde_json::to_vec(&self)?;
        writer.write_all(&meta_bytes)?;
        let metas_size = meta_bytes.len() as u32;
        writer.write_all(&metas_size.to_le_bytes())?;
        writer.flush()?;
        let new_size = writer.len() as u64;

        Ok(new_size - original_size)
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
    // the minimum value (alphabetically) indexed in this column.
    #[serde(default)]
    pub min_val: Bytes,
    // the maximum value (alphabetically) indexed in this column.
    #[serde(default)]
    pub max_val: Bytes,
}

const fn default_min_len() -> usize {
    usize::MAX
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
