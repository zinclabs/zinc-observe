use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
};

use ahash::HashSet;
use anyhow::{Context, Result};
use futures::{AsyncRead, AsyncSeek};
use itertools::Itertools;
use tantivy::{
    directory::{error::OpenReadError, Directory, RamDirectory, WatchCallback, WatchHandle},
    doc,
    schema::Schema,
    HasLen,
};

use crate::{
    get_config,
    meta::puffin::{reader::PuffinBytesReader, writer::PuffinBytesWriter},
};

// This is an identifier for tantivy blobs inside puffin file.
// Note: Tantivy blobs are not compressed.
const TANTIVY_INDEX_VERSION: &str = "TIIv0.1.0";

// We do not need all of the tantivy files, only the .term and .idx files
// for getting doc IDs and also the meta.json file
// This might change in the future when we add more features to the index
const ALLOWED_FILE_EXT: &[&str] = &["term", "idx", "pos"];
const META_JSON: &str = "meta.json";

// Lazy loaded global instance of RAM directory which will contain
// all the files of an empty tantivy index. This instance will be used to fill the missing files
// from the `.ttv` file, as tantivy needs them regardless of the configuration of a field.
static EMPTY_PUFFIN_DIRECTORY: LazyLock<PuffinDirectory> = LazyLock::new(|| {
    let puffin_dir = PuffinDirectory::new();
    let puffin_dir_clone = puffin_dir.clone();
    let schema = Schema::builder().build();
    let mut index_writer = tantivy::IndexBuilder::new()
        .schema(schema)
        .single_segment_index_writer(puffin_dir_clone, 50_000_000)
        .unwrap();
    let _ = index_writer.add_document(doc!());
    index_writer.finalize().unwrap();
    puffin_dir
});

/// Puffin directory is a puffin file which contains all the tantivy files.
/// Each tantivy file is stored as a blob in the puffin file, along with their file name.
#[derive(Debug)]
pub struct PuffinDirectory {
    ram_directory: Arc<RamDirectory>,
    /// record all the files paths in the puffin file
    file_paths: Arc<RwLock<HashSet<PathBuf>>>,
}

impl Clone for PuffinDirectory {
    fn clone(&self) -> Self {
        PuffinDirectory {
            ram_directory: self.ram_directory.clone(),
            file_paths: self.file_paths.clone(),
        }
    }
}

pub fn convert_puffin_dir_to_tantivy_dir(
    mut puffin_dir_path: PathBuf,
    puffin_dir: PuffinDirectory,
) {
    // create directory
    let cfg = get_config();
    let file_name = puffin_dir_path.file_name().unwrap();
    let mut file_name = file_name.to_os_string();
    file_name.push(".folder");
    puffin_dir_path.set_file_name(file_name);
    let mut tantivy_folder_path = PathBuf::from(&cfg.common.data_stream_dir);
    tantivy_folder_path.push(PathBuf::from(&puffin_dir_path));

    // Check if the folder already exists
    if !tantivy_folder_path.exists() {
        std::fs::create_dir_all(&tantivy_folder_path).unwrap();
        log::info!(
            "Created folder for index at {}",
            tantivy_folder_path.to_str().unwrap()
        );
    } else {
        log::warn!(
            "Folder already exists for index at {}",
            tantivy_folder_path.to_str().unwrap()
        );
    }

    for file in puffin_dir.list_files() {
        let file_data = puffin_dir.open_read(&PathBuf::from(file.clone())).unwrap();
        let mut file_handle = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(tantivy_folder_path.join(&file))
            .unwrap();
        file_handle
            .write_all(&file_data.read_bytes().unwrap())
            .unwrap();
        file_handle.flush().unwrap();
    }
}

impl PuffinDirectory {
    pub fn new() -> Self {
        PuffinDirectory {
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
        let empty_puffin_dir = &EMPTY_PUFFIN_DIRECTORY;
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
                    file_data
                        .read_bytes()
                        .expect("failed to read file")
                        .to_vec(),
                    TANTIVY_INDEX_VERSION.to_string(),
                    path.to_str().unwrap().to_owned(),
                    false,
                )
                .context("Failed to add blob")?;
        }

        let allowed_file_paths = allowed_file_paths.collect_vec();
        // find the files which are present in the empty puffin dir instance and write them
        // to the new puffin directory
        for file in empty_puffin_dir
            .list_files()
            .iter()
            .filter(|file_name| !file_name.extension().is_some_and(|ext| ext == "json"))
        {
            let mut empty_puffin_dir_path = PathBuf::from(&file);

            // convert the empty puffin dir path to match the current dir file names
            let ext = empty_puffin_dir_path.extension().unwrap().to_str().unwrap();
            empty_puffin_dir_path.set_file_name(format!("{}.{}", segment_id, ext));

            // we skip the files which are already added
            if allowed_file_paths.contains(&&empty_puffin_dir_path) {
                continue;
            }

            let path = PathBuf::from(&file);
            let file_data = empty_puffin_dir.open_read(&path)?;

            log::debug!(
                "Substituting file for puffin dir: len: {}, path: {}",
                file_data.len(),
                path.to_str().unwrap()
            );

            puffin_writer
                .add_blob(
                    file_data
                        .read_bytes()
                        .expect("failed to read file")
                        .to_vec(),
                    TANTIVY_INDEX_VERSION.to_string(),
                    empty_puffin_dir_path.to_str().unwrap().to_owned(),
                    false,
                )
                .context("Failed to add blob")?;
        }

        puffin_writer.finish().context("Failed to finish writing")?;
        Ok(puffin_buf)
    }

    /// Open a puffin direcotry from the given bytes data
    pub async fn from_bytes<R>(data: R) -> Result<Self>
    where
        R: AsyncRead + AsyncSeek + Unpin + Send,
    {
        let mut puffin_reader = PuffinBytesReader::new(data);
        let puffin_dir = PuffinDirectory::new();

        let puffin_meta = puffin_reader
            .get_metadata()
            .await
            .context("Failed to get blobs meta")?;

        for blob_meta in puffin_meta.blob_metadata {
            let blob = puffin_reader.read_blob_bytes(&blob_meta).await?;
            // Fetch the files names from the blob_meta itself
            if let Some(file_name) = blob_meta.properties.get("file_name") {
                let path = PathBuf::from(file_name);
                let mut writer = puffin_dir
                    .open_write(&path)
                    .context("Failed to write to RAM directory")?;
                writer.write_all(&blob)?;
                writer.flush()?;
                puffin_dir.add_file_path(path);
            }
        }
        Ok(puffin_dir)
    }

    fn add_file_path(&self, path: PathBuf) {
        self.file_paths.write().unwrap().insert(path);
    }
}

impl Directory for PuffinDirectory {
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
