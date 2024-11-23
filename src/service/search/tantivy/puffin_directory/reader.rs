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
    io,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::future::try_join_all;
use hashbrown::HashMap;
use tantivy::{
    directory::{error::OpenReadError, Directory, FileHandle, OwnedBytes},
    HasLen,
};

use crate::service::search::tantivy::{
    puffin::{reader::PuffinBytesReader, BlobMetadata},
    puffin_directory::{
        get_file_from_empty_puffin_dir_with_ext, EMPTY_PUFFIN_DIRECTORY, EMPTY_PUFFIN_SEG_ID,
    },
};

#[derive(Debug)]
pub struct PuffinDirReader {
    source: Arc<PuffinBytesReader>,
    blobs_metadata: Arc<HashMap<PathBuf, Arc<BlobMetadata>>>,
}

impl PuffinDirReader {
    pub async fn from_path(source: object_store::ObjectMeta) -> io::Result<Self> {
        let mut source = PuffinBytesReader::new(source);
        let Some(metadata) = source.get_metadata().await.map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Error reading metadata from puffin file: {:?}", e),
            )
        })?
        else {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Error reading metadata from puffin file",
            ));
        };

        let mut blobs_metadata = HashMap::new();
        for meta in metadata.blobs {
            // Fetch the files names from the blob_meta itself
            if let Some(file_name) = meta.properties.get("blob_tag") {
                let path = PathBuf::from(file_name);
                blobs_metadata.insert(path, Arc::new(meta));
            }
        }

        Ok(Self {
            source: Arc::new(source),
            blobs_metadata: Arc::new(blobs_metadata),
        })
    }

    pub fn list_files(&self) -> Vec<PathBuf> {
        self.blobs_metadata.keys().cloned().collect()
    }
}

impl Clone for PuffinDirReader {
    fn clone(&self) -> Self {
        PuffinDirReader {
            source: self.source.clone(),
            blobs_metadata: self.blobs_metadata.clone(),
        }
    }
}

// Version 1: Keep the blob withing the file handle and return it when read
#[derive(Debug)]
struct PuffinSliceHandle {
    path: PathBuf,
    source: Arc<PuffinBytesReader>,
    metadata: Arc<BlobMetadata>,
}

impl HasLen for PuffinSliceHandle {
    fn len(&self) -> usize {
        self.metadata.length as usize
    }
}

#[async_trait::async_trait]
impl FileHandle for PuffinSliceHandle {
    fn read_bytes(&self, _byte_range: Range<usize>) -> io::Result<OwnedBytes> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Error read_bytes from blob: {:?}, Not supported with PuffinSliceHandle",
                self.path
            ),
        ))
    }

    async fn read_bytes_async(&self, byte_range: Range<usize>) -> io::Result<OwnedBytes> {
        if byte_range.is_empty() {
            return Ok(OwnedBytes::empty());
        }
        let data = self
            .source
            .read_blob_bytes(&self.metadata, Some(byte_range.clone()))
            .await
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error reading bytes from blob: {:?}", e),
                )
            })?;
        Ok(OwnedBytes::new(data.to_vec()))
    }
}

impl Directory for PuffinDirReader {
    fn get_file_handle(
        &self,
        path: &Path,
    ) -> std::result::Result<Arc<dyn FileHandle>, OpenReadError> {
        match self.blobs_metadata.get(path) {
            Some(meta) => {
                let file_handle = PuffinSliceHandle {
                    path: path.to_path_buf(),
                    source: self.source.clone(),
                    metadata: meta.clone(),
                };
                Ok(Arc::new(file_handle))
            }
            None => {
                let ext = match path.extension().and_then(|ext| ext.to_str()) {
                    Some(ext) => ext,
                    None => return Err(OpenReadError::FileDoesNotExist(path.to_path_buf())),
                };
                match get_file_from_empty_puffin_dir_with_ext(ext) {
                    Ok(blob) => Ok(Arc::new(blob)),
                    Err(_) => Err(OpenReadError::FileDoesNotExist(path.to_path_buf())),
                }
            }
        }
    }

    fn exists(&self, path: &Path) -> std::result::Result<bool, OpenReadError> {
        if !self.blobs_metadata.contains_key(path) {
            let ext = match path.extension().and_then(|ext| ext.to_str()) {
                Some(ext) => ext,
                None => return Ok(false),
            };
            let dir_path = format!("{}.{}", &EMPTY_PUFFIN_SEG_ID.as_str(), ext);
            EMPTY_PUFFIN_DIRECTORY.exists(&PathBuf::from(dir_path))
        } else {
            Ok(true)
        }
    }

    fn atomic_read(&self, _path: &Path) -> std::result::Result<Vec<u8>, OpenReadError> {
        unimplemented!("read-only")
    }

    fn atomic_write(&self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        unimplemented!("read-only")
    }

    fn delete(&self, _path: &Path) -> Result<(), tantivy::directory::error::DeleteError> {
        unimplemented!("read-only")
    }

    fn open_write(
        &self,
        _path: &Path,
    ) -> Result<tantivy::directory::WritePtr, tantivy::directory::error::OpenWriteError> {
        unimplemented!("read-only")
    }

    fn sync_directory(&self) -> io::Result<()> {
        unimplemented!("read-only")
    }

    fn watch(
        &self,
        _watch_callback: tantivy::directory::WatchCallback,
    ) -> tantivy::Result<tantivy::directory::WatchHandle> {
        Ok(tantivy::directory::WatchHandle::empty())
    }

    fn acquire_lock(
        &self,
        _lock: &tantivy::directory::Lock,
    ) -> Result<tantivy::directory::DirectoryLock, tantivy::directory::error::LockError> {
        Ok(tantivy::directory::DirectoryLock::from(Box::new(|| {})))
    }
}

/// preload the terms in the index
pub async fn warm_up_terms(
    searcher: &tantivy::Searcher,
    terms_grouped_by_field: &HashMap<tantivy::schema::Field, HashMap<tantivy::Term, bool>>,
) -> anyhow::Result<()> {
    let mut warm_up_fields_futures = Vec::new();
    let mut warm_up_fields_term_futures = Vec::new();
    let mut warm_up_terms_futures = Vec::new();
    for (field, terms) in terms_grouped_by_field {
        for segment_reader in searcher.segment_readers() {
            let inv_idx = segment_reader.inverted_index(*field)?;
            if terms.is_empty() {
                let inv_idx_clone = inv_idx.clone();
                warm_up_fields_futures
                    .push(async move { inv_idx_clone.warm_postings_full(false).await });
                warm_up_fields_term_futures
                    .push(async move { inv_idx.terms().warm_up_dictionary().await });
                continue;
            }
            for (term, position_needed) in terms.iter() {
                let inv_idx_clone = inv_idx.clone();
                warm_up_terms_futures
                    .push(async move { inv_idx_clone.warm_postings(term, *position_needed).await });
            }
        }
    }
    if !warm_up_fields_futures.is_empty() {
        try_join_all(warm_up_fields_futures).await?;
    }
    if !warm_up_fields_term_futures.is_empty() {
        try_join_all(warm_up_fields_term_futures).await?;
    }
    if !warm_up_terms_futures.is_empty() {
        try_join_all(warm_up_terms_futures).await?;
    }
    Ok(())
}
