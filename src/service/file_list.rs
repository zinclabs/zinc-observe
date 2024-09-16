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

use std::io::Write;

use config::{
    get_config, ider,
    meta::{
        search::ScanStats,
        stream::{FileKey, FileMeta, FileQueryData, PartitionTimeLevel, StreamType},
    },
    utils::{file::get_file_meta as util_get_file_meta, json},
};
use infra::{file_list, storage};

use crate::service::db;

#[tracing::instrument(
    name = "service::file_list::query",
    skip_all,
    fields(org_id = org_id, stream_name = stream_name)
)]
pub async fn query(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    time_level: PartitionTimeLevel,
    time_min: i64,
    time_max: i64,
) -> Result<Vec<FileKey>, anyhow::Error> {
    let files = file_list::query(
        org_id,
        stream_type,
        stream_name,
        time_level,
        Some((time_min, time_max)),
        None,
    )
    .await?;
    let mut file_keys = Vec::with_capacity(files.len());
    for file in files {
        file_keys.push(FileKey {
            key: file.0,
            meta: file.1,
            deleted: false,
            segment_ids: None,
        });
    }
    Ok(file_keys)
}

pub async fn query_by_ids(ids: &[i64]) -> Result<Vec<FileKey>, anyhow::Error> {
    let files = file_list::query_by_ids(ids).await?;
    let mut file_keys = Vec::with_capacity(files.len());
    for (key, meta) in files {
        file_keys.push(FileKey {
            key,
            meta,
            deleted: false,
            segment_ids: None,
        });
    }
    Ok(file_keys)
}

#[tracing::instrument(
    name = "service::file_list::query_ids",
    skip_all,
    fields(org_id = org_id, stream_name = stream_name)
)]
pub async fn query_ids(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    time_level: PartitionTimeLevel,
    time_min: i64,
    time_max: i64,
) -> Result<Vec<FileQueryData>, anyhow::Error> {
    Ok(file_list::query_ids(
        org_id,
        stream_type,
        stream_name,
        time_level,
        Some((time_min, time_max)),
        None,
    )
    .await?)
}

#[inline]
pub async fn calculate_files_size(files: &[FileKey]) -> Result<ScanStats, anyhow::Error> {
    let mut stats = ScanStats::new();
    stats.files = files.len() as i64;
    for file in files {
        stats.records += file.meta.records;
        stats.original_size += file.meta.original_size;
        stats.compressed_size += file.meta.compressed_size;
    }
    Ok(stats)
}

#[inline]
pub fn calculate_local_files_size(files: &[String]) -> Result<u64, anyhow::Error> {
    let mut size = 0;
    for file in files {
        let file_size = match util_get_file_meta(file) {
            Ok(resp) => resp.len(),
            Err(_) => 0,
        };
        size += file_size;
    }
    Ok(size)
}

// Delete one parquet file and update the file list
pub async fn delete_parquet_file(key: &str, file_list_only: bool) -> Result<(), anyhow::Error> {
    if get_config().common.meta_store_external {
        delete_parquet_file_db_only(key, file_list_only).await
    } else {
        delete_parquet_file_s3(key, file_list_only).await
    }
}

async fn delete_parquet_file_db_only(key: &str, file_list_only: bool) -> Result<(), anyhow::Error> {
    // delete from file list in metastore
    file_list::batch_remove(&[key.to_string()]).await?;

    // delete the parquet whaterever the file is exists or not
    if !file_list_only {
        _ = storage::del(&[key]).await;
    }
    Ok(())
}

async fn delete_parquet_file_s3(key: &str, file_list_only: bool) -> Result<(), anyhow::Error> {
    let columns = key.split('/').collect::<Vec<&str>>();
    if columns[0] != "files" || columns.len() < 9 {
        return Ok(());
    }
    let new_file_list_key = format!(
        "file_list/{}/{}/{}/{}/{}.json.zst",
        columns[4],
        columns[5],
        columns[6],
        columns[7],
        ider::generate()
    );

    let meta = FileMeta::default();
    let deleted = true;
    let file_data = FileKey {
        key: key.to_string(),
        meta: meta.clone(),
        deleted,
        segment_ids: None,
    };

    // generate the new file list
    let mut buf = zstd::Encoder::new(Vec::new(), 3)?;
    let mut write_buf = json::to_vec(&file_data)?;
    write_buf.push(b'\n');
    buf.write_all(&write_buf)?;
    let compressed_bytes = buf.finish().unwrap();
    storage::put(&new_file_list_key, compressed_bytes.into()).await?;
    db::file_list::progress(key, Some(&meta), deleted).await?;
    db::file_list::broadcast::send(&[file_data], None).await?;

    // delete the parquet whaterever the file is exists or not
    if !file_list_only {
        _ = storage::del(&[key]).await;
    }
    Ok(())
}
