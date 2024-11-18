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

use config::{meta::folder::Folder, utils::json};

use crate::service::db;

#[tracing::instrument]
pub(crate) async fn get(org_id: &str, folder_id: &str) -> Result<Folder, anyhow::Error> {
    let val = db::get(&format!("/folders/{org_id}/{folder_id}")).await?;
    Ok(json::from_slice(&val).unwrap())
}

#[tracing::instrument(skip(folder))]
pub(crate) async fn put(org_id: &str, folder: Folder) -> Result<Folder, anyhow::Error> {
    let key = format!("/folders/{org_id}/{}", folder.folder_id);
    match db::put(&key, json::to_vec(&folder)?.into(), db::NO_NEED_WATCH, None).await {
        Ok(_) => Ok(folder),
        Err(_) => Err(anyhow::anyhow!("Failed to save folder")),
    }
}

#[tracing::instrument]
pub(crate) async fn list(org_id: &str) -> Result<Vec<Folder>, anyhow::Error> {
    let db_key = format!("/folders/{org_id}/");
    db::list(&db_key)
        .await?
        .into_values()
        .map(|val| json::from_slice(&val).map_err(|e| anyhow::anyhow!(e)))
        .collect()
}

#[tracing::instrument]
pub(crate) async fn delete(org_id: &str, folder_id: &str) -> Result<(), anyhow::Error> {
    let key = format!("/folders/{org_id}/{folder_id}");
    Ok(db::delete(&key, false, db::NO_NEED_WATCH, None).await?)
}
