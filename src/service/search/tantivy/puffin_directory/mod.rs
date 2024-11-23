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

use std::{io::Write, path::PathBuf, sync::LazyLock};

use anyhow::Result;
use reader::PuffinDirReader;
use tantivy::{
    directory::{Directory, OwnedBytes},
    doc,
    schema::Schema,
};
use writer::PuffinDirWriter;

pub mod caching_directory;
pub mod footer_cache;
pub mod reader;
pub mod reader_cache;
pub mod writer;

// We do not need all of the tantivy files, only the .term and .idx files
// for getting doc IDs and also the meta.json file
// This might change in the future when we add more features to the index
const ALLOWED_FILE_EXT: &[&str] = &["term", "idx", "pos"];
const EMPTY_FILE_EXT: &[&str] = &["fast", "fieldnorm", "store"];
const META_JSON: &str = "meta.json";
const FOOTER_CACHE: &str = "footer_cache";

// Lazy loaded global instance of RAM directory which will contain
// all the files of an empty tantivy index. This instance will be used to fill the missing files
// from the `.ttv` file, as tantivy needs them regardless of the configuration of a field.
static EMPTY_PUFFIN_DIRECTORY: LazyLock<PuffinDirWriter> = LazyLock::new(|| {
    let puffin_dir = PuffinDirWriter::new();
    let puffin_dir_clone = puffin_dir.clone();
    let schema = Schema::builder().build();
    let mut index_writer = tantivy::IndexBuilder::new()
        .schema(schema)
        .single_segment_index_writer(puffin_dir_clone, 50_000_000)
        .expect("Failed to create index writer for EMPTY_PUFFIN_DIRECTORY");
    let _ = index_writer.add_document(doc!());
    index_writer
        .finalize()
        .expect("Failed to finalize index writer for EMPTY_PUFFIN_DIRECTORY");
    puffin_dir
});

// Lazy loaded global segment id of the empty puffin directory which will be used to construct the
// path of a file
static EMPTY_PUFFIN_SEG_ID: LazyLock<String> = LazyLock::new(|| {
    EMPTY_PUFFIN_DIRECTORY
        .list_files()
        .iter()
        .find(|path| path.extension().is_some_and(|ext| ext != "json"))
        .unwrap()
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
});

pub fn get_file_from_empty_puffin_dir_with_ext(file_ext: &str) -> Result<OwnedBytes> {
    let empty_puffin_dir = &EMPTY_PUFFIN_DIRECTORY;
    let seg_id = &EMPTY_PUFFIN_SEG_ID;
    let file_path = format!("{}.{}", seg_id.as_str(), file_ext);
    let file_data = empty_puffin_dir.open_read(&PathBuf::from(file_path))?;
    Ok(file_data.read_bytes()?)
}

pub async fn convert_puffin_file_to_tantivy_dir<T: Into<PathBuf>>(
    puffin_dir: PuffinDirReader,
    dest_path: T,
) -> Result<usize> {
    let dest_path = dest_path.into();
    // Check if the folder already exists
    if !dest_path.exists() {
        std::fs::create_dir_all(&dest_path)?;
    }

    let mut total = 0;
    let mut filename = "".to_string();
    for file in puffin_dir.list_files() {
        if file.extension().is_none() {
            continue;
        }
        let file_data = puffin_dir.open_read(&file.clone())?;
        let mut file_handle = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_path.join(&file))?;
        let data = file_data.read_bytes_async().await?;
        file_handle.write_all(&data)?;
        file_handle.flush()?;
        total += data.len();
        if filename.is_empty() && !file.to_string_lossy().to_string().ends_with(".json") {
            filename = file.to_string_lossy().to_string();
        }
    }

    // Ensure filename is not empty
    if filename.is_empty() {
        return Err(anyhow::anyhow!(
            "No valid segment files found in 'puffin_dir' to determine filename."
        ));
    }

    // remove file ext from filename
    filename = filename.split('.').next().unwrap_or_default().to_string();

    // add other files from the empty tantivy directory
    for file_ext in EMPTY_FILE_EXT {
        let data = get_file_from_empty_puffin_dir_with_ext(file_ext)?;
        let mut h = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_path.join(format!("{}.{}", filename, file_ext)))?;
        h.write_all(&data)?;
        h.flush()?;
        total += data.len();
    }

    Ok(total)
}
