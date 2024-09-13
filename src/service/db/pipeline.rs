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

/// Returns all streams with existing pipelines.
pub async fn list_streams_with_pipeline(org: &str) -> Result<Vec<StreamParams>> {
    infra_pipeline::list_streams_with_pipeline(org)
        .await
        .map_err(|e| {
            log::error!("Error getting streams with pipeline for org({org}): {}", e);
            anyhow::anyhow!("Error getting streams with pipeline for org({org}): {}", e)
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
