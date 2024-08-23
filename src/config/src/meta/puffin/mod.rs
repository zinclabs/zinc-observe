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

/// puffin specs constants
pub const MAGIC: [u8; 4] = [0x50, 0x46, 0x41, 0x31];
pub const MAGIC_SIZE: u64 = MAGIC.len() as u64;
pub const MIN_FILE_SIZE: u64 = MAGIC_SIZE + MIN_FOOTER_SIZE;
pub const FLAGS_SIZE: u64 = 4;
pub const PAYLOAD_SIZE_SIZE: u64 = 4;
pub const MIN_FOOTER_SIZE: u64 = MAGIC_SIZE + FLAGS_SIZE + PAYLOAD_SIZE_SIZE + MAGIC_SIZE; // without any blobs

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PuffinFooterFlags: u32 {
        const DEFAULT = 0b00000000;
        const COMPRESSED_LZ4 = 0b00000001;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// The offset in the file where the blob contents start
    pub offset: u64,

    /// The length of the blob stored in the file (after compression, if compressed)
    pub length: u64,
}

impl BlobMetadata {
    pub fn builder() -> BlobMetadataBuilder {
        BlobMetadataBuilder::default()
    }
}

#[derive(Default)]
pub struct BlobMetadataBuilder {
    offset: Option<u64>,
    length: Option<u64>,
}

impl BlobMetadataBuilder {
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn length(mut self, length: u64) -> Self {
        self.length = Some(length);
        self
    }

    pub fn build(self) -> Result<BlobMetadata, &'static str> {
        Ok(BlobMetadata {
            offset: self.offset.ok_or("offset is required")?,
            length: self.length.ok_or("length is required")?,
        })
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

impl PuffinMeta {
    pub fn builder() -> PuffinMetaBuilder {
        PuffinMetaBuilder::default()
    }
}

#[derive(Default)]
pub struct PuffinMetaBuilder {
    blob_metadata: Vec<BlobMetadata>,
    properties: HashMap<String, String>,
}

impl PuffinMetaBuilder {
    pub fn blob_metadata(mut self, blob_metadata: Vec<BlobMetadata>) -> Self {
        self.blob_metadata = blob_metadata;
        self
    }

    pub fn add_blob_metadata(mut self, blob_metadata: BlobMetadata) -> Self {
        self.blob_metadata.push(blob_metadata);
        self
    }

    pub fn properties(mut self, properties: HashMap<String, String>) -> Self {
        self.properties = properties;
        self
    }

    pub fn add_property(mut self, key: String, value: String) -> Self {
        self.properties.insert(key, value);
        self
    }

    pub fn build(self) -> PuffinMeta {
        PuffinMeta {
            blob_metadata: self.blob_metadata,
            properties: self.properties,
        }
    }
}

// /// Puffin Blob metadata. [Puffin specs](https://iceberg.apache.org/puffin-spec/#filemetadata)
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[serde(rename_all = "lowercase")]
// pub struct BlobMetadata {
//     /// Blob type
//     #[serde(rename = "type")]
//     pub blob_type: String,

//     /// Required by specs. Not used within OpenObserve
//     #[serde(default)]
//     pub fields: Vec<i32>,

//     /// Required by specs. Not used within OpenObserve
//     #[serde(default)]
//     pub snapshot_id: i64,

//     /// Required by specs. Not used within OpenObserve
//     #[serde(default)]
//     pub sequence_number: i64,

//     /// The offset in the file where the blob contents start
//     pub offset: i64,

//     /// The length of the blob stored in the file (after compression, if compressed)
//     pub length: i64,

//     /// See [`CompressionCodec`]. If omitted, the data is assumed to be uncompressed.
//     #[serde(default, skip_serializing_if = "Option::is_none")]
//     pub compression_codec: Option<CompressionCodec>,

//     /// Storage for arbitrary meta-information about the blob
//     #[serde(default, skip_serializing_if = "Option::is_none")]
//     pub properties: HashMap<String, String>,
// }

// /// Puffin Blob metadata. [Puffin specs](https://iceberg.apache.org/puffin-spec/#filemetadata)
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// #[serde(rename_all = "lowercase")]
// pub enum CompressionCodec {
//     Lz4,
//     #[serde(default)]
//     Zstd,
// }
