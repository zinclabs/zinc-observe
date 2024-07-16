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
    collections::{HashMap, HashSet},
    io::Write,
};

use chrono::{DateTime, Duration, TimeZone, Utc};
use config::{
    cluster::LOCAL_NODE,
    get_config, ider, is_local_disk_storage,
    meta::stream::{FileKey, FileMeta, PartitionTimeLevel, StreamStats, StreamType},
    utils::{json, time::BASE_TIME},
};
use infra::{cache, dist_lock, file_list as infra_file_list, storage};

use crate::{
    common::infra::cluster::get_node_by_uuid,
    service::{db, file_list},
};

pub async fn delete_by_stream(
    lifecycle_end: &str,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    // get schema
    let stats = cache::stats::get_stream_stats(org_id, stream_name, stream_type);
    let created_at = stats.doc_time_min;
    if created_at == 0 {
        return Ok(()); // no data, just skip
    }
    let created_at: DateTime<Utc> = Utc.timestamp_nanos(created_at * 1000);
    let lifecycle_start = created_at.format("%Y-%m-%d").to_string();
    let lifecycle_start = lifecycle_start.as_str();
    if lifecycle_start.ge(lifecycle_end) {
        return Ok(()); // created_at is after lifecycle_end, just skip
    }

    // Hack for 1970-01-01
    if lifecycle_start.le("1970-01-01") {
        let lifecycle_end = created_at + Duration::try_days(1).unwrap();
        let lifecycle_end = lifecycle_end.format("%Y-%m-%d").to_string();
        return db::compact::retention::delete_stream(
            org_id,
            stream_type,
            stream_name,
            Some((lifecycle_start, lifecycle_end.as_str())),
        )
        .await;
    }

    // delete files
    db::compact::retention::delete_stream(
        org_id,
        stream_type,
        stream_name,
        Some((lifecycle_start, lifecycle_end)),
    )
    .await
}

pub async fn delete_all(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    let lock_key = format!("/compact/retention/{org_id}/{stream_type}/{stream_name}");
    let locker = dist_lock::lock(&lock_key, 0).await?;
    let node = db::compact::retention::get_stream(org_id, stream_type, stream_name, None).await;
    if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        log::error!("[COMPACT] stream {org_id}/{stream_type}/{stream_name} is deleting by {node}");
        dist_lock::unlock(&locker).await?;
        return Ok(()); // not this node, just skip
    }

    // before start merging, set current node to lock the stream
    let ret = db::compact::retention::process_stream(
        org_id,
        stream_type,
        stream_name,
        None,
        &LOCAL_NODE.uuid.clone(),
    )
    .await;
    // already bind to this node, we can unlock now
    dist_lock::unlock(&locker).await?;
    drop(locker);
    ret?;

    let start_time = BASE_TIME.timestamp_micros();
    let end_time = Utc::now().timestamp_micros();

    let cfg = get_config();
    if is_local_disk_storage() {
        let data_dir = format!(
            "{}files/{org_id}/{stream_type}/{stream_name}",
            cfg.common.data_stream_dir
        );
        let path = std::path::Path::new(&data_dir);
        if path.exists() {
            tokio::fs::remove_dir_all(path).await?;
        }
        log::info!("deleted all files: {:?}", path);
    } else {
        // delete files from s3
        // first fetch file list from local cache
        let files = file_list::query(
            org_id,
            stream_name,
            stream_type,
            PartitionTimeLevel::Unset,
            start_time,
            end_time,
            true,
        )
        .await?;
        if cfg.compact.data_retention_history {
            // only store the file_list into history, don't delete files
            if let Err(e) = infra_file_list::batch_add_history(&files).await {
                log::error!("[COMPACT] file_list batch_add_history failed: {}", e);
                return Err(e.into());
            }
        }
    }

    // delete from file list
    delete_from_file_list(org_id, stream_type, stream_name, (start_time, end_time)).await?;
    log::info!(
        "deleted file list for: {}/{}/{}/all",
        org_id,
        stream_type,
        stream_name
    );

    // delete stream schema
    db::schema::delete(org_id, stream_name, Some(stream_type)).await?;
    log::info!(
        "deleted stream_schema for: {}/{}/{}",
        org_id,
        stream_type,
        stream_name
    );

    // delete stream stats
    infra_file_list::del_stream_stats(org_id, stream_type, stream_name).await?;
    log::info!(
        "deleted stream_stats for: {}/{}/{}/all",
        org_id,
        stream_type,
        stream_name
    );

    // mark delete done
    db::compact::retention::delete_stream_done(org_id, stream_type, stream_name, None).await?;
    log::info!(
        "deleted stream all: {}/{}/{}",
        org_id,
        stream_type,
        stream_name
    );

    Ok(())
}

pub async fn delete_by_date(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: (&str, &str),
) -> Result<(), anyhow::Error> {
    let lock_key = format!("/compact/retention/{org_id}/{stream_type}/{stream_name}");
    let locker = dist_lock::lock(&lock_key, 0).await?;
    let node =
        db::compact::retention::get_stream(org_id, stream_type, stream_name, Some(date_range))
            .await;
    if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        log::error!(
            "[COMPACT] stream {org_id}/{stream_type}/{stream_name}/{:?} is deleting by {node}",
            date_range
        );
        dist_lock::unlock(&locker).await?;
        return Ok(()); // not this node, just skip
    }

    // before start merging, set current node to lock the stream
    let ret = db::compact::retention::process_stream(
        org_id,
        stream_type,
        stream_name,
        Some(date_range),
        &LOCAL_NODE.uuid.clone(),
    )
    .await;
    // already bind to this node, we can unlock now
    dist_lock::unlock(&locker).await?;
    drop(locker);
    ret?;

    let mut date_start =
        DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", date_range.0))?.with_timezone(&Utc);
    let date_end =
        DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", date_range.1))?.with_timezone(&Utc);
    let time_range = { (date_start.timestamp_micros(), date_end.timestamp_micros()) };

    let cfg = get_config();
    if is_local_disk_storage() {
        while date_start <= date_end {
            let data_dir = format!(
                "{}files/{org_id}/{stream_type}/{stream_name}/{}",
                cfg.common.data_stream_dir,
                date_start.format("%Y/%m/%d")
            );
            let path = std::path::Path::new(&data_dir);
            if path.exists() {
                tokio::fs::remove_dir_all(path).await?;
            }
            date_start += Duration::try_days(1).unwrap();
        }
    } else {
        // delete files from s3
        // first fetch file list from local cache
        let files = file_list::query(
            org_id,
            stream_name,
            stream_type,
            PartitionTimeLevel::Unset,
            time_range.0,
            time_range.1,
            true,
        )
        .await?;
        if cfg.compact.data_retention_history {
            // only store the file_list into history, don't delete files
            if let Err(e) = infra_file_list::batch_add_history(&files).await {
                log::error!("[COMPACT] file_list batch_add_history failed: {}", e);
                return Err(e.into());
            }
        }
    }

    // delete from file list
    delete_from_file_list(org_id, stream_type, stream_name, time_range).await?;

    // archive old schema versions
    let mut schema_versions =
        infra::schema::get_versions(org_id, stream_name, stream_type, Some(time_range)).await?;
    // pop last version, it's the current version
    schema_versions.pop();
    for schema in schema_versions {
        let start_dt: i64 = match schema.metadata().get("start_dt") {
            Some(v) => v.parse().unwrap_or_default(),
            None => 0,
        };
        if start_dt == 0 {
            continue;
        }
        infra::schema::history::create(org_id, stream_type, stream_name, start_dt, schema).await?;
        infra::schema::delete(org_id, stream_type, stream_name, Some(start_dt)).await?;
    }

    // update stream stats retention time
    let mut stats = cache::stats::get_stream_stats(org_id, stream_name, stream_type);
    let mut min_ts = if time_range.1 > BASE_TIME.timestamp_micros() {
        time_range.1
    } else {
        infra_file_list::get_min_ts(org_id, stream_type, stream_name)
            .await
            .unwrap_or_default()
    };
    if min_ts == 0 {
        min_ts = stats.doc_time_min;
    };
    infra_file_list::reset_stream_stats_min_ts(
        org_id,
        format!("{org_id}/{stream_type}/{stream_name}").as_str(),
        min_ts,
    )
    .await?;
    // update stream stats in cache
    if min_ts > stats.doc_time_min {
        stats.doc_time_min = min_ts;
        cache::stats::set_stream_stats(org_id, stream_name, stream_type, stats);
    }

    // mark delete done
    db::compact::retention::delete_stream_done(org_id, stream_type, stream_name, Some(date_range))
        .await
}

async fn delete_from_file_list(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    time_range: (i64, i64),
) -> Result<(), anyhow::Error> {
    let files = file_list::query(
        org_id,
        stream_name,
        stream_type,
        PartitionTimeLevel::Unset,
        time_range.0,
        time_range.1,
        true,
    )
    .await?;
    if files.is_empty() {
        return Ok(());
    }

    // collect stream stats
    let mut stream_stats = StreamStats::default();

    let mut file_list_days: HashSet<String> = HashSet::new();
    let mut hours_files: HashMap<String, Vec<FileKey>> = HashMap::with_capacity(24);
    for file in files {
        stream_stats = stream_stats - file.meta;
        let file_name = file.key.clone();
        let columns: Vec<_> = file_name.split('/').collect();
        let day_key = format!("{}-{}-{}", columns[4], columns[5], columns[6]);
        file_list_days.insert(day_key);
        let hour_key = format!(
            "{}/{}/{}/{}",
            columns[4], columns[5], columns[6], columns[7]
        );
        let entry = hours_files.entry(hour_key).or_default();
        entry.push(FileKey {
            key: file_name,
            meta: FileMeta::default(),
            deleted: true,
            segment_ids: None,
        });
    }

    // write file list to storage
    write_file_list(org_id, file_list_days, hours_files).await?;

    // update stream stats
    if stream_stats.doc_num != 0 {
        infra_file_list::set_stream_stats(
            org_id,
            &[(
                format!("{org_id}/{stream_type}/{stream_name}"),
                stream_stats,
            )],
        )
        .await?;
    }

    Ok(())
}

async fn write_file_list(
    org_id: &str,
    file_list_days: HashSet<String>,
    hours_files: HashMap<String, Vec<FileKey>>,
) -> Result<(), anyhow::Error> {
    if get_config().common.meta_store_external {
        write_file_list_db_only(org_id, hours_files).await
    } else {
        write_file_list_s3(org_id, file_list_days, hours_files).await
    }
}

async fn write_file_list_db_only(
    org_id: &str,
    hours_files: HashMap<String, Vec<FileKey>>,
) -> Result<(), anyhow::Error> {
    for (_key, events) in hours_files {
        let put_items = events
            .iter()
            .filter(|v| !v.deleted)
            .map(|v| v.to_owned())
            .collect::<Vec<_>>();
        let del_items = events
            .iter()
            .filter(|v| v.deleted)
            .map(|v| v.key.clone())
            .collect::<Vec<_>>();
        // set to external db
        // retry 5 times
        let mut success = false;
        let created_at = Utc::now().timestamp_micros();
        for _ in 0..5 {
            if let Err(e) =
                infra_file_list::batch_add_deleted(org_id, false, created_at, &del_items).await
            {
                log::error!(
                    "[COMPACT] batch_add_deleted to external db failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            if let Err(e) = infra_file_list::batch_add(&put_items).await {
                log::error!("[COMPACT] batch_add to external db failed, retrying: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            if let Err(e) = infra_file_list::batch_remove(&del_items).await {
                log::error!(
                    "[COMPACT] batch_delete to external db failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            success = true;
            break;
        }
        if !success {
            return Err(anyhow::anyhow!(
                "[COMPACT] batch_write to external db failed"
            ));
        }
    }
    Ok(())
}

async fn write_file_list_s3(
    org_id: &str,
    file_list_days: HashSet<String>,
    hours_files: HashMap<String, Vec<FileKey>>,
) -> Result<(), anyhow::Error> {
    for (key, events) in hours_files {
        // upload the new file_list to storage
        let new_file_list_key = format!("file_list/{key}/{}.json.zst", ider::generate());
        let mut buf = zstd::Encoder::new(Vec::new(), 3)?;
        for file in events.iter() {
            let mut write_buf = json::to_vec(&file)?;
            write_buf.push(b'\n');
            buf.write_all(&write_buf)?;
        }
        let compressed_bytes = buf.finish().unwrap();
        storage::put(&new_file_list_key, compressed_bytes.into()).await?;

        // write deleted files into file_list_deleted
        let del_items = events
            .iter()
            .filter(|v| v.deleted)
            .map(|v| v.key.clone())
            .collect::<Vec<_>>();
        if !del_items.is_empty() {
            let deleted_file_list_key = format!(
                "file_list_deleted/{org_id}/{}/{}.json.zst",
                Utc::now().format("%Y/%m/%d/%H"),
                ider::generate()
            );
            let mut buf = zstd::Encoder::new(Vec::new(), 3)?;
            for file in del_items.iter() {
                buf.write_all((file.to_owned() + "\n").as_bytes())?;
            }
            let compressed_bytes = buf.finish().unwrap();
            storage::put(&deleted_file_list_key, compressed_bytes.into()).await?;
        }

        // set to local cache & send broadcast
        // retry 5 times
        for _ in 0..5 {
            // set to local cache
            let mut cache_success = true;
            for event in &events {
                if let Err(e) =
                    db::file_list::progress(&event.key, Some(&event.meta), event.deleted).await
                {
                    cache_success = false;
                    log::error!(
                        "[COMPACT] delete_from_file_list set local cache failed, retrying: {}",
                        e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    break;
                }
            }
            if !cache_success {
                continue;
            }
            // send broadcast to other nodes
            if let Err(e) = db::file_list::broadcast::send(&events, None).await {
                log::error!(
                    "[COMPACT] delete_from_file_list send broadcast failed, retrying: {}",
                    e
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }
            break;
        }
    }

    // mark file list need to do merge again
    for key in file_list_days {
        db::compact::file_list::set_delete(&key).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_delete_by_stream() {
        infra_file_list::create_table().await.unwrap();
        let org_id = "test";
        let stream_name = "test";
        let stream_type = config::meta::stream::StreamType::Logs;
        let lifecycle_end = "2023-01-01";
        delete_by_stream(lifecycle_end, org_id, stream_type, stream_name)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_delete_all() {
        infra_file_list::create_table().await.unwrap();
        let org_id = "test";
        let stream_name = "test";
        let stream_type = config::meta::stream::StreamType::Logs;
        delete_all(org_id, stream_type, stream_name).await.unwrap();
    }
}
