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

use config::{
    ider,
    meta::{
        dashboards::ListDashboardsParams,
        folder::{Folder, DEFAULT_FOLDER},
    },
};
use infra::table::{self, folders::FolderType};

use crate::common::{
    meta::authz::Authz,
    utils::auth::{remove_ownership, set_ownership},
};

/// Errors that can occur when interacting with folders.
#[derive(Debug, thiserror::Error)]
pub enum FolderError {
    /// An error that occurs while interacting with the database through the
    /// [infra] crate.
    #[error("InfraError# {0}")]
    InfraError(#[from] infra::errors::Error),

    /// An error that occurs when trying to set a folder name to the empty string.
    #[error("Folder name cannot be empty")]
    MissingName,

    /// An error that occurs when trying to update the special "default" folder.
    #[error("Can't update default folder")]
    UpdateDefaultFolder,

    /// An error that occurs when trying to delete a folder that contains dashboards.
    #[error("Folder contains dashboards. Please move/delete dashboards from folder.")]
    DeleteWithDashboards,

    /// An error that occurs when trying to delete a folder that cannot be found.
    #[error("Folder not found")]
    NotFound,

    /// An error occured trying to get the list of permitted folders in
    /// enterprise mode because no user_id was provided.
    #[error("user_id required to get permitted folders in enterprise mode")]
    PermittedFoldersMissingUser,

    /// An error occured trying to get the list of permitted folders in
    /// enterprise mode using the validator.
    #[error("PermittedFoldersValidator# {0}")]
    PermittedFoldersValidator(String),
}

#[tracing::instrument(skip(folder))]
pub async fn save_folder(
    org_id: &str,
    mut folder: Folder,
    folder_type: FolderType,
    is_internal: bool,
) -> Result<Folder, FolderError> {
    folder.name = folder.name.trim().to_string();
    if folder.name.is_empty() {
        return Err(FolderError::MissingName);
    }

    if !is_internal && folder.folder_id == DEFAULT_FOLDER {
        return Err(FolderError::UpdateDefaultFolder);
    }

    if folder.folder_id != DEFAULT_FOLDER {
        folder.folder_id = ider::generate();
    }

    let folder = table::folders::put(org_id, folder, folder_type).await?;
    set_ownership(org_id, "folders", Authz::new(&folder.folder_id)).await;
    Ok(folder)
}

#[tracing::instrument(skip(folder))]
pub async fn update_folder(
    org_id: &str,
    folder_id: &str,
    folder_type: FolderType,
    mut folder: Folder,
) -> Result<Folder, FolderError> {
    if folder_id.eq(DEFAULT_FOLDER) {
        return Err(FolderError::UpdateDefaultFolder);
    }

    folder.folder_id = folder_id.to_string();
    let folder = table::folders::put(org_id, folder, folder_type).await?;
    Ok(folder)
}

#[tracing::instrument()]
pub async fn list_folders(
    org_id: &str,
    user_id: Option<&str>,
    folder_type: FolderType,
) -> Result<Vec<Folder>, FolderError> {
    let permitted_folders = permitted_folders(org_id, user_id).await?;
    let folders = table::folders::list_folders(org_id, folder_type).await?;
    let filtered = match permitted_folders {
        Some(permitted_folders) => {
            if permitted_folders.contains(&format!("{}:_all_{}", "dfolder", org_id)) {
                folders
            } else {
                folders
                    .into_iter()
                    .filter(|folder_loc| {
                        permitted_folders
                            .contains(&format!("{}:{}", "dfolder", folder_loc.folder_id))
                    })
                    .collect::<Vec<_>>()
            }
        }
        None => folders,
    };
    Ok(filtered)
}

#[tracing::instrument()]
pub async fn get_folder(
    org_id: &str,
    folder_id: &str,
    folder_type: FolderType,
) -> Result<Folder, FolderError> {
    table::folders::get(org_id, folder_id, folder_type)
        .await?
        .ok_or(FolderError::NotFound)
}

#[tracing::instrument()]
pub async fn delete_folder(
    org_id: &str,
    folder_id: &str,
    folder_type: FolderType,
) -> Result<(), FolderError> {
    match folder_type {
        FolderType::Dashboards => {
            let filter = ListDashboardsParams::new(org_id).with_folder_id(folder_id);
            let dashboards = table::dashboards::list(filter).await?;
            if !dashboards.is_empty() {
                return Err(FolderError::DeleteWithDashboards);
            }
        }
    };

    if !table::folders::exists(org_id, folder_id, folder_type).await? {
        return Err(FolderError::NotFound);
    }

    table::folders::delete(org_id, folder_id, folder_type).await?;
    remove_ownership(org_id, "folders", Authz::new(folder_id)).await;
    Ok(())
}

#[cfg(not(feature = "enterprise"))]
async fn permitted_folders(
    _org_id: &str,
    _user_id: Option<&str>,
) -> Result<Option<Vec<String>>, FolderError> {
    Ok(None)
}

#[cfg(feature = "enterprise")]
async fn permitted_folders(
    org_id: &str,
    user_id: Option<&str>,
) -> Result<Option<Vec<String>>, FolderError> {
    let Some(user_id) = user_id else {
        return Err(FolderError::PermittedFoldersMissingUser);
    };
    let stream_list = crate::handler::http::auth::validator::list_objects_for_user(
        org_id, user_id, "GET", "dfolder",
    )
    .await
    .map_err(|err| FolderError::PermittedFoldersValidator(err.to_string()))?;
    Ok(stream_list)
}
