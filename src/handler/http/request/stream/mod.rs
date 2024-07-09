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
    collections::HashMap,
    io::{Error, ErrorKind},
};

use actix_web::{delete, get, http, put, web, HttpRequest, HttpResponse, Responder};
use config::meta::stream::{StreamSettings, StreamType};

use crate::{
    common::{
        meta::{
            self,
            http::HttpResponse as MetaHttpResponse,
            stream::{ListStream, StreamDeleteFields},
        },
        utils::http::get_stream_type_from_request,
    },
    service::{format_stream_name, stream},
};

/// GetSchema
#[utoipa::path(
    context_path = "/api",
    tag = "Streams",
    operation_id = "StreamSchema",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = Stream),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/streams/{stream_name}/schema")]
async fn schema(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };
    let stream_type = stream_type.unwrap_or(StreamType::Logs);
    stream::get_stream(&org_id, &stream_name, stream_type).await
}

/// UpdateStreamSettings
#[utoipa::path(
    context_path = "/api",
    tag = "Streams",
    operation_id = "StreamSettings",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
    ),
    request_body(content = StreamSettings, description = "Stream settings", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/streams/{stream_name}/settings")]
async fn settings(
    path: web::Path<(String, String)>,
    settings: web::Json<StreamSettings>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, mut stream_name) = path.into_inner();
    if !config::get_config().common.skip_formatting_bulk_stream_name {
        stream_name = format_stream_name(&stream_name);
    }
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => {
            if let Some(s_type) = v {
                if s_type == StreamType::EnrichmentTables || s_type == StreamType::Index {
                    return Ok(
                        HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                            http::StatusCode::BAD_REQUEST.into(),
                            format!("Stream type '{}' not allowed", s_type),
                        )),
                    );
                }
                Some(s_type)
            } else {
                v
            }
        }
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };

    let stream_type = stream_type.unwrap_or(StreamType::Logs);
    stream::save_stream_settings(&org_id, &stream_name, stream_type, settings.into_inner()).await
}

/// DeleteStreamFields
#[utoipa::path(
    context_path = "/api",
    tag = "Streams",
    operation_id = "StreamDeleteFields",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
    ),
    request_body(content = StreamDeleteFields, description = "Stream delete fields", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/streams/{stream_name}/delete_fields")]
async fn delete_fields(
    path: web::Path<(String, String)>,
    fields: web::Json<StreamDeleteFields>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };
    match stream::delete_fields(
        &org_id,
        &stream_name,
        stream_type,
        &fields.into_inner().fields,
    )
    .await
    {
        Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "fields deleted".to_string(),
        ))),
        Err(e) => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            e.to_string(),
        ))),
    }
}

/// DeleteStream
#[utoipa::path(
    context_path = "/api",
    tag = "Streams",
    operation_id = "StreamDelete",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/streams/{stream_name}")]
async fn delete(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };
    let stream_type = stream_type.unwrap_or(StreamType::Logs);
    stream::delete_stream(&org_id, &stream_name, stream_type).await
}

/// ListStreams
#[utoipa::path(
    context_path = "/api",
    tag = "Streams",
    operation_id = "StreamList",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = ListStream),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/streams")]
async fn list(org_id: web::Path<String>, req: HttpRequest) -> impl Responder {
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };

    let fetch_schema = match query.get("fetchSchema") {
        Some(s) => match s.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    " 'fetchSchema' query param with value 'true' or 'false' allowed",
                ));
            }
        },
        None => false,
    };
    let mut _stream_list_from_rbac = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = req.headers().get("user_id").unwrap();
        if let Some(s_type) = &stream_type {
            if !s_type.eq(&StreamType::EnrichmentTables) && !s_type.eq(&StreamType::Metadata) {
                match crate::handler::http::auth::validator::list_objects_for_user(
                    &org_id,
                    user_id.to_str().unwrap(),
                    "GET",
                    &s_type.to_string(),
                )
                .await
                {
                    Ok(stream_list) => {
                        _stream_list_from_rbac = stream_list;
                    }
                    Err(e) => {
                        return Ok(crate::common::meta::http::HttpResponse::forbidden(
                            e.to_string(),
                        ));
                    }
                }
            }
        }
        // Get List of allowed objects ends
    }

    let mut indices = stream::get_streams(
        org_id.as_str(),
        stream_type,
        fetch_schema,
        _stream_list_from_rbac,
    )
    .await;
    indices.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(HttpResponse::Ok().json(ListStream { list: indices }))
}

#[delete("/{org_id}/streams/{stream_name}/cache/results")]
async fn delete_stream_cache(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    if !config::get_config().common.result_cache_enabled {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            "Result Cache is disabled".to_string(),
        )));
    }
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(
                HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    e.to_string(),
                )),
            );
        }
    };
    let stream_type = stream_type.unwrap_or(StreamType::Logs);
    let path = if stream_name.eq("_all") {
        org_id
    } else {
        format!("{}/{}/{}", org_id, stream_type, stream_name)
    };

    match crate::service::search::cluster::cacher::delete_cached_results(path).await {
        true => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "cache deleted".to_string(),
        ))),
        false => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            "Error deleting cache, please retry".to_string(),
        ))),
    }
}
