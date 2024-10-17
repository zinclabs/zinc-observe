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

use std::{
    collections::HashMap,
    io::{self, Write},
    mem,
};

use anyhow::{Context, Result};

use super::{
    BlobMetadata, BlobMetadataBuilder, CompressionCodec, PuffinFooterFlags, PuffinMeta, BLOB_TYPE,
    MAGIC, MAGIC_SIZE, MIN_FOOTER_SIZE,
};

pub struct PuffinBytesWriter<W> {
    /// buffer to write to
    writer: W,

    /// The properties of the file.
    properties: HashMap<String, String>,

    /// The metadata of the blobs.
    blobs_metadata: Vec<BlobMetadata>,

    /// The number of bytes written.
    written_bytes: u64,
}

impl<W> PuffinBytesWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            properties: HashMap::new(),
            blobs_metadata: vec![],
            written_bytes: 0,
        }
    }

    fn add_blob_metadata(
        &self,
        blob_type: String,
        compression_codec: Option<CompressionCodec>,
        size: u64,
    ) -> BlobMetadata {
        BlobMetadataBuilder::default()
            .blob_type(blob_type)
            .compression_codec(compression_codec)
            .offset(self.written_bytes as _)
            .length(size as _)
            .build()
            .expect("Missing required fields")
    }
}

impl<W: io::Write> PuffinBytesWriter<W> {
    pub fn add_blob(&mut self, raw_data: Vec<u8>) -> Result<()> {
        self.add_header_if_needed()
            .context("Error writing puffin header")?;

        // compress blob raw data
        let mut encoder = zstd::Encoder::new(vec![], 3)?;
        encoder
            .write_all(&raw_data)
            .context("Error encoding blob raw data")?;
        let compressed_bytes = encoder.finish()?;
        let compressed_size = compressed_bytes.len() as u64;
        self.writer.write_all(&compressed_bytes)?;

        // add metadata for this blob
        let blob_metadata = self.add_blob_metadata(
            BLOB_TYPE.to_string(),
            Some(CompressionCodec::Zstd),
            compressed_size,
        );
        self.blobs_metadata.push(blob_metadata);
        self.written_bytes += compressed_size;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<u64> {
        self.add_header_if_needed()
            .context("Error writing puffin header")?;
        self.write_footer().context("Error writing puffin footer")?;
        self.writer.flush()?;
        Ok(self.written_bytes)
    }

    fn add_header_if_needed(&mut self) -> Result<()> {
        if self.written_bytes == 0 {
            self.writer.write_all(&MAGIC)?;
            self.written_bytes += MAGIC_SIZE;
        }
        Ok(())
    }

    fn write_footer(&mut self) -> Result<()> {
        let footer_bytes = PuffinFooterWriter::new(
            mem::take(&mut self.blobs_metadata),
            mem::take(&mut self.properties),
        )
        .into_bytes()?;

        self.writer.write_all(&footer_bytes)?;
        self.written_bytes += footer_bytes.len() as u64;
        Ok(())
    }
}

/// Footer layout: HeadMagic Payload PayloadSize Flags FootMagic
///                [4]       [?]     [4]         [4]   [4]
struct PuffinFooterWriter {
    blob_metadata: Vec<BlobMetadata>,
    file_properties: HashMap<String, String>,
}

impl PuffinFooterWriter {
    fn new(blob_metadata: Vec<BlobMetadata>, file_properties: HashMap<String, String>) -> Self {
        Self {
            blob_metadata,
            file_properties,
        }
    }

    fn into_bytes(mut self) -> Result<Vec<u8>> {
        let payload = self.get_payload()?;
        let payload_size = payload.len();

        let capacity = MIN_FOOTER_SIZE as usize + payload_size;
        let mut buf = Vec::with_capacity(capacity);

        // HeadMagic
        buf.extend_from_slice(&MAGIC);

        // payload + payload size
        buf.extend_from_slice(&payload);
        buf.extend_from_slice(&(payload_size as i32).to_le_bytes());

        // flags
        buf.extend_from_slice(&PuffinFooterFlags::COMPRESSED_ZSTD.bits().to_le_bytes());

        // FootMagic
        buf.extend_from_slice(&MAGIC);

        Ok(buf)
    }

    fn get_payload(&mut self) -> Result<Vec<u8>> {
        let file_metdadata = PuffinMeta {
            blob_metadata: mem::take(&mut self.blob_metadata),
            properties: mem::take(&mut self.file_properties),
        };

        let mut encoder = zstd::Encoder::new(vec![], 3)?;
        serde_json::to_writer(&mut encoder, &file_metdadata)?;
        Ok(encoder.finish()?)
    }
}
