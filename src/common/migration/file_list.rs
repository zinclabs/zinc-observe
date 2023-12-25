// Copyright 2023 Zinc Labs Inc.
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

use config::CONFIG;

use crate::{
    common::{
        infra::file_list as infra_file_list,
        meta::{common::FileKey, stream::PartitionTimeLevel, StreamType},
        utils::{file::get_file_meta, time::BASE_TIME},
    },
    job::{file_list, files},
    service::{compact::stats::update_stats_from_file_list, db},
};

pub async fn run(prefix: &str) -> Result<(), anyhow::Error> {
    if get_file_meta(&CONFIG.common.data_wal_dir).is_err() {
        // there is no local wal files, no need upgrade
        return Ok(());
    }

    // move files from wal for disk
    if let Err(e) = files::json::move_files_to_storage().await {
        log::error!("Error moving disk json files to remote: {}", e);
    }
    if let Err(e) = files::parquet::move_files_to_storage().await {
        log::error!("Error moving disk parquet files to remote: {}", e);
    }

    // move file_list from wal for disk
    if let Err(e) = file_list::move_file_list_to_storage(false).await {
        log::error!("Error moving disk files to remote: {}", e);
    }

    // load stream list
    db::schema::cache().await?;
    // load file list to db
    db::file_list::remote::cache(prefix, false)
        .await
        .expect("file list migration failed");
    infra_file_list::set_initialised()
        .await
        .expect("file list migration set initialised failed");
    // update stream stats
    update_stats_from_file_list()
        .await
        .expect("file list migration stats failed");
    Ok(())
}

/// Run the file list migration for DynamoDB to add new fields `created_at` and
/// `org`.
pub async fn run_for_dynamo() -> Result<(), anyhow::Error> {
    // load stream list
    db::schema::cache().await?;
    // load file list from DynamoDB
    let stream_types = [
        StreamType::Logs,
        StreamType::Metrics,
        StreamType::Traces,
        StreamType::Metadata,
    ];
    let start_time = BASE_TIME.timestamp_micros();
    let end_time = chrono::Utc::now().timestamp_micros();
    let orgs = db::schema::list_organizations_from_cache();
    for org_id in orgs.iter() {
        for stream_type in stream_types {
            let streams = db::schema::list_streams_from_cache(org_id, stream_type);
            for stream_name in streams.iter() {
                let files = infra_file_list::query(
                    org_id,
                    stream_type,
                    stream_name,
                    PartitionTimeLevel::Unset,
                    (start_time, end_time),
                )
                .await
                .expect("file list get failed");
                let put_items = files
                    .into_iter()
                    .map(|(file_key, file_meta)| FileKey::new(&file_key, file_meta, false))
                    .collect::<Vec<_>>();
                infra_file_list::batch_add(&put_items)
                    .await
                    .expect("file list put failed");
            }
        }
    }
    // create secondary index
    infra_file_list::dynamo::create_table_file_list_org_crated_at_index()
        .await
        .expect("file list migration create index failed");
    Ok(())
}

pub async fn run_for_deleted() -> Result<(), anyhow::Error> {
    if get_file_meta(&CONFIG.common.data_wal_dir).is_err() {
        // there is no local wal files, no need upgrade
        return Ok(());
    }

    // move files from wal for disk
    if let Err(e) = files::json::move_files_to_storage().await {
        log::error!("Error moving disk json files to remote: {}", e);
    }
    if let Err(e) = files::parquet::move_files_to_storage().await {
        log::error!("Error moving disk parquet files to remote: {}", e);
    }

    // move file_list from wal for disk
    if let Err(e) = file_list::move_file_list_to_storage(false).await {
        log::error!("Error moving disk files to remote: {}", e);
    }

    // load stream list
    db::schema::cache().await?;
    let max_time = chrono::Utc::now().timestamp_micros();
    let orgs = db::schema::list_organizations_from_cache();
    for org_id in orgs.iter() {
        let files = crate::service::compact::file_list_deleted::load_prefix_from_s3(org_id).await?;
        if files.is_empty() {
            continue;
        }
        let files = files
            .values()
            .flatten()
            .map(|file| file.to_owned())
            .collect::<Vec<_>>();
        if let Err(e) = infra_file_list::batch_add_deleted(org_id, max_time, &files).await {
            log::error!("load file_list_deleted into db err: {}", e);
            continue;
        }
    }
    Ok(())
}
