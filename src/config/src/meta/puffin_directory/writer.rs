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
    io::{self},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use ahash::HashSet;
use anyhow::{Context, Result};
use tantivy::{
    directory::{error::OpenReadError, Directory, RamDirectory, WatchCallback, WatchHandle},
    HasLen,
};

use crate::meta::{
    puffin::{writer::PuffinBytesWriter, BlobTypes},
    puffin_directory::{ALLOWED_FILE_EXT, META_JSON},
};
/// Puffin directory is a puffin file which contains all the tantivy files.
/// Each tantivy file is stored as a blob in the puffin file, along with their file name.
#[derive(Debug)]
pub struct PuffinDirWriter {
    ram_directory: Arc<RamDirectory>,
    /// record all the files paths in the puffin file
    file_paths: Arc<RwLock<HashSet<PathBuf>>>,
}

impl Default for PuffinDirWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PuffinDirWriter {
    fn clone(&self) -> Self {
        PuffinDirWriter {
            ram_directory: self.ram_directory.clone(),
            file_paths: self.file_paths.clone(),
        }
    }
}

impl PuffinDirWriter {
    pub fn new() -> Self {
        PuffinDirWriter {
            ram_directory: Arc::new(RamDirectory::create()),
            file_paths: Arc::new(RwLock::new(HashSet::default())),
        }
    }

    pub fn list_files(&self) -> Vec<PathBuf> {
        self.file_paths
            .read()
            .expect("poisoned lock")
            .iter()
            .cloned()
            .collect()
    }

    // This function will serialize the directory into a single puffin file
    pub fn to_puffin_bytes(&self) -> Result<Vec<u8>> {
        let mut puffin_buf: Vec<u8> = Vec::new();
        let mut puffin_writer = PuffinBytesWriter::new(&mut puffin_buf);
        let mut segment_id = String::new();

        let file_paths = self.file_paths.read().expect("poisoned lock");
        let allowed_file_paths = file_paths.iter().filter(|path| {
            let mut allowed = false;
            if let Some(path_ext) = path.extension() {
                if ALLOWED_FILE_EXT.contains(&path_ext.to_str().unwrap()) {
                    allowed = true;
                }
            }

            // check if its meta.json file
            if !allowed && path.to_str().unwrap() == META_JSON {
                allowed = true;
            };
            allowed
        });
        for path in allowed_file_paths.clone() {
            if segment_id.is_empty() && path.extension().is_some_and(|ext| ext != "json") {
                segment_id = path.file_stem().unwrap().to_str().unwrap().to_owned();
            }

            let file_data = self.ram_directory.open_read(path)?;
            log::debug!(
                "Serializing file to puffin: len: {}, path: {}",
                file_data.len(),
                path.to_str().unwrap()
            );
            puffin_writer
                .add_blob(
                    &file_data
                        .read_bytes()
                        .expect("failed to read file")
                        .to_vec(),
                    BlobTypes::O2TtvV1,
                    path.to_str().unwrap().to_owned(),
                    false,
                )
                .context("Failed to add blob")?;
        }

        puffin_writer.finish().context("Failed to finish writing")?;
        Ok(puffin_buf)
    }
}

/// This implementation is used during index creation.
impl Directory for PuffinDirWriter {
    fn get_file_handle(
        &self,
        path: &Path,
    ) -> Result<std::sync::Arc<dyn tantivy::directory::FileHandle>, OpenReadError> {
        self.ram_directory.get_file_handle(path)
    }

    fn delete(&self, path: &Path) -> Result<(), tantivy::directory::error::DeleteError> {
        self.ram_directory.delete(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        self.ram_directory.exists(path)
    }

    fn open_write(
        &self,
        path: &Path,
    ) -> Result<tantivy::directory::WritePtr, tantivy::directory::error::OpenWriteError> {
        // capture the files being written to ram directory
        self.file_paths.write().unwrap().insert(path.to_path_buf());
        self.ram_directory.open_write(path)
    }

    // this should read from the puffin source
    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        self.ram_directory.atomic_read(path)
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        // capture the files being written to ram directory
        self.file_paths
            .write()
            .expect("poisoned lock")
            .insert(path.to_path_buf());
        self.ram_directory.atomic_write(path, data)
    }

    fn sync_directory(&self) -> io::Result<()> {
        self.ram_directory.sync_directory()
    }

    fn watch(&self, watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        self.ram_directory.watch(watch_callback)
    }
}
