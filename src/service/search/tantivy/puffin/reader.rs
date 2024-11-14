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

use std::sync::Arc;

use anyhow::{anyhow, ensure, Result};
use bytes::Buf;

use super::*;

#[derive(Debug)]
pub struct PuffinBytesReader {
    source: Arc<object_store::ObjectMeta>,
    metadata: Option<PuffinMeta>,
}

impl PuffinBytesReader {
    pub fn new(source: object_store::ObjectMeta) -> Self {
        Self {
            source: Arc::new(source),
            metadata: None,
        }
    }
}

impl PuffinBytesReader {
    pub async fn read_blob_bytes(
        &self,
        blob_metadata: &BlobMetadata,
        range: Option<core::ops::Range<usize>>,
    ) -> Result<bytes::Bytes> {
        let raw_data = infra::cache::storage::get_range(
            &self.source.location,
            blob_metadata.get_offset(range),
        )
        .await?;

        let decompressed = match blob_metadata.compression_codec {
            Some(CompressionCodec::Lz4) => {
                return Err(anyhow!("Lz4 compression is not supported"));
            }
            Some(CompressionCodec::Zstd) => {
                return Err(anyhow!("Zstd compression is not supported"));
            }
            None => raw_data,
        };

        Ok(decompressed)
    }

    pub async fn get_field(&mut self, field: &str) -> Result<Option<BlobMetadata>> {
        self.parse_footer().await?;
        match self.metadata.as_ref() {
            None => Err(anyhow!("Metadata not found")),
            Some(v) => Ok(v
                .blobs
                .iter()
                .find(|b| b.properties.get("blob_tag").is_some_and(|val| val == field))
                .cloned()),
        }
    }

    pub async fn get_metadata(&mut self) -> Result<Option<PuffinMeta>> {
        self.parse_footer().await?;
        Ok(self.metadata.clone())
    }

    pub async fn parse_footer(&mut self) -> Result<()> {
        if self.metadata.is_some() {
            return Ok(());
        }

        if (self.source.size as u64) < MIN_FILE_SIZE {
            return Err(anyhow!(
                "Unexpected bytes size: minimal size {} vs actual size {}",
                MIN_FILE_SIZE,
                self.source.size
            ));
        }

        // check MAGIC
        let magic =
            infra::cache::storage::get_range(&self.source.location, 0..MAGIC_SIZE as usize).await?;
        ensure!(magic.to_vec() == MAGIC, anyhow!("Header MAGIC mismatch"));

        let puffin_meta = PuffinFooterBytesReader::new(self.source.clone())
            .parse()
            .await?;
        self.metadata = Some(puffin_meta);
        Ok(())
    }
}

/// Footer layout: HeadMagic Payload PayloadSize Flags FootMagic
///                [4]       [?]     [4]         [4]   [4]
struct PuffinFooterBytesReader {
    source: Arc<object_store::ObjectMeta>,
    flags: PuffinFooterFlags,
    payload_size: u64,
    metadata: Option<PuffinMeta>,
}

impl PuffinFooterBytesReader {
    fn new(source: Arc<object_store::ObjectMeta>) -> Self {
        Self {
            source,
            flags: PuffinFooterFlags::empty(),
            payload_size: 0,
            metadata: None,
        }
    }
}

impl PuffinFooterBytesReader {
    async fn parse(mut self) -> Result<PuffinMeta> {
        // read footer
        let footer = infra::cache::storage::get_range(
            &self.source.location,
            (self.source.size - FOOTER_SIZE as usize)..self.source.size,
        )
        .await?;

        // check the footer magic
        ensure!(
            footer
                .slice((FOOTER_SIZE - MAGIC_SIZE) as usize..FOOTER_SIZE as usize)
                .to_vec()
                == MAGIC,
            anyhow!("Footer MAGIC mismatch")
        );

        // check the flags
        let mut flags = [0u8; 4];
        footer
            .slice(
                (FOOTER_SIZE - MAGIC_SIZE - FLAGS_SIZE) as usize
                    ..(FOOTER_SIZE - MAGIC_SIZE) as usize,
            )
            .copy_to_slice(&mut flags);
        self.flags = PuffinFooterFlags::from_bits(u32::from_le_bytes(flags))
            .ok_or_else(|| anyhow!("Error parsing Puffin flags from bytes"))?;

        // check the payload size
        let mut payload_size = [0u8; 4];
        footer
            .slice(0..FOOTER_PAYLOAD_SIZE_SIZE as usize)
            .copy_to_slice(&mut payload_size);
        self.payload_size = i32::from_le_bytes(payload_size) as u64;

        // read the payload
        let payload = infra::cache::storage::get_range(
            &self.source.location,
            (self.source.size
                - FOOTER_SIZE as usize
                - self.payload_size as usize
                - MAGIC_SIZE as usize)..(self.source.size - FOOTER_SIZE as usize),
        )
        .await?;

        // check the footer magic
        ensure!(
            payload.slice(0..MAGIC_SIZE as usize).to_vec() == MAGIC,
            anyhow!("Footer MAGIC mismatch")
        );

        self.metadata = Some(self.parse_payload(&payload.slice(MAGIC_SIZE as usize..))?);
        self.validate_payload()?;

        Ok(self.metadata.unwrap())
    }

    fn parse_payload(&self, bytes: &[u8]) -> Result<PuffinMeta> {
        if self.flags.contains(PuffinFooterFlags::COMPRESSED) {
            let decoder = zstd::Decoder::new(bytes)?;
            serde_json::from_reader(decoder)
                .map_err(|e| anyhow!("Error decompress footer payload {}", e.to_string()))
        } else {
            serde_json::from_slice(bytes)
                .map_err(|e| anyhow!("Error serializing footer {}", e.to_string()))
        }
    }

    fn validate_payload(&self) -> Result<()> {
        let puffin_metadata = self.metadata.as_ref().expect("metadata is not set");

        let mut offset = MAGIC_SIZE;
        for blob in &puffin_metadata.blobs {
            ensure!(
                blob.offset == offset,
                anyhow!("Blob payload offset mismatch")
            );
            offset += blob.length;
        }

        let payload_ends_at = puffin_metadata
            .blobs
            .last()
            .map_or(MAGIC_SIZE, |blob| blob.offset + blob.length);
        let footer_size = MAGIC_SIZE + self.payload_size + FOOTER_SIZE;
        ensure!(
            payload_ends_at == (self.source.size as u64 - footer_size),
            anyhow!("Payload chunk offset mismatch")
        );
        Ok(())
    }
}
