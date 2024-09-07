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

use std::{collections::HashMap, io::Error};

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse};

use crate::{
    common::{
        meta::{self, pipelines::PipeLine},
        utils::http::get_stream_type_from_request,
    },
    service::format_stream_name,
};

/// CreatePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "createPipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = PipeLine, description = "Pipeline data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/streams/{stream_name}/pipelines")]
pub async fn save_pipeline(
    path: web::Path<(String, String)>,
    pipeline: web::Json<PipeLine>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let mut pipeline = pipeline.into_inner();
    pipeline.name = pipeline.name.trim().to_string();
    pipeline.stream_name = stream_name;
    pipeline.stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };
    if let Some(ref mut routing) = &mut pipeline.routing {
        let keys_to_update: Vec<_> = routing.keys().cloned().collect();
        for key in keys_to_update {
            let value = routing.remove(&key).unwrap();
            if value.is_empty() {
                return Ok(
                    HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                        http::StatusCode::BAD_REQUEST.into(),
                        format!("Routing condition for {} is empty", key),
                    )),
                );
            }
            let formatted_key = format_stream_name(&key);
            routing.insert(formatted_key, value);
        }
    }
    crate::service::pipelines::save_pipeline(org_id, pipeline).await
}

/// ListPipelines
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "listPipelines",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = PipelineList),
    )
)]
#[get("/{org_id}/pipelines")]
async fn list_pipelines(
    org_id: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let mut _permitted = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = _req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            "logs",
        )
        .await
        {
            Ok(list) => {
                _permitted = list;
            }
            Err(e) => {
                return Ok(crate::common::meta::http::HttpResponse::forbidden(
                    e.to_string(),
                ));
            }
        }
        // Get List of allowed objects ends
    }

    crate::service::pipelines::list_pipelines(org_id.into_inner(), _permitted).await
}

/// DeletePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "deletePipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Pipeline name"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/streams/{stream_name}/pipelines/{name}")]
async fn delete_pipeline(
    path: web::Path<(String, String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(crate::common::meta::http::HttpResponse::bad_request(e));
        }
    };
    let pipeline_source =
        config::meta::stream::StreamParams::new(&org_id, &stream_name, stream_type);
    crate::service::pipelines::delete_pipeline(&name, pipeline_source).await
}

/// UpdatePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "updatePipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Pipeline name"),
    ),
    request_body(content = PipeLine, description = "Pipeline data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/streams/{stream_name}/pipelines/{name}")]
pub async fn update_pipeline(
    path: web::Path<(String, String, String)>,
    pipeline: web::Json<PipeLine>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Ok(crate::common::meta::http::HttpResponse::bad_request(e));
        }
    };
    let name = name.trim();
    let mut pipeline = pipeline.into_inner();
    pipeline.name = name.to_string();
    pipeline.stream_type = stream_type;
    pipeline.stream_name = stream_name;

    if let Some(ref mut routing) = &mut pipeline.routing {
        let keys_to_update: Vec<_> = routing.keys().cloned().collect();
        for key in keys_to_update {
            let value = routing.remove(&key).unwrap();
            if value.is_empty() {
                return Ok(
                    HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                        http::StatusCode::BAD_REQUEST.into(),
                        format!("Routing condition for {} is empty", key),
                    )),
                );
            }
            let formatted_key = format_stream_name(&key);
            routing.insert(formatted_key, value);
        }
    }
    crate::service::pipelines::update_pipeline(&org_id, pipeline).await
}
