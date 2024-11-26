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
pub const FOOTER_SIZE: u64 = MAGIC_SIZE + FLAGS_SIZE + FOOTER_PAYLOAD_SIZE_SIZE;
pub const MIN_FOOTER_SIZE: u64 = MAGIC_SIZE + FLAGS_SIZE + FOOTER_PAYLOAD_SIZE_SIZE + MAGIC_SIZE; // without any blobs

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PuffinFooterFlags: u32 {
        const DEFAULT = 0b00000000;
        const COMPRESSED = 0b00000001;
    }
}

/// Metadata of a Puffin file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuffinMeta {
    /// Metadata for each blob in the file
    pub blobs: Vec<BlobMetadata>,

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
    pub blob_type: BlobTypes,

    /// Required by specs
    #[serde(default)]
    pub fields: Vec<u32>,

    /// Required by specs
    #[serde(default)]
    pub snapshot_id: u64,

    /// Required by specs
    #[serde(default)]
    pub sequence_number: u64,

    /// The offset in the file where the blob contents start
    pub offset: u64,

    /// The length of the blob stored in the file (after compression, if compressed)
    pub length: u64,

    /// Default to ZSTD compression for OpenObserve inverted index
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_codec: Option<CompressionCodec>,

    /// Additional meta information of the file.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, String>,
}

impl BlobMetadata {
    pub fn get_offset(&self, range: Option<core::ops::Range<usize>>) -> core::ops::Range<usize> {
        match range {
            None => self.offset as usize..(self.offset + self.length) as usize,
            Some(v) => self.offset as usize + v.start..(self.offset as usize + v.start + v.len()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionCodec {
    Lz4,
    Zstd,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobTypes {
    #[serde(rename = "apache-datasketches-theta-v1")]
    ApacheDatasketchesThetaV1,
    #[serde(rename = "deletion-vector-v1")]
    DeletionVectorV1,
    #[default]
    #[serde(rename = "o2-fst-v1")]
    O2FstV1,
    #[serde(rename = "o2-ttv-v1")]
    O2TtvV1,
    #[serde(rename = "o2-ttv-footer-v1")]
    O2TtvFooterV1,
}

#[derive(Default)]
pub struct BlobMetadataBuilder {
    blob_type: Option<BlobTypes>,
    fields: Vec<u32>,
    snapshot_id: u64,
    sequence_number: u64,
    offset: Option<u64>,
    length: Option<u64>,
    compression_codec: Option<CompressionCodec>,
    properties: HashMap<String, String>,
}

impl BlobMetadataBuilder {
    pub fn blob_type(mut self, blob_type: BlobTypes) -> Self {
        self.blob_type = Some(blob_type);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn length(mut self, length: u64) -> Self {
        self.length = Some(length);
        self
    }

    pub fn properties(mut self, properties: HashMap<String, String>) -> Self {
        self.properties = properties;
        self
    }

    pub fn build(self) -> Result<BlobMetadata, &'static str> {
        Ok(BlobMetadata {
            blob_type: self.blob_type.ok_or("blob_type is required")?,
            fields: self.fields,
            snapshot_id: self.snapshot_id,
            sequence_number: self.sequence_number,
            offset: self.offset.ok_or("offset is required")?,
            length: self.length.unwrap_or_default(),
            compression_codec: self.compression_codec,
            properties: self.properties,
        })
    }
}
