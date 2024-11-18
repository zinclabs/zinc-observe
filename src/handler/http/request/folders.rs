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

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use config::meta::folder::Folder;

use crate::service::folders;

/// CreateFolder
#[utoipa::path(
    context_path = "/api",
    tag = "Folders",
    operation_id = "CreateFolder",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(
        content = Folder,
        description = "Folder details",
        example = json!({
            "name": "Infrastructure",
            "description": "Traffic patterns and network performance of the infrastructure",
        }),
    ),
    responses(
        (status = StatusCode::CREATED, description = "Folder created", body = Folder),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error", body = HttpResponse),
    ),
)]
#[post("/{org_id}/folders")]
pub async fn create_folder(
    path: web::Path<String>,
    folder: web::Json<Folder>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    folders::save_folder(&org_id, folder.into_inner(), false).await
}

/// UpdateFolder
#[utoipa::path(
    context_path = "/api",
    tag = "Folders",
    operation_id = "UpdateFolder",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("folder_id" = String, Path, description = "Folder name"),
    ),
    request_body(
        content = Folder,
        description = "Folder details",
        example = json!({
            "title": "Infra",
            "description": "Traffic patterns and network performance of the infrastructure",
        }),
    ),
    responses(
        (status = StatusCode::CREATED, description = "Folder updated", body = Folder),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error", body = HttpResponse),
    ),
)]
#[put("/{org_id}/folders/{folder_id}")]
pub async fn update_folder(
    path: web::Path<(String, String)>,
    folder: web::Json<Folder>,
) -> Result<HttpResponse, Error> {
    let (org_id, folder_id) = path.into_inner();
    folders::update_folder(&org_id, &folder_id, folder.into_inner()).await
}

/// ListFolders
#[utoipa::path(
    context_path = "/api",
    tag = "Folders",
    operation_id = "ListFolders",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = StatusCode::OK, body = FolderList),
    ),
)]
#[get("/{org_id}/folders")]
pub async fn list_folders(
    path: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();

    let mut _permitted = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = _req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            "dfolder",
        )
        .await
        {
            Ok(stream_list) => {
                _permitted = stream_list;
            }
            Err(e) => {
                return Ok(crate::common::meta::http::HttpResponse::forbidden(
                    e.to_string(),
                ));
            }
        }
        // Get List of allowed objects ends
    }

    folders::list_folders(&org_id, _permitted).await
}

/// GetFolder
#[utoipa::path(
    context_path = "/api",
    tag = "Folders",
    operation_id = "GetFolder",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("folder_id" = String, Path, description = "Folder ID"),
    ),
    responses(
        (status = StatusCode::OK, body = Folder),
        (status = StatusCode::NOT_FOUND, description = "Folder not found", body = HttpResponse),
    ),
)]
#[get("/{org_id}/folders/{folder_id}")]
pub async fn get_folder(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, folder_id) = path.into_inner();
    let resp = folders::get_folder(&org_id, &folder_id).await;
    Ok(resp)
}

/// DeleteFolder
#[utoipa::path(
    context_path = "/api",
    tag = "Folders",
    operation_id = "DeleteFolder",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("folder_id" = String, Path, description = "Folder ID"),
    ),
    responses(
        (status = StatusCode::OK, description = "Success", body = HttpResponse),
        (status = StatusCode::NOT_FOUND, description = "NotFound", body = HttpResponse),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Error", body = HttpResponse),
    ),
)]
#[delete("/{org_id}/folders/{folder_id}")]
async fn delete_folder(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, folder_id) = path.into_inner();
    folders::delete_folder(&org_id, &folder_id).await
}
