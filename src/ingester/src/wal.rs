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
    fs::{create_dir_all, File},
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::Arc,
};

use async_walkdir::WalkDir;
use config::utils::schema::infer_json_schema_from_values;
use futures::StreamExt;
use snafu::ResultExt;

use crate::{errors::*, immutable, memtable, writer::WriterKey};

// check uncompleted parquet files
// the wal file process have 4 steps:
// 1. write the memory file into disk with .par file extension
// 2. create a lock file with those file names
// 3. delete the wal file
// 4. rename the .par files to .parquet
// 5. delete the lock file
//
// so, there are some cases that the process is not completed:
// 1. the process is killed before step 2, so there are some .par files and have no lock file, need
//    delete those files
// 2. the process is killed before step 3, so there are some .par files and have lock file, the
//    files actually wrote to disk completely, need to continue step 3, 4 and 5
// 3. the process is killed before step 4, so there are some .par files and have lock file, the
//    files actually wrote to disk completely, need to continue step 4 and 5
// 4. the process is killed before step 5, so there are some .parquet files and have lock file, the
//    files actually wrote to disk completely, need to continue step 5
pub(crate) async fn check_uncompleted_parquet_files() -> Result<()> {
    let cfg = config::get_config();
    // 1. get all .lock files
    let wal_dir = PathBuf::from(&cfg.common.data_wal_dir).join("logs");
    // create wal dir if not exists
    create_dir_all(&wal_dir).context(OpenDirSnafu {
        path: wal_dir.clone(),
    })?;
    let lock_files = wal_scan_files(wal_dir, "lock").await.unwrap_or_default();

    // 2. check if there is a .wal file with same name, delete it and rename the .par to .parquet
    for lock_file in lock_files.iter() {
        log::warn!("found uncompleted wal file: {:?}", lock_file);
        let wal_file = lock_file.with_extension("wal");
        if wal_file.exists() {
            // delete the .wal file
            log::warn!("delete processed wal file: {:?}", wal_file);
            std::fs::remove_file(&wal_file).context(DeleteFileSnafu { path: wal_file })?;
        }
        // read all the .par files
        let mut file = File::open(lock_file).context(OpenFileSnafu { path: lock_file })?;
        let mut par_files = Vec::new();
        for line in BufReader::new(&mut file).lines() {
            let line = line.context(ReadFileSnafu { path: lock_file })?;
            par_files.push(line);
        }
        // rename the .par file to .parquet
        for par_file in par_files.iter() {
            let par_file = PathBuf::from(par_file);
            let parquet_file = par_file.with_extension("parquet");
            log::warn!("rename par file: {:?} to parquet", par_file);
            if par_file.exists() {
                std::fs::rename(&par_file, &parquet_file)
                    .context(RenameFileSnafu { path: par_file })?;
            }
        }
        // delete the .lock file
        log::warn!("delete lock file: {:?}", lock_file);
        std::fs::remove_file(lock_file).context(DeleteFileSnafu {
            path: lock_file.clone(),
        })?;
    }

    // 4. delete all the .par files
    let parquet_dir = PathBuf::from(&cfg.common.data_wal_dir).join("files");
    // create wal dir if not exists
    create_dir_all(&parquet_dir).context(OpenDirSnafu {
        path: parquet_dir.clone(),
    })?;
    let par_files = wal_scan_files(parquet_dir, "par").await.unwrap_or_default();
    for par_file in par_files.iter() {
        log::warn!("delete uncompleted par file: {:?}", par_file);
        std::fs::remove_file(par_file).context(DeleteFileSnafu { path: par_file })?;
    }
    Ok(())
}

// replay wal files to create immutable
pub(crate) async fn replay_wal_files() -> Result<()> {
    let wal_dir = PathBuf::from(&config::get_config().common.data_wal_dir).join("logs");
    create_dir_all(&wal_dir).context(OpenDirSnafu {
        path: wal_dir.clone(),
    })?;
    let wal_files = wal_scan_files(&wal_dir, "wal").await.unwrap_or_default();
    if wal_files.is_empty() {
        return Ok(());
    }
    for wal_file in wal_files.iter() {
        log::warn!("starting replay wal file: {:?}", wal_file);
        let file_str = wal_file
            .strip_prefix(&wal_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/")
            .to_string();
        let file_columns = file_str.split('/').collect::<Vec<_>>();
        let stream_type = file_columns[file_columns.len() - 2];
        let org_id = file_columns[file_columns.len() - 3];
        let idx: usize = file_columns[file_columns.len() - 4]
            .parse()
            .unwrap_or_default();
        let key = WriterKey::new(org_id, stream_type);
        let mut memtable = memtable::MemTable::new();
        let mut reader = match wal::Reader::from_path(wal_file) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Unable to open the wal file err: {}, skip", e);
                continue;
            }
        };
        let mut total = 0;
        let mut i = 0;
        loop {
            if i > 0 && i % 1000 == 0 {
                log::warn!(
                    "replay wal file: {:?}, entries: {}, records: {}",
                    wal_file,
                    i,
                    total
                );
            }
            let entry = match reader.read_entry() {
                Ok(entry) => entry,
                Err(wal::Error::UnableToReadData { source }) => {
                    log::error!("Unable to read entry from: {}, skip the entry", source);
                    continue;
                }
                Err(wal::Error::LengthMismatch { expected, actual }) => {
                    log::error!(
                        "Unable to read entry: Length mismatch: expected {}, actual {}, skip the entry",
                        expected,
                        actual
                    );
                    continue;
                }
                Err(wal::Error::ChecksumMismatch { expected, actual }) => {
                    log::error!(
                        "Unable to read entry: Checksum mismatch: expected {}, actual {}, skip the entry",
                        expected,
                        actual
                    );
                    continue;
                }
                Err(e) => {
                    return Err(Error::WalError { source: e });
                }
            };
            let Some(entry_bytes) = entry else {
                break;
            };
            let entry = match super::Entry::from_bytes(&entry_bytes) {
                Ok(v) => v,
                Err(Error::ReadDataError { source }) => {
                    log::error!("Unable to read entry from: {}, skip the entry", source);
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };
            i += 1;
            total += entry.data.len();
            let infer_schema =
                infer_json_schema_from_values(entry.data.iter().cloned(), stream_type)
                    .context(InferJsonSchemaSnafu)?;
            let infer_schema = Arc::new(infer_schema);
            let batch = entry.into_batch(key.stream_type.clone(), infer_schema.clone())?;
            memtable.write(infer_schema, entry, batch)?;
        }
        log::warn!(
            "replay wal file: {:?}, entries: {}, records: {}",
            wal_file,
            i,
            total
        );

        immutable::IMMUTABLES.write().await.insert(
            wal_file.to_owned(),
            Arc::new(immutable::Immutable::new(idx, key, memtable)),
        );

        // to avoid the memory OOM, sleep for a while after reply one wal file
        if immutable::len().await > 2 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    Ok(())
}

async fn wal_scan_files(root_dir: impl Into<PathBuf>, ext: &str) -> Result<Vec<PathBuf>> {
    Ok(WalkDir::new(root_dir.into())
        .filter_map(|entry| async move {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() {
                let path_ext = path.extension()?.to_str()?;
                if path_ext == ext {
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
        .await)
}
