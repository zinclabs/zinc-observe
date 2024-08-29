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

use std::collections::HashMap;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

pub mod reader;
pub mod writer;

/// puffin specs constants
pub const MAGIC: [u8; 4] = [0x50, 0x46, 0x41, 0x31];
pub const MAGIC_SIZE: u64 = MAGIC.len() as u64;
pub const MIN_FILE_SIZE: u64 = MAGIC_SIZE + MIN_FOOTER_SIZE;
pub const FLAGS_SIZE: u64 = 4;
pub const FOOTER_PAYLOAD_SIZE_SIZE: u64 = 4;
pub const MIN_FOOTER_SIZE: u64 = MAGIC_SIZE + FLAGS_SIZE + FOOTER_PAYLOAD_SIZE_SIZE + MAGIC_SIZE; // without any blobs
// Version 1 of the inverted index blob
pub const BLOB_TYPE: &str = "o2_inverted_index_v1";

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PuffinFooterFlags: u32 {
        const DEFAULT = 0b00000000;
        const COMPRESSED_ZSTD = 0b00000001;
    }
}

/// Metadata of a Puffin file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuffinMeta {
    /// Metadata for each blob in the file
    pub blob_metadata: Vec<BlobMetadata>,

    /// Storage for arbitrary meta-information, like writer identification/version
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, String>,
}

/// Puffin Blob metadata. [Puffin specs](https://iceberg.apache.org/puffin-spec/#filemetadata)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub struct BlobMetadata {
    /// Blob type
    #[serde(rename = "type")]
    pub blob_type: String,

    /// Required by specs. Not used for InvertedIndex within OpenObserve
    #[serde(default)]
    pub fields: Vec<i32>,

    /// Required by specs. Not used for InvertedIndex within OpenObserve
    #[serde(default)]
    pub snapshot_id: i64,

    /// Required by specs. Not used for InvertedIndex within OpenObserve
    #[serde(default)]
    pub sequence_number: i64,

    /// The offset in the file where the blob contents start
    pub offset: i64,

    /// The length of the blob stored in the file (after compression, if compressed)
    pub length: i64,

    /// Default to ZSTD compression for OpenObserve inverted index
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_codec: Option<CompressionCodec>,

    /// Additional meta information of the file. Not used for InvertedIndex within OpenObserve
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionCodec {
    Lz4,
    Zstd,
}

#[derive(Default)]
pub struct BlobMetadataBuilder {
    blob_type: Option<String>,
    fields: Vec<i32>,
    snapshot_id: i64,
    sequence_number: i64,
    offset: Option<i64>,
    length: Option<i64>,
    compression_codec: Option<CompressionCodec>,
    properties: HashMap<String, String>,
}

impl BlobMetadataBuilder {
    pub fn blob_type(mut self, blob_type: String) -> Self {
        self.blob_type = Some(blob_type);
        self
    }

    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn length(mut self, length: i64) -> Self {
        self.length = Some(length);
        self
    }

    pub fn compression_codec(mut self, compression_codec: Option<CompressionCodec>) -> Self {
        self.compression_codec = compression_codec;
        self
    }

    pub fn build(self) -> Result<BlobMetadata, &'static str> {
        Ok(BlobMetadata {
            blob_type: self.blob_type.ok_or("blob_type is required")?,
            fields: self.fields,
            snapshot_id: self.snapshot_id,
            sequence_number: self.sequence_number,
            offset: self.offset.ok_or("offset is required")?,
            length: self.length.ok_or("length is required")?,
            compression_codec: self.compression_codec,
            properties: self.properties,
        })
    }
}
