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
use config::utils::{hash::Sum64, json};

use crate::{
    common::meta::dashboards::{v1, v2, v3, v4, v5, Dashboard, DashboardVersion},
    service::db,
};

pub mod reports;

#[tracing::instrument]
pub(crate) async fn get(
    org_id: &str,
    dashboard_id: &str,
    folder: &str,
) -> Result<Dashboard, anyhow::Error> {
    let key = format!("/dashboard/{org_id}/{folder}/{dashboard_id}");
    let bytes = db::get(&key).await?;
    let dash_str = std::str::from_utf8(&bytes)?;
    let hash = config::utils::hash::gxhash::new()
        .sum64(dash_str)
        .to_string();
    let d_version: DashboardVersion = json::from_slice(&bytes)?;
    if d_version.version == 1 {
        let dash: v1::Dashboard = json::from_slice(&bytes)?;
        Ok(Dashboard {
            v1: Some(dash),
            version: 1,
            hash,
            ..Default::default()
        })
    } else if d_version.version == 2 {
        let dash: v2::Dashboard = json::from_slice(&bytes)?;
        Ok(Dashboard {
            v2: Some(dash),
            version: 2,
            hash,
            ..Default::default()
        })
    } else if d_version.version == 3 {
        let dash: v3::Dashboard = json::from_slice(&bytes)?;
        Ok(Dashboard {
            v3: Some(dash),
            version: 3,
            hash,
            ..Default::default()
        })
    } else if d_version.version == 4 {
        let dash: v4::Dashboard = json::from_slice(&bytes)?;
        Ok(Dashboard {
            v4: Some(dash),
            version: 4,
            hash,
            ..Default::default()
        })
    } else {
        let dash: v5::Dashboard = json::from_slice(&bytes)?;
        Ok(Dashboard {
            v5: Some(dash),
            version: 5,
            hash,
            ..Default::default()
        })
    }
}

#[tracing::instrument(skip(body))]
pub(crate) async fn put(
    org_id: &str,
    dashboard_id: &str,
    folder: &str,
    body: web::Bytes,
    hash: Option<&str>,
) -> Result<Dashboard, anyhow::Error> {
    let key = format!("/dashboard/{org_id}/{folder}/{}", dashboard_id);
    if let Ok(existing_dash_bytes) = db::get(&key).await {
        let existing_dash_str = std::str::from_utf8(&existing_dash_bytes)?;
        let existing_dash_hash = config::utils::hash::gxhash::new().sum64(existing_dash_str);
        let Some(Ok(hash_val)) = hash.map(|hash_str| hash_str.parse::<u64>()) else {
            return Err(anyhow::anyhow!(
                "Request to update existing dashboard with missing or invalid hash value. BUG"
            ));
        };
        if hash_val != existing_dash_hash {
            return Err(anyhow::anyhow!(
                "Conflict: Failed to save due to concurrent changes. Please refresh the page after backing up your work to avoid losing changes."
            ));
        }
    };
    let d_version: DashboardVersion = json::from_slice(&body)?;
    if d_version.version == 1 {
        let mut dash: v1::Dashboard = json::from_slice(&body)?;
        dash.title = dash.title.trim().to_string();
        if dash.title.is_empty() {
            return Err(anyhow::anyhow!("Dashboard should have title"));
        };
        dash.dashboard_id = dashboard_id.to_string();
        let value: bytes::Bytes = json::to_vec(&dash)?.into();
        let dash_str = std::str::from_utf8(&value)?;
        let hash = config::utils::hash::gxhash::new()
            .sum64(dash_str)
            .to_string();

        match db::put(&key, value, db::NO_NEED_WATCH, None).await {
            Ok(_) => Ok(Dashboard {
                v1: Some(dash),
                version: 1,
                hash,
                ..Default::default()
            }),
            Err(_) => Err(anyhow::anyhow!("Failed to save Dashboard")),
        }
    } else if d_version.version == 2 {
        let mut dash: v2::Dashboard = json::from_slice(&body)?;
        dash.title = dash.title.trim().to_string();
        if dash.title.is_empty() {
            return Err(anyhow::anyhow!("Dashboard should have title"));
        };
        dash.dashboard_id = dashboard_id.to_string();
        let value: bytes::Bytes = json::to_vec(&dash)?.into();
        let dash_str = std::str::from_utf8(&value)?;
        let hash = config::utils::hash::gxhash::new()
            .sum64(dash_str)
            .to_string();
        match db::put(&key, value, db::NO_NEED_WATCH, None).await {
            Ok(_) => Ok(Dashboard {
                v2: Some(dash),
                version: 2,
                hash,
                ..Default::default()
            }),
            Err(_) => Err(anyhow::anyhow!("Failed to save Dashboard")),
        }
    } else if d_version.version == 3 {
        let mut dash: v3::Dashboard = json::from_slice(&body)?;
        dash.title = dash.title.trim().to_string();
        if dash.title.is_empty() {
            return Err(anyhow::anyhow!("Dashboard should have title"));
        };
        dash.dashboard_id = dashboard_id.to_string();
        let value: bytes::Bytes = json::to_vec(&dash)?.into();
        let dash_str = std::str::from_utf8(&value)?;
        let hash = config::utils::hash::gxhash::new()
            .sum64(dash_str)
            .to_string();
        match db::put(&key, value, db::NO_NEED_WATCH, None).await {
            Ok(_) => Ok(Dashboard {
                v3: Some(dash),
                version: 3,
                hash,
                ..Default::default()
            }),
            Err(_) => Err(anyhow::anyhow!("Failed to save Dashboard")),
        }
    } else if d_version.version == 4 {
        let mut dash: v4::Dashboard = json::from_slice(&body)?;
        dash.title = dash.title.trim().to_string();
        if dash.title.is_empty() {
            return Err(anyhow::anyhow!("Dashboard should have title"));
        };
        dash.dashboard_id = dashboard_id.to_string();
        let value: bytes::Bytes = json::to_vec(&dash)?.into();
        let dash_str = std::str::from_utf8(&value)?;
        let hash = config::utils::hash::gxhash::new()
            .sum64(dash_str)
            .to_string();
        match db::put(&key, value, db::NO_NEED_WATCH, None).await {
            Ok(_) => Ok(Dashboard {
                v4: Some(dash),
                version: 4,
                hash,
                ..Default::default()
            }),
            Err(_) => Err(anyhow::anyhow!("Failed to save Dashboard")),
        }
    } else {
        let mut dash: v5::Dashboard = json::from_slice(&body)?;
        dash.title = dash.title.trim().to_string();
        if dash.title.is_empty() {
            return Err(anyhow::anyhow!("Dashboard should have title"));
        };
        dash.dashboard_id = dashboard_id.to_string();
        let value: bytes::Bytes = json::to_vec(&dash)?.into();
        let dash_str = std::str::from_utf8(&value)?;
        let hash = config::utils::hash::gxhash::new()
            .sum64(dash_str)
            .to_string();
        match db::put(&key, value, db::NO_NEED_WATCH, None).await {
            Ok(_) => Ok(Dashboard {
                v5: Some(dash),
                version: 5,
                hash,
                ..Default::default()
            }),
            Err(_) => Err(anyhow::anyhow!("Failed to save Dashboard")),
        }
    }
}

#[tracing::instrument]
pub(crate) async fn list(org_id: &str, folder: &str) -> Result<Vec<Dashboard>, anyhow::Error> {
    let db_key = format!("/dashboard/{org_id}/{folder}/");
    db::list(&db_key)
        .await?
        .into_values()
        .map(|val| {
            let dash_str = std::str::from_utf8(&val)?;
            let hash = config::utils::hash::gxhash::new()
                .sum64(dash_str)
                .to_string();
            let d_version: DashboardVersion = json::from_slice(&val).unwrap();
            if d_version.version == 1 {
                let dash: v1::Dashboard = json::from_slice(&val).unwrap();
                Ok(Dashboard {
                    v1: Some(dash),
                    version: 1,
                    hash,
                    ..Default::default()
                })
            } else if d_version.version == 2 {
                let dash: v2::Dashboard = json::from_slice(&val).unwrap();
                Ok(Dashboard {
                    v2: Some(dash),
                    version: 2,
                    hash,
                    ..Default::default()
                })
            } else if d_version.version == 3 {
                let dash: v3::Dashboard = json::from_slice(&val).unwrap();
                Ok(Dashboard {
                    v3: Some(dash),
                    version: 3,
                    hash,
                    ..Default::default()
                })
            } else if d_version.version == 4 {
                let dash: v4::Dashboard = json::from_slice(&val).unwrap();
                Ok(Dashboard {
                    v4: Some(dash),
                    version: 4,
                    hash,
                    ..Default::default()
                })
            } else {
                let dash: v5::Dashboard = json::from_slice(&val).unwrap();
                Ok(Dashboard {
                    v5: Some(dash),
                    version: 5,
                    hash,
                    ..Default::default()
                })
            }
        })
        .collect()
}

#[tracing::instrument]
pub(crate) async fn delete(
    org_id: &str,
    dashboard_id: &str,
    folder: &str,
) -> Result<(), anyhow::Error> {
    let key = format!("/dashboard/{org_id}/{folder}/{dashboard_id}");
    Ok(db::delete(&key, false, db::NO_NEED_WATCH, None).await?)
}

#[tracing::instrument]
pub async fn reset() -> Result<(), anyhow::Error> {
    let key = "/dashboard/";
    Ok(db::delete(key, true, db::NO_NEED_WATCH, None).await?)
}
