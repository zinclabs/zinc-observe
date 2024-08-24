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

use std::io::{Read, SeekFrom};

use anyhow::{anyhow, ensure, Result};
use futures::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use super::{
    BlobMetadata, PuffinFooterFlags, PuffinMeta, FLAGS_SIZE, FOOTER_PAYLOAD_SIZE_SIZE, MAGIC,
    MAGIC_SIZE,
};
use crate::meta::puffin::{CompressionCodec, MIN_FILE_SIZE};

pub struct PuffinBytesReader<R> {
    source: R,

    metadata: Option<PuffinMeta>,
}

impl<R> PuffinBytesReader<R> {
    pub fn new(source: R) -> Self {
        Self {
            source,
            metadata: None,
        }
    }
}

impl<R: AsyncRead + AsyncSeek + Unpin + Send> PuffinBytesReader<R> {
    pub async fn read_blob_bytes(&mut self, blob_metadata: &BlobMetadata) -> Result<Vec<u8>> {
        // Seek to the start of the blob
        self.source
            .seek(SeekFrom::Start(blob_metadata.offset as _))
            .await?;

        // decompress bytes since OpenObserve InvertedIndex compresses index data by default
        ensure!(
            blob_metadata.compression_codec == Some(CompressionCodec::Zstd),
            anyhow!("Unexpected CompressionCodex found in BlobMetadata")
        );
        let mut compressed = vec![0u8; blob_metadata.length as usize];
        self.source.read_exact(&mut compressed).await?;

        let mut decompressed = Vec::new();
        let mut decoder = zstd::Decoder::new(&compressed[..])?;
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    pub async fn get_metadata(&mut self) -> Result<PuffinMeta> {
        if let Some(meta) = &self.metadata {
            return Ok(meta.clone());
        }

        // check MAGIC
        let mut magic = [0u8; MAGIC_SIZE as usize];
        self.source.read_exact(&mut magic).await?;
        ensure!(magic == MAGIC, anyhow!("Header MAGIC mismatch"));

        let end_offset = self.source.seek(SeekFrom::End(0)).await?;
        ensure!(
            end_offset >= MIN_FILE_SIZE,
            anyhow!(
                "Unexpected bytes size: minimal size {} vs actual size {}",
                MIN_FILE_SIZE,
                end_offset
            )
        );

        let puffin_meta = PuffinFooterBytesReader::new(&mut self.source, end_offset)
            .parse()
            .await?;
        self.metadata = Some(puffin_meta.clone());

        Ok(puffin_meta)
    }
}

/// Footer layout: HeadMagic Payload PayloadSize Flags FootMagic
///                [4]       [?]     [4]         [4]   [4]
struct PuffinFooterBytesReader<R> {
    source: R,
    file_size: u64,
    chunk: FooterChunk,
    flags: PuffinFooterFlags,
    payload_size: u64,
    metadata: Option<PuffinMeta>,
}

impl<R> PuffinFooterBytesReader<R> {
    fn new(source: R, file_size: u64) -> Self {
        Self {
            source,
            file_size,
            chunk: FooterChunk::FootMagic,
            flags: PuffinFooterFlags::empty(),
            payload_size: 0,
            metadata: None,
        }
    }
}

impl<R: AsyncRead + AsyncSeek + Unpin + Send> PuffinFooterBytesReader<R> {
    async fn parse(mut self) -> Result<PuffinMeta> {
        let mut buf = Vec::new();
        while let Some((offset, size)) = self.next_chunk_to_read() {
            self.source.seek(SeekFrom::Start(offset)).await?;
            let size = size as usize;
            buf.resize(size, 0);
            let buf = &mut buf[..size];
            self.source.read_exact(buf).await?;
            self.parse_chunk(buf)?;
        }

        ensure!(
            self.chunk == FooterChunk::Finished,
            anyhow!("Error parsing Puffin footer from bytes")
        );

        Ok(self.metadata.unwrap())
    }

    fn next_chunk_to_read(&mut self) -> Option<(u64, u64)> {
        match self.chunk {
            FooterChunk::FootMagic => Some((self.footer_magic_offset(), MAGIC_SIZE)),
            FooterChunk::Flags => Some((self.flags_offset(), FLAGS_SIZE)),
            FooterChunk::PayloadSize => {
                Some((self.payload_size_offset(), FOOTER_PAYLOAD_SIZE_SIZE))
            }
            FooterChunk::Payload => Some((self.payload_offset(), self.payload_size)),
            FooterChunk::HeadMagic => Some((self.head_magic_offset(), MAGIC_SIZE)),
            FooterChunk::Finished => None,
        }
    }

    fn parse_chunk(&mut self, bytes: &[u8]) -> Result<()> {
        match self.chunk {
            FooterChunk::FootMagic => {
                ensure!(bytes == MAGIC, anyhow!("Head Magic mismatch"));
                self.chunk = FooterChunk::Flags;
            }
            FooterChunk::Flags => {
                let flag_bits = u32::from_le_bytes(bytes.try_into()?);
                self.flags = PuffinFooterFlags::from_bits_truncate(flag_bits);
                self.chunk = FooterChunk::PayloadSize;
            }
            FooterChunk::PayloadSize => {
                let size = i32::from_le_bytes(bytes.try_into()?);
                ensure!(
                    size >= 0,
                    anyhow!("Unexpected footer payload size {size}. Should be non-negative")
                );
                self.payload_size = size as _;
                self.chunk = FooterChunk::Payload;
            }
            FooterChunk::Payload => {
                self.metadata = Some(self.parse_payload(bytes)?);
                self.validate_payload()?;
                self.chunk = FooterChunk::HeadMagic;
            }
            FooterChunk::HeadMagic => {
                ensure!(bytes == MAGIC, anyhow!("Magic Mismatch"));
                self.chunk = FooterChunk::Finished;
            }
            FooterChunk::Finished => {}
        }
        Ok(())
    }

    fn parse_payload(&self, bytes: &[u8]) -> Result<PuffinMeta> {
        if self.flags.contains(PuffinFooterFlags::COMPRESSED_ZSTD) {
            let decoder = zstd::Decoder::new(bytes)?;
            serde_json::from_reader(decoder)
                .map_err(|e| anyhow!("Error decompress footer payload {}", e.to_string()))
        } else {
            serde_json::from_slice(bytes)
                .map_err(|e| anyhow!("Error decompress footer payload {}", e.to_string()))
        }
    }

    fn validate_payload(&self) -> Result<()> {
        let puffin_metadata = self.metadata.as_ref().expect("metadata is not set");

        let mut offset = MAGIC_SIZE;
        for blob in &puffin_metadata.blob_metadata {
            ensure!(
                blob.offset as u64 == offset,
                anyhow!("Blob payload offset mismatch")
            );
            offset += blob.length as u64;
        }

        let payload_ends_at = puffin_metadata
            .blob_metadata
            .last()
            .map_or(MAGIC_SIZE, |blob| (blob.offset + blob.length) as u64);

        ensure!(
            payload_ends_at == self.head_magic_offset(),
            anyhow!("Payload chunk offset mismatch")
        );
        Ok(())
    }

    fn footer_magic_offset(&self) -> u64 {
        self.file_size - MAGIC_SIZE
    }

    fn flags_offset(&self) -> u64 {
        self.footer_magic_offset() - FLAGS_SIZE
    }

    fn payload_size_offset(&self) -> u64 {
        self.flags_offset() - FOOTER_PAYLOAD_SIZE_SIZE
    }

    fn payload_offset(&self) -> u64 {
        self.payload_size_offset() - self.payload_size
    }

    fn head_magic_offset(&self) -> u64 {
        self.payload_offset() - MAGIC_SIZE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FooterChunk {
    FootMagic,
    Flags,
    PayloadSize,
    Payload,
    HeadMagic,
    Finished,
}
