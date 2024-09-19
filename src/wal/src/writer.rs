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
    fs::{create_dir_all, remove_file, File, OpenOptions},
    io::{BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
};

use byteorder::{BigEndian, WriteBytesExt};
use crc32fast::Hasher;
use snafu::ResultExt;

use crate::errors::*;

pub struct Writer {
    path: PathBuf,
    f: BufWriter<File>,
    bytes_written: usize,
    uncompressed_bytes_written: usize,
    buffer: Vec<u8>,
}

impl Writer {
    pub fn new(
        root_dir: impl Into<PathBuf>,
        org_id: &str,
        stream_type: &str,
        id: u64,
        init_size: u64,
        buffer_size: usize,
    ) -> Result<Self> {
        let path = super::build_file_path(root_dir, org_id, stream_type, id);
        create_dir_all(path.parent().unwrap()).context(FileOpenSnafu { path: path.clone() })?;
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .context(FileOpenSnafu { path: path.clone() })?;

        if init_size > 0 {
            f.set_len(init_size)
                .context(FileWriteSnafu { path: path.clone() })?;
            f.seek(SeekFrom::Start(0))
                .context(FileReadSnafu { path: path.clone() })?;
        }

        if let Err(e) = f.write_all(super::FILE_TYPE_IDENTIFIER) {
            _ = remove_file(&path);
            return Err(Error::WriteFileType { source: e });
        }
        let bytes_written = super::FILE_TYPE_IDENTIFIER.len();

        if let Err(e) = f.sync_all() {
            _ = remove_file(&path);
            return Err(Error::WriteFileType { source: e });
        }

        Ok(Self {
            path,
            f: BufWriter::with_capacity(buffer_size, f),
            bytes_written,
            uncompressed_bytes_written: bytes_written,
            buffer: Vec::with_capacity(8 * 1204),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Return the number of bytes written (compressed, uncompressed) to the
    /// file.
    pub fn size(&self) -> (usize, usize) {
        (self.bytes_written, self.uncompressed_bytes_written)
    }

    /// write the data to the wal file
    pub fn write(&mut self, data: &[u8], sync: bool) -> Result<()> {
        // Ensure the write buffer is always empty before using it.
        self.buffer.clear();
        // And shrink the buffer below the maximum permitted size should the odd
        // large batch grow it. This is a NOP if the size is less than
        // SOFT_MAX_BUFFER_LEN already.
        self.buffer.shrink_to(super::SOFT_MAX_BUFFER_LEN);

        // Only designed to support chunks up to `u32::max` bytes long.
        let uncompressed_len = data.len();
        u32::try_from(uncompressed_len).context(EntrySizeTooLargeSnafu {
            actual: uncompressed_len,
        })?;

        // The chunk header is two u32 values, so write a dummy u64 value and
        // come back to fill them in later.
        self.buffer
            .write_u64::<BigEndian>(0)
            .expect("cannot fail to write to buffer");

        // Compress the payload into the reused buffer, recording the crc hash
        // as it is wrote.
        let mut encoder = snap::write::FrameEncoder::new(HasherWrapper::new(&mut self.buffer));
        encoder.write_all(data).context(UnableToCompressDataSnafu)?;
        let (checksum, buf) = encoder
            .into_inner()
            .expect("cannot fail to flush to a Vec")
            .finalize();

        // Adjust the compressed length to take into account the u64 padding above.
        let compressed_len = buf.len() - std::mem::size_of::<u64>();
        let compressed_len = u32::try_from(compressed_len).context(EntrySizeTooLargeSnafu {
            actual: compressed_len,
        })?;

        // Go back and write the chunk header values
        let mut buf = std::io::Cursor::new(buf);
        buf.set_position(0);

        buf.write_u32::<BigEndian>(checksum)
            .context(WriteChecksumSnafu)?;
        buf.write_u32::<BigEndian>(compressed_len)
            .context(WriteLengthSnafu)?;

        // Write the entire buffer to the file
        let buf = buf.into_inner();
        let bytes_written = buf.len();

        self.f.write_all(buf).context(WriteDataSnafu)?;

        // fsync the fd
        if sync {
            self.sync()?;
        }

        self.bytes_written += bytes_written;
        self.uncompressed_bytes_written += uncompressed_len;

        Ok(())
    }

    pub fn sync(&mut self) -> Result<()> {
        self.f.flush().context(FileSyncSnafu {
            path: self.path.clone(),
        })?;
        self.f.get_ref().sync_data().context(FileSyncSnafu {
            path: self.path.clone(),
        })?;
        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        self.sync()
    }
}

/// A [`HasherWrapper`] acts as a [`Write`] decorator, recording the crc
/// checksum of the data wrote to the inner [`Write`] implementation.
struct HasherWrapper<W> {
    inner: W,
    hasher: Hasher,
}

impl<W> HasherWrapper<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Hasher::default(),
        }
    }

    fn finalize(self) -> (u32, W) {
        (self.hasher.finalize(), self.inner)
    }
}

impl<W> Write for HasherWrapper<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
