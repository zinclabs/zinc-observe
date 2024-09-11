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
};

use super::db;
use crate::common::{
    // infra::config::STREAM_FUNCTIONS,
    meta::http::HttpResponse as MetaHttpResponse,
};

#[tracing::instrument(skip(pipeline))]
pub async fn save_pipeline(mut pipeline: Pipeline) -> Result<HttpResponse, Error> {
    // TODO(taiming): check similar pipeline exists?
    // depends on this function is called
    // if check_existing_pipeline(
    //     &org_id,
    //     pipeline.stream_type,
    //     &pipeline.stream_name,
    //     &pipeline.name,
    // )
    // .await
    // .is_some()
    // {
    //     return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
    //         StatusCode::BAD_REQUEST.into(),
    //         "Pipeline already exits".to_string(),
    //     )));
    // }

    // validate pipeline
    if let Err(e) = pipeline.validate() {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            format!("Invalid Pipeline: {e}"),
        )));
    }

    // Save DerivedStream details if there's any
    if let PipelineSource::Query(ref mut derived_stream) = &mut pipeline.source {
        if derived_stream.trigger_condition.period == 0 {
            // Invalid trigger condition
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Invalid Pipeline: DerivedStream source's TriggerCondition period missing or is 0"
                    .to_string(),
            )));
        }
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

    match db::pipeline::set(pipeline).await {
        Err(error) => Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                error.to_string(),
            )),
        ),
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline saved successfully".to_string(),
        ))),
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
            if existing_pipeline == pipeline {
                return Ok(HttpResponse::Ok().json(pipeline));
            }
            // now check if there's a pipeline with the same source and structure
            if let Ok(similar_pl) = db::pipeline::get_by_src_and_struct(&pipeline).await {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    format!(
                        "A pipeline with the same source and structure exists in org: {} with name: {}",
                        similar_pl.org,
                        similar_pl.name
                    ),
                )));
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
    if let PipelineSource::Query(ref mut derived_stream) = &mut pipeline.source {
        if derived_stream.trigger_condition.period == 0 {
            // Invalid trigger condition
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Invalid Pipeline: DerivedStream source's TriggerCondition period missing or is 0"
                    .to_string(),
            )));
        }
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

    match db::pipeline::set(pipeline).await {
        Err(error) => Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::message(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                error.to_string(),
            )),
        ),
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline saved successfully".to_string(),
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
                // QUESTION(taiming): what's this check for?
                // || permitted
                //     .as_ref()
                //     .unwrap()
                //     .contains(&format!("logs:{}", pipeline.stream_name))
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("logs:_all_{}", org_id))
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
    if let PipelineSource::Query(derived_stream) = existing_pipeline.source {
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
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline deleted".to_string(),
        ))),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}
