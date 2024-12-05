// Copyright 2024 OpenObserve Inc.
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

use actix_http::StatusCode;
use actix_web::{delete, get, http, post, web, HttpRequest, HttpResponse};

use crate::{
    common::meta::{self, http::HttpResponse as MetaHttpResponse},
    kms::{Key, KeyAddRequest, KeyGetResponse},
};

/// Store a key credential in db
#[utoipa::path(
    post,
    context_path = "/api",
    request_body(
        content = KeyAddRequest,
        description = "Key data to add",
        content_type = "application/json",
        // example = json!({            
        // })
    ),
    responses(
        (
            status = 200,
            description = "Empty response",
            body = (),
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
        (status = 400, description = "Invalid request", content_type = "application/json")
    ),
    tag = "Key"
)]
#[post("/{org_id}/keys")]
pub async fn store(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let req: KeyAddRequest = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };

    let user_id = in_req
        .headers()
        .get("user_id")
        .map(|v| v.to_str().unwrap_or("").to_string())
        .unwrap();

    let key = Key {
        created_at: chrono::Utc::now().timestamp_micros(),
        created_by: user_id,
        name: req.name,
        credentials: req.credentials,
    };

    match infra::table::keys::add(org_id.as_str(), key.into()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            log::error!("error while storing key : {e}",);
            Ok(
                HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            )
        }
    }
}

/// get credentials for given key name
#[utoipa::path(
    get,
    context_path = "/api",
    params(
        ("key_name" = String, Path, description = "Name of the key to retrieve", example = "test_key")
    ),
    responses(
        (
            status = 200,
            description = "Key info",
            body = KeyGetResponse,
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
        (status = 404, description = "Key not found", content_type = "text/plain")
    ),
    tag = "Key"
)]
#[get("/{org_id}/keys/{key_name}")]
pub async fn get(
    _req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (org_id, key_name) = path.into_inner();
    match infra::table::keys::get(&org_id, &key_name).await {
        Ok(v) => match v {
            Some(model) => {
                let k: Key = model.into();
                let res = KeyGetResponse {
                    name: key_name,
                    key_type: k.credentials.get_type(),
                    credentials: k.credentials,
                };
                Ok(HttpResponse::Ok().json(res))
            }
            None => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                http::StatusCode::NOT_FOUND.into(),
                format!("Key {key_name} not found"),
            ))),
        },
        Err(e) => {
            log::error!("error while storing key {e}");
            Ok(
                HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            )
        }
    }
}

/// delete key credentials for given key name
#[utoipa::path(
    delete,
    context_path = "/api",
    params(
        ("key_name" = String, Path, description = "name of the key to delete", example = "test_key")
    ),
    responses(
        (
            status = 200,
            description = "Empty response",
            body = (),
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
    ),
    tag = "Keys"
)]
#[delete("/{org_id}/keys/{key_name}")]
pub async fn delete(
    _req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (org_id, key_name) = path.into_inner();
    match infra::table::keys::remove(&org_id, &key_name).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            log::error!("error while deleting key {key_name} : {e}");
            Ok(
                HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            )
        }
    }
}
