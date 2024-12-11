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
        dashboards::{Dashboard, ListDashboardsParams},
        folder::{Folder, DEFAULT_FOLDER},
    },
};
use infra::table;

use super::folders;
use crate::common::{
    meta::authz::Authz,
    utils::auth::{remove_ownership, set_ownership},
};
pub mod reports;

/// An error that occurs interacting with dashboards.
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    /// An error that occurs while interacting with the database through the
    /// [infra] crate.
    #[error("InfraError# {0}")]
    InfraError(#[from] infra::errors::Error),

    /// Error that occurs when trying to access a dashboard that cannot be
    /// found.
    #[error("dashboard not found")]
    DashboardNotFound,

    /// Error that occurs when trying to update a dashboard using a missing hash
    /// value or a hash value that cannot be parsed as an [i64].
    #[error("tried to update dashboard using hash that cannot be parsed")]
    UpdateMissingHash,

    /// Error that occurs when trying to update a dashboard using a stale hash.
    /// This occurs when the two clients attempt to update the dashboard
    /// concurrently.
    #[error("tried to update dashboard using conflicting hash")]
    UpdateConflictingHash,

    /// Error that occurs when trying to create an update a dashboard but not
    /// title is provided.
    #[error("dashboard cannot have empty title")]
    PutMissingTitle,

    /// Error that occurs when trying to move a dashboard but either the source
    /// or destination folder is not specified.
    #[error("missing source or destination folder for dashboard move")]
    MoveMissingFolderParam,

    /// Error that occurs when trying to move a dashboard to a destination
    /// folder that cannot be found.
    #[error("error moving dashboard to folder that cannot be found")]
    MoveDestinationFolderNotFound,

    /// Error that occurs when trying to create a dashboard in a folder that
    /// cannot be found.
    #[error("error creating dashboard in folder that cannot be found")]
    CreateFolderNotFound,

    /// Error that occurs when trying to create a destination folder for a new
    /// dashboard.
    #[error("error creating default folder")]
    CreateDefaultFolder,
}

#[tracing::instrument(skip(dashboard))]
pub async fn create_dashboard(
    org_id: &str,
    folder_id: &str,
    dashboard: Dashboard,
) -> Result<Dashboard, DashboardError> {
    // NOTE: Overwrite whatever `dashboard_id` the client has sent us
    // If folder is default folder & doesn't exist then create it

    if table::folders::exists(org_id, folder_id).await? {
        let dashboard_id = ider::generate();
        let saved = put(org_id, &dashboard_id, folder_id, dashboard, None).await?;
        set_ownership(
            org_id,
            "dashboards",
            Authz {
                obj_id: dashboard_id,
                parent_type: "folders".to_owned(),
                parent: folder_id.to_owned(),
            },
        )
        .await;
        Ok(saved)
    } else if folder_id == DEFAULT_FOLDER {
        let folder = Folder {
            folder_id: DEFAULT_FOLDER.to_string(),
            name: DEFAULT_FOLDER.to_string(),
            description: DEFAULT_FOLDER.to_string(),
        };
        folders::save_folder(org_id, folder, true)
            .await
            .map_err(|_| DashboardError::CreateDefaultFolder)?;
        let dashboard_id = ider::generate();
        let saved = put(org_id, &dashboard_id, folder_id, dashboard, None).await?;
        set_ownership(
            org_id,
            "dashboards",
            Authz {
                obj_id: dashboard_id,
                parent_type: "folders".to_owned(),
                parent: folder_id.to_owned(),
            },
        )
        .await;
        Ok(saved)
    } else {
        Err(DashboardError::CreateFolderNotFound)
    }
}

#[tracing::instrument(skip(dashboard))]
pub async fn update_dashboard(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
    dashboard: Dashboard,
    hash: Option<&str>,
) -> Result<Dashboard, DashboardError> {
    put(org_id, dashboard_id, folder_id, dashboard, hash).await
}

#[tracing::instrument]
pub async fn list_dashboards(
    params: ListDashboardsParams,
) -> Result<Vec<(Folder, Dashboard)>, DashboardError> {
    let dashboards = table::dashboards::list(params).await?;
    Ok(dashboards)
}

#[tracing::instrument]
pub async fn get_dashboard(org_id: &str, dashboard_id: &str) -> Result<Dashboard, DashboardError> {
    table::dashboards::get_by_id(org_id, dashboard_id)
        .await?
        .ok_or(DashboardError::DashboardNotFound)
        .map(|(_f, d)| d)
}

#[tracing::instrument]
pub async fn delete_dashboard(org_id: &str, dashboard_id: &str) -> Result<(), DashboardError> {
    let Some((folder, _dashboard)) = table::dashboards::get_by_id(org_id, dashboard_id).await?
    else {
        return Err(DashboardError::DashboardNotFound);
    };
    table::dashboards::delete_from_folder(org_id, &folder.folder_id, dashboard_id).await?;
    remove_ownership(
        org_id,
        "dashboards",
        Authz {
            obj_id: dashboard_id.to_owned(),
            parent_type: "folders".to_owned(),
            parent: folder.folder_id,
        },
    )
    .await;
    Ok(())
}

#[tracing::instrument]
pub async fn move_dashboard(
    org_id: &str,
    dashboard_id: &str,
    from_folder: &str,
    to_folder: &str,
) -> Result<(), DashboardError> {
    if from_folder.is_empty() || to_folder.is_empty() {
        return Err(DashboardError::MoveMissingFolderParam);
    };

    let Some(dashboard) =
        table::dashboards::get_from_folder(org_id, from_folder, dashboard_id).await?
    else {
        return Err(DashboardError::DashboardNotFound);
    };

    // make sure the destination folder exists
    if !table::folders::exists(org_id, to_folder).await? {
        return Err(DashboardError::MoveDestinationFolderNotFound);
    };

    // add the dashboard to the destination folder
    put(org_id, dashboard_id, to_folder, dashboard, None).await?;

    // delete the dashboard from the source folder
    let _ = table::dashboards::delete_from_folder(org_id, from_folder, dashboard_id).await;

    Ok(())
}

#[tracing::instrument(skip(dashboard))]
async fn put(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
    mut dashboard: Dashboard,
    hash: Option<&str>,
) -> Result<Dashboard, DashboardError> {
    if let Some(existing_dash) =
        table::dashboards::get_from_folder(org_id, folder_id, dashboard_id).await?
    {
        let existing_dash_hash = existing_dash.hash;

        let Some(Ok(hash_val)) = hash.map(|hash_str| hash_str.parse::<u64>()) else {
            return Err(DashboardError::UpdateMissingHash);
        };
        if hash_val.to_string() != existing_dash_hash {
            return Err(DashboardError::UpdateConflictingHash);
        }
    };

    let title = dashboard
        .title()
        .map(|t| t.trim().to_string())
        .and_then(|t| if t.is_empty() { None } else { Some(t) })
        .ok_or_else(|| DashboardError::PutMissingTitle)?;
    dashboard.set_title(title);

    dashboard.set_dashboard_id(dashboard_id.to_owned());
    let dash = table::dashboards::put(org_id, folder_id, dashboard).await?;
    Ok(dash)
}

/// Internal helper function find dashboard and its folder by id.
///
/// Used by self_reporting to enrich dashboard SearchEventContext
pub(crate) async fn get_folder_and_dashboard(
    org_id: &str,
    dashboard_id: &str,
) -> Result<(Folder, Dashboard), DashboardError> {
    table::dashboards::get_by_id(org_id, dashboard_id)
        .await?
        .ok_or(DashboardError::DashboardNotFound)
}
