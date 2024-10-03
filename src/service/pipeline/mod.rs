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

use std::io::Error;

use actix_web::{
    http::{self, StatusCode},
    HttpResponse,
};
use config::meta::{
    pipeline::{components::PipelineSource, Pipeline, PipelineList},
    search::SearchEventType,
    stream::ListStreamParams,
};

use super::db;
use crate::common::{
    meta::{authz::Authz, http::HttpResponse as MetaHttpResponse},
    utils::auth::{remove_ownership, set_ownership},
};

pub mod execution;

#[tracing::instrument(skip(pipeline))]
pub async fn save_pipeline(mut pipeline: Pipeline) -> Result<HttpResponse, Error> {
    // validate pipeline
    if let Err(e) = pipeline.validate() {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            format!("Invalid Pipeline: {e}"),
        )));
    }

    // Save DerivedStream details if there's any
    if let PipelineSource::Scheduled(ref mut derived_stream) = &mut pipeline.source {
        derived_stream.query_condition.search_event_type = Some(SearchEventType::DerivedStream);
        // save derived_stream to triggers table
        if let Err(e) = super::alerts::derived_streams::save(
            derived_stream.clone(),
            &pipeline.name,
            &pipeline.id,
        )
        .await
        {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Failed to save DerivedStream details error: {}", e),
            )));
        }
    }

    match db::pipeline::set(&pipeline).await {
        Err(error) => Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                error.to_string(),
            )),
        ),
        Ok(_) => {
            set_ownership(&pipeline.org, "pipelines", Authz::new(&pipeline.id)).await;
            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "Pipeline saved successfully".to_string(),
            )))
        }
    }
}

#[tracing::instrument(skip(pipeline))]
pub async fn update_pipeline(mut pipeline: Pipeline) -> Result<HttpResponse, Error> {
    match db::pipeline::get_by_id(&pipeline.id).await {
        Err(_err) => {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                format!("Existing Pipeline with ID {} not found", pipeline.id),
            )));
        }
        Ok(existing_pipeline) => {
            if existing_pipeline == pipeline {
                return Ok(HttpResponse::Ok().json("No changes found".to_string()));
            }
            // check version
            if existing_pipeline.version != pipeline.version {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    format!(
                        "Pipeline with ID {} modified by someone else. Please refresh",
                        pipeline.id
                    ),
                )));
            }
            // if the source is changed, check if the new source exists in another pipeline
            if existing_pipeline.source != pipeline.source {
                if let Ok(similar_pl) = db::pipeline::get_with_same_source_stream(&pipeline).await {
                    return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                        http::StatusCode::BAD_REQUEST.into(),
                        format!(
                            "The new source already exists in another pipeline in org: {} with name: {}",
                            similar_pl.org,
                            similar_pl.name
                        ),
                    )));
                }
            }
        }
    };

    // validate pipeline
    if let Err(e) = pipeline.validate() {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            format!("Invalid Pipeline: {e}"),
        )));
    }

    // update the pipeline version
    pipeline.version += 1;

    // Save DerivedStream details if there's any
    if let PipelineSource::Scheduled(ref mut derived_stream) = &mut pipeline.source {
        derived_stream.query_condition.search_event_type = Some(SearchEventType::DerivedStream);
        // save derived_stream to triggers table
        if let Err(e) = super::alerts::derived_streams::save(
            derived_stream.clone(),
            &pipeline.name,
            &pipeline.id,
        )
        .await
        {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Failed to save DerivedStream details error: {}", e),
            )));
        }
    }

    match db::pipeline::update(pipeline).await {
        Err(error) => Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                error.to_string(),
            )),
        ),
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline updated successfully".to_string(),
        ))),
    }
}

#[tracing::instrument]
pub async fn list_pipelines(
    org_id: String,
    permitted: Option<Vec<String>>,
) -> Result<HttpResponse, Error> {
    if let Ok(pipelines) = db::pipeline::list_by_org(&org_id).await {
        let mut result = Vec::new();
        for pipeline in pipelines {
            if permitted.is_none()
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("pipelines:{}", pipeline.id))
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("pipelines:_all_{}", org_id))
            {
                result.push(pipeline)
            }
        }

        Ok(HttpResponse::Ok().json(PipelineList { list: result }))
    } else {
        Ok(HttpResponse::Ok().json(PipelineList { list: vec![] }))
    }
}

#[tracing::instrument]
pub async fn list_streams_with_pipeline(org: &str) -> Result<HttpResponse, Error> {
    match db::pipeline::list_streams_with_pipeline(org).await {
        Ok(stream_params) => Ok(HttpResponse::Ok().json(ListStreamParams {
            list: stream_params,
        })),
        Err(_) => Ok(HttpResponse::Ok().json(PipelineList { list: vec![] })),
    }
}

#[tracing::instrument]
pub async fn delete_pipeline(pipeline_id: &str) -> Result<HttpResponse, Error> {
    let existing_pipeline = match db::pipeline::get_by_id(pipeline_id).await {
        Ok(pipeline) => pipeline,
        Err(_) => {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                format!("Pipeline with ID {pipeline_id} not found"),
            )));
        }
    };

    // delete DerivedStream details if there's any
    if let PipelineSource::Scheduled(derived_stream) = existing_pipeline.source {
        if let Err(error) = super::alerts::derived_streams::delete(
            derived_stream,
            &existing_pipeline.name,
            &existing_pipeline.id,
        )
        .await
        {
            return Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    error.to_string(),
                )),
            );
        }
    }

    let result = db::pipeline::delete(pipeline_id).await;
    match result {
        Ok(_) => {
            remove_ownership(
                &existing_pipeline.org,
                "pipelines",
                Authz::new(&existing_pipeline.id),
            )
            .await;
            Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "Pipeline deleted".to_string(),
            )))
        }
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}
