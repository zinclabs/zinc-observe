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

use std::{path::PathBuf, sync::LazyLock};

use anyhow::Result;
use tantivy::{
    directory::{Directory, OwnedBytes},
    doc,
    schema::Schema,
};
use writer::PuffinDirWriter;

pub mod reader;
pub mod writer;

// We do not need all of the tantivy files, only the .term and .idx files
// for getting doc IDs and also the meta.json file
// This might change in the future when we add more features to the index
const ALLOWED_FILE_EXT: &[&str] = &["term", "idx", "pos"];
const META_JSON: &str = "meta.json";

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
