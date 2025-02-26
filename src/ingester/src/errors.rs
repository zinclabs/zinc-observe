// Copyright 2025 OpenObserve Inc.
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

use std::{io, path::PathBuf};

use snafu::Snafu;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    WalError {
        source: wal::Error,
    },
    #[snafu(display("Failed to open file {}: {}", path.display(), source))]
    OpenFileError {
        source: io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to open directory {}: {}", path.display(), source))]
    OpenDirError {
        source: io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to create file {}: {}", path.display(), source))]
    CreateFileError {
        source: io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to delete file {}: {}", path.display(), source))]
    DeleteFileError {
        source: io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to rename file {}: {}", path.display(), source))]
    RenameFileError {
        source: io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to read file {}: {}", path.display(), source))]
    ReadFileError {
        source: io::Error,
        path: PathBuf,
    },
    #[snafu(display("Failed to write file {}: {}", path.display(), source))]
    WriteFileError {
        source: io::Error,
        path: PathBuf,
    },
    WriteDataError {
        source: io::Error,
    },
    ReadDataError {
        source: io::Error,
    },
    JSONSerialization {
        source: serde_json::Error,
    },
    FromUtf8Error {
        source: std::string::FromUtf8Error,
    },
    NotImplemented,
    InferJsonSchemaError {
        source: arrow::error::ArrowError,
    },
    MergeSchemaError {
        source: arrow::error::ArrowError,
    },
    CreateArrowWriterError {
        source: arrow::error::ArrowError,
    },
    WriteArrowRecordBatchError {
        source: arrow::error::ArrowError,
    },
    CreateArrowJsonEncoder {
        source: arrow::error::ArrowError,
    },
    ArrowJsonEncodeError {
        source: arrow::error::ArrowError,
    },
    ReadParquetFileError {
        source: parquet::errors::ParquetError,
    },
    WriteParquetRecordBatchError {
        source: parquet::errors::ParquetError,
    },
    MergeRecordBatchError {
        source: datafusion::error::DataFusionError,
    },
    TokioJoinError {
        source: tokio::task::JoinError,
    },
    TokioMpscSendError {
        source: tokio::sync::mpsc::error::SendError<PathBuf>,
    },
    TokioMpscSendEntriesError {
        source: tokio::sync::mpsc::error::SendError<(
            crate::WriterSignal,
            Vec<crate::entry::Entry>,
            bool,
        )>,
    },
    MemoryTableOverflowError {},
    ExternalError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}
