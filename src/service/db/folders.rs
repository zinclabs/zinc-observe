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

use anyhow::Ok;
use config::meta::folder::Folder;
use infra::table::folders;

/// Gets the folder with the given `folder_id`.
#[tracing::instrument]
pub(crate) async fn get(org_id: &str, folder_id: &str) -> Result<Option<Folder>, anyhow::Error> {
    let folder = folders::get(org_id, folder_id).await?;
    Ok(folder)
}

/// Checks if the folder with the given `folder_id` exists.
#[tracing::instrument]
pub(crate) async fn exists(org_id: &str, folder_id: &str) -> Result<bool, anyhow::Error> {
    let folder = folders::get(org_id, folder_id).await?;
    Ok(folder.is_some())
}

/// Creates a new folder or updates an existing folder with the given
/// `folder_id`.
#[tracing::instrument(skip(folder))]
pub(crate) async fn put(org_id: &str, folder: Folder) -> Result<Folder, anyhow::Error> {
    let folder = folders::put(org_id, folder).await?;
    Ok(folder)
}

/// Lists all dashboard folders.
#[tracing::instrument]
pub(crate) async fn list_dashboard_folders(org_id: &str) -> Result<Vec<Folder>, anyhow::Error> {
    let folders = folders::list_dashboard_folders(org_id).await?;
    Ok(folders)
}

/// Deletes the folder with the given `folder_id`.
#[tracing::instrument]
pub(crate) async fn delete(org_id: &str, folder_id: &str) -> Result<(), anyhow::Error> {
    let _ = folders::delete(org_id, folder_id).await?;
    Ok(())
}
