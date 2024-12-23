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

//! These models define the schemas of HTTP request and response JSON bodies in
//! folders API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// HTTP request body for `CreateFolder` endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequestBody {
    pub name: String,
    pub description: String,
}

/// HTTP response body for `CreateFolder` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct CreateFolderResponseBody(pub Folder);

/// HTTP response body for `GetFolder` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetFolderResponseBody(pub Folder);

/// HTTP request body for `UpdateFolder` endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateFolderRequestBody(pub Folder);

/// HTTP response body for `ListFolder` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListFoldersResponseBody {
    pub list: Vec<Folder>,
}

/// Indicates the type of data that the folder can contain.
#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FolderType {
    Dashboards,
    Alerts,
}

/// Common folder fields used in HTTP request and response bodies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    #[serde(default)]
    pub folder_id: String,
    pub name: String,
    pub description: String,
}

impl From<CreateFolderRequestBody> for config::meta::folder::Folder {
    fn from(value: CreateFolderRequestBody) -> Self {
        Self {
            folder_id: String::default(),
            name: value.name,
            description: value.description,
        }
    }
}

impl From<config::meta::folder::Folder> for CreateFolderResponseBody {
    fn from(value: config::meta::folder::Folder) -> Self {
        Self(value.into())
    }
}

impl From<config::meta::folder::Folder> for GetFolderResponseBody {
    fn from(value: config::meta::folder::Folder) -> Self {
        Self(value.into())
    }
}

impl From<UpdateFolderRequestBody> for config::meta::folder::Folder {
    fn from(value: UpdateFolderRequestBody) -> Self {
        value.0.into()
    }
}

impl From<FolderType> for infra::table::folders::FolderType {
    fn from(value: FolderType) -> Self {
        match value {
            FolderType::Dashboards => Self::Dashboards,
            FolderType::Alerts => Self::Alerts,
        }
    }
}

impl From<Vec<config::meta::folder::Folder>> for ListFoldersResponseBody {
    fn from(value: Vec<config::meta::folder::Folder>) -> Self {
        Self {
            list: value.into_iter().map(Folder::from).collect(),
        }
    }
}

impl From<config::meta::folder::Folder> for Folder {
    fn from(value: config::meta::folder::Folder) -> Self {
        Self {
            folder_id: value.folder_id,
            name: value.name,
            description: value.description,
        }
    }
}

impl From<Folder> for config::meta::folder::Folder {
    fn from(value: Folder) -> Self {
        Self {
            folder_id: value.folder_id,
            name: value.name,
            description: value.description,
        }
    }
}
