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

use std::io::Error as StdErr;

use actix_web::{delete, get, post, put, web, HttpResponse};
use config::utils::json;
use infra::errors::{DbError, Error};
#[cfg(feature = "enterprise")]
use {
    actix_multipart::Multipart,
    futures::{StreamExt, TryStreamExt},
    o2_enterprise::enterprise::common::settings,
};
#[cfg(feature = "enterprise")]
use std::collections::HashMap;

use crate::{
    common::meta::{
        http::HttpResponse as MetaHttpResponse,
        organization::{OrganizationSetting, OrganizationSettingResponse},
    },
    service::db::organization::{get_org_setting, set_org_setting},
};

/// Organization specific settings
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "OrganizationSettingCreate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = OrganizationSetting, description = "Organization settings", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/settings")]
async fn create(
    path: web::Path<String>,
    settings: web::Json<OrganizationSetting>,
) -> Result<HttpResponse, StdErr> {
    if settings.scrape_interval == 0 {
        return Ok(MetaHttpResponse::bad_request(
            "scrape_interval should be a positive value",
        ));
    }

    let org_id = path.into_inner();
    match set_org_setting(&org_id, &settings).await {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e.to_string().as_str())),
    }
}

/// Retrieve organization specific settings
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "OrganizationSettingGet",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/settings")]
async fn get(path: web::Path<String>) -> Result<HttpResponse, StdErr> {
    let org_id = path.into_inner();
    match get_org_setting(&org_id).await {
        Ok(s) => {
            let data: OrganizationSetting = json::from_slice(&s).unwrap();
            Ok(HttpResponse::Ok().json(OrganizationSettingResponse { data }))
        }
        Err(err) => {
            if let Error::DbError(DbError::KeyNotExists(_e)) = &err {
                let setting = OrganizationSetting::default();
                if let Ok(()) = set_org_setting(&org_id, &setting).await {
                    return Ok(
                        HttpResponse::Ok().json(OrganizationSettingResponse { data: setting })
                    );
                }
            };
            Ok(MetaHttpResponse::bad_request(&err))
        }
    }
}

#[cfg(feature = "enterprise")]
#[post("/{org_id}/settings/logo")]
async fn upload_logo(mut payload: Multipart) -> Result<HttpResponse, StdErr> {
    match payload.try_next().await {
        Ok(field) => {
            let mut data: Vec<u8> = Vec::<u8>::new();
            if let Some(mut field) = field {
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.unwrap();
                    data.extend(chunk);
                }
                if data.is_empty() {
                    return Ok(MetaHttpResponse::bad_request("Image data not present"));
                }

                match settings::upload_logo(data).await {
                    Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
                    Err(e) => Ok(MetaHttpResponse::bad_request(e)),
                }
            } else {
                Ok(MetaHttpResponse::bad_request("Payload file not present"))
            }
        }
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/settings/logo")]
async fn upload_logo() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/settings/logo")]
async fn delete_logo() -> Result<HttpResponse, StdErr> {
    match settings::delete_logo().await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/settings/logo")]
async fn delete_logo() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[post("/{org_id}/settings/logo/text")]
async fn set_logo_text(body: web::Bytes) -> Result<HttpResponse, StdErr> {
    match settings::set_logo_text(body).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/settings/logo/text")]
async fn set_logo_text() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/settings/logo/text")]
async fn delete_logo_text() -> Result<HttpResponse, StdErr> {
    match settings::delete_logo_text().await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/settings/logo/text")]
async fn delete_logo_text() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

// custom settings api

#[cfg(not(feature = "enterprise"))]
#[get("/{org_id}/settings/custom")]
pub async fn get_custom_settings()->Result<HttpResponse,StdErr>{
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/settings/custom")]
pub async fn set_custom_settings()->Result<HttpResponse,StdErr>{
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(not(feature = "enterprise"))]
#[put("/{org_id}/settings/custom")]
pub async fn update_custom_settings()->Result<HttpResponse,StdErr>{
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/settings/custom")]
pub async fn delete_custom_settings()->Result<HttpResponse,StdErr>{
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

/// This will retrieve the custom settings for the specified org
/// and return them as a json object
#[cfg(feature = "enterprise")]
#[get("/{org_id}/settings/custom")]
pub async fn get_custom_settings(path: web::Path<String>)->Result<HttpResponse,StdErr>{
    let org_id = path.into_inner();
    match settings::get_custom_settings(&org_id).await {
        Ok(settings) => Ok(HttpResponse::Ok().json(serde_json::json!(settings))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}


/// This expects the settings as object with string kv pairs
/// and will store those in db as is.
#[cfg(feature = "enterprise")]
#[post("/{org_id}/settings/custom")]
pub async fn set_custom_settings(
    path: web::Path<String>, 
    settings: web::Json<HashMap<String, settings::CustomSettingEntry>>
)->Result<HttpResponse,StdErr>{
    let org_id = path.into_inner();
    let settings = settings.into_inner();
    match settings::set_custom_settings(&org_id, &settings).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

#[cfg(feature = "enterprise")]
#[put("/{org_id}/settings/custom")]
pub async fn update_custom_settings(
    path: web::Path<String>, 
    settings: web::Json<HashMap<String, settings::CustomSettingEntry>>
)->Result<HttpResponse,StdErr>{
    let org_id = path.into_inner();
    let settings = settings.into_inner();
    match settings::update_custom_settings(&org_id, settings).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/settings/custom")]
pub async fn delete_custom_settings(
    path: web::Path<String>, 
    keys: web::Json<Vec<String>>
)->Result<HttpResponse,StdErr>{
    let org_id = path.into_inner();
    let keys = keys.into_inner();
    match settings::delete_custom_settings(&org_id, &keys).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}