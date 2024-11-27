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

use actix_web::web;
use config::{
    meta::dashboards::{v1, v2, v3, v4, v5, Dashboard, DashboardVersion},
    utils::json,
};
use infra::table::dashboards;

pub mod reports;

#[tracing::instrument]
pub(crate) async fn get(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
) -> Result<Option<Dashboard>, anyhow::Error> {
    let dash = dashboards::get(org_id, folder_id, dashboard_id).await?;
    Ok(dash)
}

#[tracing::instrument(skip(body))]
pub(crate) async fn put(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
    body: web::Bytes,
    hash: Option<&str>,
) -> Result<Dashboard, anyhow::Error> {
    if let Some(existing_dash) = get(org_id, dashboard_id, folder_id).await? {
        let existing_dash_hash = existing_dash.hash;

        let Some(Ok(hash_val)) = hash.map(|hash_str| hash_str.parse::<u64>()) else {
            return Err(anyhow::anyhow!(
                "Request to update existing dashboard with missing or invalid hash value. BUG"
            ));
        };
        if hash_val.to_string() != existing_dash_hash {
            return Err(anyhow::anyhow!(
                "Conflict: Failed to save due to concurrent changes. Please refresh the page after backing up your work to avoid losing changes."
            ));
        }
    };

    let d_version: DashboardVersion = json::from_slice(&body)?;
    let mut dash = parse_dashboard(body, d_version.version)?;

    let title = dash
        .title()
        .map(|t| t.trim().to_string())
        .and_then(|t| if t.is_empty() { None } else { Some(t) })
        .ok_or_else(|| anyhow::anyhow!("Dashboard should have title"))?;
    dash.set_title(title);

    dash.set_dashboard_id(dashboard_id.to_owned());
    let dash = dashboards::put(org_id, folder_id, dash).await?;
    Ok(dash)
}

#[tracing::instrument]
pub(crate) async fn list(org_id: &str, folder_id: &str) -> Result<Vec<Dashboard>, anyhow::Error> {
    let ds = dashboards::list(org_id, folder_id).await?;
    Ok(ds)
}

#[tracing::instrument]
pub(crate) async fn delete(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
) -> Result<(), anyhow::Error> {
    dashboards::delete(org_id, folder_id, dashboard_id).await?;
    Ok(())
}

#[tracing::instrument]
pub async fn reset() -> Result<(), anyhow::Error> {
    dashboards::delete_all().await?;
    Ok(())
}

/// Parses the bytes into a dashboard with the given version.
pub(crate) fn parse_dashboard(
    bytes: web::Bytes,
    version: i32,
) -> Result<Dashboard, serde_json::Error> {
    let dash = match version {
        1 => {
            let inner: v1::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        2 => {
            let inner: v2::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        3 => {
            let inner: v3::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        4 => {
            let inner: v4::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
        _ => {
            let inner: v5::Dashboard = json::from_slice(&bytes)?;
            inner.into()
        }
    };
    Ok(dash)
}
