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

use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        cluster::Role,
        stream::{PartitionTimeLevel, StreamType, ALL_STREAM_TYPES},
    },
};
use infra::{
    dist_lock, file_list as infra_file_list,
    schema::{get_settings, unwrap_partition_time_level},
};
use tokio::sync::{mpsc, Semaphore};

use crate::{
    common::infra::cluster::{get_node_by_uuid, get_node_from_consistent_hash},
    service::db,
};

mod file_list;
pub mod file_list_deleted;
pub mod flatten;
pub mod merge;
pub mod retention;
pub mod stats;

/// compactor retention run steps:
pub async fn run_retention() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    // check data retention
    if cfg.compact.data_retention_days <= 0 {
        return Ok(());
    }

    let now = config::utils::time::now();
    let date = now - Duration::try_days(cfg.compact.data_retention_days).unwrap();
    let data_lifecycle_end = date.format("%Y-%m-%d").to_string();

    let orgs = db::schema::list_organizations_from_cache().await;
    for org_id in orgs {
        for stream_type in ALL_STREAM_TYPES {
            let streams = db::schema::list_streams_from_cache(&org_id, stream_type).await;
            for stream_name in streams {
                let Some(node_name) =
                    get_node_from_consistent_hash(&stream_name, &Role::Compactor, None).await
                else {
                    continue; // no compactor node
                };
                if LOCAL_NODE.name.ne(&node_name) {
                    continue; // not this node
                }

                let schema = infra::schema::get(&org_id, &stream_name, stream_type).await?;
                let stream = super::stream::stream_res(&stream_name, stream_type, schema, None);
                let stream_data_retention_end = if stream.settings.data_retention > 0 {
                    let date = now - Duration::try_days(stream.settings.data_retention).unwrap();
                    date.format("%Y-%m-%d").to_string()
                } else {
                    data_lifecycle_end.clone()
                };
                if let Err(e) = retention::delete_by_stream(
                    &stream_data_retention_end,
                    &org_id,
                    stream_type,
                    &stream_name,
                )
                .await
                {
                    log::error!(
                        "[COMPACTOR] lifecycle: delete_by_stream [{}/{}/{}] error: {}",
                        org_id,
                        stream_type,
                        stream_name,
                        e
                    );
                }
            }
        }
    }

    // delete files
    let jobs = db::compact::retention::list().await?;
    for job in jobs {
        let columns = job.split('/').collect::<Vec<&str>>();
        let org_id = columns[0];
        let stream_type = StreamType::from(columns[1]);
        let stream_name = columns[2];
        let retention = columns[3];

        let Some(node_name) =
            get_node_from_consistent_hash(stream_name, &Role::Compactor, None).await
        else {
            continue; // no compactor node
        };
        if LOCAL_NODE.name.ne(&node_name) {
            continue; // not this node
        }

        let ret = if retention.eq("all") {
            retention::delete_all(org_id, stream_type, stream_name).await
        } else {
            let date_range = retention.split(',').collect::<Vec<&str>>();
            retention::delete_by_date(
                org_id,
                stream_type,
                stream_name,
                (date_range[0], date_range[1]),
            )
            .await
        };

        if let Err(e) = ret {
            log::error!(
                "[COMPACTOR] delete: delete [{}/{}/{}] error: {}",
                org_id,
                stream_type,
                stream_name,
                e
            );
        }
    }

    Ok(())
}

/// Generate job for compactor
pub async fn run_generate_job() -> Result<(), anyhow::Error> {
    let orgs = db::schema::list_organizations_from_cache().await;
    for org_id in orgs {
        // check backlist
        if !db::file_list::BLOCKED_ORGS.is_empty() && db::file_list::BLOCKED_ORGS.contains(&org_id)
        {
            continue;
        }
        for stream_type in ALL_STREAM_TYPES {
            let streams = db::schema::list_streams_from_cache(&org_id, stream_type).await;
            for stream_name in streams {
                let Some(node_name) =
                    get_node_from_consistent_hash(&stream_name, &Role::Compactor, None).await
                else {
                    continue; // no compactor node
                };
                if LOCAL_NODE.name.ne(&node_name) {
                    // Check if this node holds the stream
                    if let Some((offset, _)) = db::compact::files::get_offset_from_cache(
                        &org_id,
                        stream_type,
                        &stream_name,
                    )
                    .await
                    {
                        // release the stream
                        db::compact::files::set_offset(
                            &org_id,
                            stream_type,
                            &stream_name,
                            offset,
                            None,
                        )
                        .await?;
                    }
                    continue; // not this node
                }

                // check if we are allowed to merge or just skip
                if db::compact::retention::is_deleting_stream(
                    &org_id,
                    stream_type,
                    &stream_name,
                    None,
                ) {
                    log::warn!(
                        "[COMPACTOR] the stream [{}/{}/{}] is deleting, just skip",
                        &org_id,
                        stream_type,
                        &stream_name,
                    );
                    continue;
                }

                if let Err(e) =
                    merge::generate_job_by_stream(&org_id, stream_type, &stream_name).await
                {
                    log::error!(
                        "[COMPACTOR] generate_job_by_stream [{}/{}/{}] error: {}",
                        org_id,
                        stream_type,
                        stream_name,
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

/// compactor merging
pub async fn run_merge(
    worker_tx: mpsc::Sender<(merge::MergeSender, merge::MergeBatch)>,
) -> Result<(), anyhow::Error> {
    let cfg = get_config();
    let mut jobs =
        infra_file_list::get_pending_jobs(&LOCAL_NODE.uuid, cfg.compact.batch_size).await?;
    if jobs.is_empty() {
        return Ok(());
    }

    // check the stream, if the stream partition_time_level is daily or compact step secs less than
    // 1 hour, we only allow one compactor to working on it
    let mut need_release_ids = Vec::new();
    for job in jobs.iter() {
        let columns = job.stream.split('/').collect::<Vec<&str>>();
        assert_eq!(columns.len(), 3);
        let org_id = columns[0].to_string();
        let stream_type = StreamType::from(columns[1]);
        let stream_name = columns[2].to_string();
        let stream_setting = get_settings(&org_id, &stream_name, stream_type)
            .await
            .unwrap_or_default();
        let partition_time_level =
            unwrap_partition_time_level(stream_setting.partition_time_level, stream_type);
        if partition_time_level == PartitionTimeLevel::Daily || cfg.compact.step_secs < 3600 {
            // check if this stream need process by this node
            let Some(node_name) =
                get_node_from_consistent_hash(&stream_name, &Role::Compactor, None).await
            else {
                continue; // no compactor node
            };
            if LOCAL_NODE.name.ne(&node_name) {
                need_release_ids.push(job.id); // not this node
            }
        }
    }
    if !need_release_ids.is_empty() {
        // release those jobs
        if let Err(e) = infra_file_list::set_job_pending(&need_release_ids).await {
            log::error!("[COMPACT] set_job_pending failed: {}", e);
        }
        jobs.retain(|job| !need_release_ids.contains(&job.id));
    }

    // create a thread to keep updating the job status
    //
    // Update job status (updated_at) to prevent pickup by another node
    // convert job_timeout from secs to micros, and check 1/4 of job_timeout
    // why 1/4 of job_run_timeout?
    // because the timeout is for the entire job, we need to update the job status
    // before it timeout, using 1/2 might still risk a timeout, so we use 1/4 for safety
    let ttl = std::cmp::max(60, cfg.compact.job_run_timeout / 4) as u64;
    let job_ids = jobs.iter().map(|job| job.id).collect::<Vec<_>>();
    let (_tx, mut rx) = mpsc::channel::<()>(1);
    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(ttl)) => {}
                _ = rx.recv() => {
                    log::debug!("[COMPACT] update_running_jobs done");
                    return;
                }
            }
            for id in job_ids.iter() {
                if let Err(e) = infra_file_list::update_running_jobs(*id).await {
                    log::error!("[COMPACT] update_job_status failed: {}", e);
                }
            }
        }
    });

    let mut tasks = Vec::with_capacity(jobs.len());
    let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.file_merge_thread_num));
    let mut min_offset = i64::MAX;
    for job in jobs {
        if job.offsets == 0 {
            log::error!("[COMPACTOR] merge job offset error: {}", job.offsets);
            continue;
        }
        if job.offsets < min_offset {
            min_offset = job.offsets;
        }

        let columns = job.stream.split('/').collect::<Vec<&str>>();
        assert_eq!(columns.len(), 3);
        let org_id = columns[0].to_string();
        let stream_type = StreamType::from(columns[1]);
        let stream_name = columns[2].to_string();

        // check if we are allowed to merge or just skip
        if db::compact::retention::is_deleting_stream(&org_id, stream_type, &stream_name, None) {
            log::warn!(
                "[COMPACTOR] the stream [{}/{}/{}] is deleting, just skip",
                &org_id,
                stream_type,
                &stream_name,
            );
            continue;
        }

        let org_id = org_id.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let worker_tx = worker_tx.clone();
        let task = tokio::task::spawn(async move {
            if let Err(e) = merge::merge_by_stream(
                worker_tx,
                &org_id,
                stream_type,
                &stream_name,
                job.id,
                job.offsets,
            )
            .await
            {
                log::error!(
                    "[COMPACTOR] merge_by_stream [{}/{}/{}] error: {}",
                    org_id,
                    stream_type,
                    stream_name,
                    e
                );
            }
            drop(permit);
        });
        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    // after compact, compact file list from storage
    if !cfg.common.meta_store_external {
        if let Err(e) = file_list::run(min_offset).await {
            log::error!("[COMPACTOR] merge file list error: {}", e);
        }
    }

    Ok(())
}

/// compactor delay delete files run steps:
/// 1. get pending deleted files from file_list_deleted table, created_at > 2 hours
/// 2. delete files from storage
pub async fn run_delay_deletion() -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let time_max =
        now - Duration::try_hours(get_config().compact.delete_files_delay_hours).unwrap();
    let time_max = Utc
        .with_ymd_and_hms(
            time_max.year(),
            time_max.month(),
            time_max.day(),
            time_max.hour(),
            0,
            0,
        )
        .unwrap();
    let time_max = time_max.timestamp_micros();
    let orgs = db::schema::list_organizations_from_cache().await;
    for org_id in orgs {
        // get the working node for the organization
        let (_, node) = db::compact::organization::get_offset(&org_id, "file_list_deleted").await;
        if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some()
        {
            continue;
        }

        // before start processing, set current node to lock the organization
        let lock_key = format!("/compact/organization/{org_id}");
        let locker = dist_lock::lock(&lock_key, 0, None).await?;
        // check the working node for the organization again, maybe other node locked it
        // first
        let (offset, node) =
            db::compact::organization::get_offset(&org_id, "file_list_deleted").await;
        if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some()
        {
            dist_lock::unlock(&locker).await?;
            continue;
        }
        let ret = if node.is_empty() || LOCAL_NODE.uuid.ne(&node) {
            db::compact::organization::set_offset(
                &org_id,
                "file_list_deleted",
                offset,
                Some(&LOCAL_NODE.uuid.clone()),
            )
            .await
        } else {
            Ok(())
        };
        // already bind to this node, we can unlock now
        dist_lock::unlock(&locker).await?;
        drop(locker);
        ret?;

        let batch_size = 10000;
        loop {
            match file_list_deleted::delete(&org_id, offset, time_max, batch_size).await {
                Ok(affected) => {
                    if affected == 0 {
                        break;
                    }
                    log::debug!("[COMPACTOR] deleted from file_list_deleted {affected} files");
                }
                Err(e) => {
                    log::error!("[COMPACTOR] delete files error: {}", e);
                    break;
                }
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // update offset
        db::compact::organization::set_offset(
            &org_id,
            "file_list_deleted",
            time_max,
            Some(&LOCAL_NODE.uuid.clone()),
        )
        .await?;
    }

    Ok(())
}
