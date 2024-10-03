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
use config::meta::function::{StreamOrder, Transform};

use crate::common::{meta, utils::http::get_stream_type_from_request};

/// CreateFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "createFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = Transform, description = "Function data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/functions")]
pub async fn save_function(
    path: web::Path<String>,
    func: web::Json<Transform>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let mut transform = func.into_inner();
    transform.name = transform.name.trim().to_string();
    transform.function = transform.function.trim().to_string();
    crate::service::functions::save_function(org_id, transform).await
}

/// ListFunctions
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "listFunctions",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = FunctionList),
    )
)]
#[get("/{org_id}/functions")]
async fn list_functions(
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
            "function",
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

    crate::service::functions::list_functions(org_id.into_inner(), _permitted).await
}

/// DeleteFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "deleteFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Function name"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/functions/{name}")]
async fn delete_function(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    crate::service::functions::delete_function(org_id, name).await
}

/// UpdateFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "updateFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Function name"),
    ),
    request_body(content = Transform, description = "Function data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/functions/{name}")]
pub async fn update_function(
    path: web::Path<(String, String)>,
    func: web::Json<Transform>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    let name = name.trim();
    let mut transform = func.into_inner();
    transform.name = transform.name.trim().to_string();
    transform.function = transform.function.trim().to_string();
    crate::service::functions::update_function(&org_id, name, transform).await
}

/// ListStreamFunctions
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "listStreamFunctions",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = StreamFunctionsList),
    )
)]
#[get("/{org_id}/streams/{stream_name}/functions")]
async fn list_stream_functions(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
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
    crate::service::functions::list_stream_functions(&org_id, stream_type, &stream_name).await
}

/// RemoveStreamFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "removeStreamFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("name" = String, Path, description = "Function name"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/streams/{stream_name}/functions/{name}")]
async fn delete_stream_function(
    path: web::Path<(String, String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
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
    crate::service::functions::delete_stream_function(&org_id, stream_type, &stream_name, &name)
        .await
}

/// ApplyFunctionToStream
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "applyFunctionToStream",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "Stream name"),
        ("name" = String, Path, description = "Function name"),
    ),
    request_body(content = StreamOrder, description = "Function data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/streams/{stream_name}/functions/{name}")]
pub async fn add_function_to_stream(
    path: web::Path<(String, String, String)>,
    stream_order: web::Json<StreamOrder>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name, name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
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
    crate::service::functions::add_function_to_stream(
        &org_id,
        stream_type,
        &stream_name,
        &name,
        stream_order.into_inner(),
    )
    .await
}
