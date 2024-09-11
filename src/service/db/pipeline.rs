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

// use std::sync::Arc;

use anyhow::Result;
use config::meta::{pipeline::Pipeline, stream::StreamParams};
use infra::pipeline::{self as infra_pipeline};

// use crate::common::infra::config::STREAM_PIPELINES;

/// Stores a new pipeline to database.
///
/// Pipeline validation should be handled by the caller.
pub async fn set(pipeline: Pipeline) -> Result<()> {
    match infra_pipeline::put(pipeline).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error saving pipeline: {}", e);
            return Err(anyhow::anyhow!("Error saving pipeline: {}", e));
        }
    }
    Ok(())
}
/// Updates a pipeline entry with the sane values.
///
/// Pipeline validation should be handled by the caller.
pub async fn update(pipeline: Pipeline) -> Result<()> {
    match infra_pipeline::update(pipeline).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error updating pipeline: {}", e);
            return Err(anyhow::anyhow!("Error updating pipeline: {}", e));
        }
    }
    Ok(())
}

/// Returns all pipelines associated with a stream.
///
/// Used to get all pipelines associated with a stream during the Stream's ingestion time.
/// `org` is the organization the pipelines belong to, not the pipeline source Stream's org_id
pub async fn get_by_stream(org: &str, stream_params: &StreamParams) -> Result<Vec<Pipeline>> {
    infra_pipeline::get_by_stream(org, stream_params)
        .await
        .map_err(|e| {
            log::debug!(
                "Error getting pipelines for org/stream({org}/{stream_params}): {}",
                e
            );
            anyhow::anyhow!(
                "Error getting pipelines for org/stream({org}/{stream_params}): {}",
                e
            )
        })
}

/// Returns the pipeline by id.
///
/// Used to get the pipeline associated with the ID when scheduled job is ran.
pub async fn get_by_id(pipeline_id: &str) -> Result<Pipeline> {
    infra_pipeline::get_by_id(pipeline_id).await.map_err(|e| {
        log::error!("Error getting pipeline with ID({pipeline_id}): {}", e);
        anyhow::anyhow!("Error getting pipeline with ID({pipeline_id}): {}", e)
    })
}

/// Finds the pipeline with the same source
///
/// Used to validate if a duplicate pipeline exists.
pub async fn get_with_same_source_stream(pipeline: &Pipeline) -> Result<Pipeline> {
    infra_pipeline::get_with_same_source_stream(pipeline)
        .await
        .map_err(|_| anyhow::anyhow!("No pipeline with the same source found"))
}

/// Lists all pipelines across all orgs.
pub async fn list() -> Result<Vec<Pipeline>> {
    infra_pipeline::list().await.map_err(|e| {
        log::debug!("Error listing pipelines for all orgs: {}", e);
        anyhow::anyhow!("Error listing pipelines for all orgs: {}", e)
    })
}

/// Lists all pipelines for a given organization.
pub async fn list_by_org(org: &str) -> Result<Vec<Pipeline>> {
    infra_pipeline::list_by_org(org).await.map_err(|e| {
        log::debug!("Error listing pipelines for org({org}): {}", e);
        anyhow::anyhow!("Error listing pipelines for org({org}): {}", e)
    })
}

/// Deletes a pipeline by ID.
pub async fn delete(pipeline_id: &str) -> Result<()> {
    infra_pipeline::delete(pipeline_id).await.map_err(|e| {
        log::error!("Error deleting pipeline with ID({pipeline_id}): {}", e);
        anyhow::anyhow!("Error deleting pipeline with ID({pipeline_id}): {}", e)
    })
}

// TODO(taiming): implement database watch
pub async fn watch() -> Result<(), anyhow::Error> {
    // let key = "/pipeline/";
    // let cluster_coordinator = db::get_coordinator().await;
    // let mut events = cluster_coordinator.watch(key).await?;
    // let events = Arc::get_mut(&mut events).unwrap();
    // log::info!("Start watching pipeline");
    // loop {
    //     let ev = match events.recv().await {
    //         Some(ev) => ev,
    //         None => {
    //             log::error!("watch_pipelines: event channel closed");
    //             break;
    //         }
    //     };
    //     match ev {
    //         db::Event::Put(ev) => {
    //             let item_key = ev.key.strip_prefix(key).unwrap();
    //             let org_id = &item_key[0..item_key.find('/').unwrap()];
    //             let item_value: PipeLine = if config::get_config().common.meta_store_external {
    //                 match db::get(&ev.key).await {
    //                     Ok(val) => match json::from_slice(&val) {
    //                         Ok(val) => val,
    //                         Err(e) => {
    //                             log::error!("Error getting value: {}", e);
    //                             continue;
    //                         }
    //                     },
    //                     Err(e) => {
    //                         log::error!("Error getting value: {}", e);
    //                         continue;
    //                     }
    //                 }
    //             } else {
    //                 json::from_slice(&ev.value.unwrap()).unwrap()
    //             };
    //             let key = format!(
    //                 "{org_id}/{}/{}",
    //                 item_value.stream_type, item_value.stream_name
    //             );
    //             STREAM_PIPELINES.insert(key.to_owned(), item_value);
    //         }
    //         db::Event::Delete(ev) => {
    //             let item_key = ev.key.strip_prefix(key).unwrap();
    //             let removal_key = match item_key.rfind('/') {
    //                 Some(index) => &item_key[0..index],
    //                 None => item_key,
    //             };

    //             STREAM_PIPELINES.remove(removal_key);
    //         }
    //         db::Event::Empty => {}
    //     }
    // }
    Ok(())
}

/// Gets all pipelines with streams
pub async fn cache() -> Result<()> {
    todo!("taiming")
    // let key = "/pipeline/";
    // let ret = db::list(key).await?;
    // for (item_key, item_value) in ret {
    //     let item_key = item_key.strip_prefix(key).unwrap();
    //     let json_val: PipeLine = json::from_slice(&item_value).unwrap();
    //     let org_id = &item_key[0..item_key.find('/').unwrap()];
    //     let key = format!("{org_id}/{}/{}", json_val.stream_type, json_val.stream_name);

    //     STREAM_PIPELINES.insert(key.to_string(), json_val);
    // }

    // log::info!("Pipelines Cached");
    // Ok(())
}
