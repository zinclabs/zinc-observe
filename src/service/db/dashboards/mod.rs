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
    meta::{
        dashboards::{v1, v2, v3, v4, v5, Dashboard, DashboardVersion, ListDashboardsParams},
        stream::{DistinctField, StreamType},
    },
    utils::json,
};
use hashbrown::HashMap;
use infra::table::{
    dashboards,
    distinct_values::{self, DistinctFieldRecord, OriginType},
};

use crate::service::stream::save_stream_settings;

pub mod reports;

async fn add_distinct_field_entry(
    dashboard_id: &str,
    org: &str,
    stream: &str,
    stype: String,
    field: &str,
) -> Result<(), anyhow::Error> {
    let record = DistinctFieldRecord::new(
        OriginType::Dashboard,
        dashboard_id,
        org,
        stream,
        stype,
        field,
    );
    distinct_values::add(record)
        .await
        .map_err(|e| anyhow::anyhow!("error in adding distinct value record : {e}"))
}

async fn remove_distinct_field_entry(
    dashboard_id: &str,
    org: &str,
    stream: &str,
    stype: String,
    field: &str,
) -> Result<(), anyhow::Error> {
    let record = DistinctFieldRecord::new(
        OriginType::Dashboard,
        dashboard_id,
        org,
        stream,
        stype,
        field,
    );
    distinct_values::remove(record)
        .await
        .map_err(|e| anyhow::anyhow!("error in removing distinct value record : {e}"))
}

async fn update_distinct_variables(
    org_id: &str,
    old_dash: Option<Dashboard>,
    new_dash: &Dashboard,
) -> Result<(), anyhow::Error> {
    let mut old_variables = get_query_variables(old_dash.as_ref());
    let new_variables = get_query_variables(Some(new_dash));

    let dashboard_id = new_dash.dashboard_id().unwrap();

    if !new_variables.is_empty() {
        for ((name, typ), fields) in new_variables.into_iter() {
            let mut stream_settings = infra::schema::get_settings(org_id, &name, typ)
                .await
                .unwrap_or_default();
            // we only store distinct values for logs and traces -
            // if anything else, we can ignore.
            if !matches!(typ, StreamType::Logs | StreamType::Traces) {
                continue;
            }
            // get entry from previous variables corresponding to this stream
            let old_fields = old_variables
                .remove(&(name.to_owned(), typ))
                .unwrap_or_default();
            let mut _new_added = false;

            for f in fields.iter() {
                // we ignore full text search no matter what
                if stream_settings.full_text_search_keys.contains(f) {
                    continue;
                }
                // we add entry for all the fields, because we need mappings for each individual
                // origin-stream-field mapping. The duplicates are handled in add function, so
                // we can call it for each field without issues
                add_distinct_field_entry(dashboard_id, org_id, &name, typ.to_string(), f).await?;
                let _temp = DistinctField {
                    name: f.to_owned(),
                    added_ts: chrono::Utc::now().timestamp_micros(),
                };
                if !stream_settings.distinct_value_fields.contains(&_temp) {
                    stream_settings.distinct_value_fields.push(_temp);
                    _new_added = true;
                }
            }
            // here we check if any of the fields used in previous version are no longer used
            // if so, remove their entry.
            for f in old_fields {
                if !fields.contains(&f) {
                    remove_distinct_field_entry(dashboard_id, org_id, &name, typ.to_string(), &f)
                        .await?;
                }
            }
            if _new_added {
                save_stream_settings(org_id, &name, typ, stream_settings).await?;
            }
        }

        // finally, whatever stream remains in the old variables
        // it has all corresponding fields removed, so remove those as well
        for ((name, typ), fields) in old_variables.into_iter() {
            for f in fields {
                remove_distinct_field_entry(dashboard_id, org_id, &name, typ.to_string(), &f)
                    .await?;
            }
        }
    } else {
        // I guess all the variables were removed from the dashboard.
        // we can batch remove all entries belonging to this dashboard.
        distinct_values::batch_remove(OriginType::Dashboard, dashboard_id).await?
    }
    Ok(())
}

macro_rules! _get_variables {
    ($map:ident, $dash:ident) => {
        if let Some(vars) = &$dash.variables {
            for v in vars.list.iter() {
                if let Some(ref qd) = v.query_data {
                    $map.entry((qd.stream.clone(), qd.stream_type))
                        .or_default()
                        .push(qd.field.clone());
                }
            }
        }
    };
}

pub fn get_query_variables(
    dashboard: Option<&Dashboard>,
) -> HashMap<(String, StreamType), Vec<String>> {
    let mut map: HashMap<(String, StreamType), Vec<String>> = HashMap::new();
    let dashboard = if let Some(d) = dashboard {
        d
    } else {
        return map;
    };
    match dashboard.version {
        1 => {
            let dash = dashboard.v1.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        2 => {
            let dash = dashboard.v2.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        3 => {
            let dash = dashboard.v3.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        4 => {
            let dash = dashboard.v4.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        5 => {
            let dash = dashboard.v5.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        _ => {
            unreachable!("we only have 5 dashboard versions")
        }
    }
    map
}

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
    let mut old_version = None;
    if let Some(existing_dash) = get(org_id, dashboard_id, folder_id).await? {
        let existing_dash_hash = existing_dash.hash.clone();
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
        old_version = Some(existing_dash);
    };

    let d_version: DashboardVersion = json::from_slice(&body)?;
    let mut dash = parse_dashboard(body, d_version.version)?;
    update_distinct_variables(org_id, old_version, &dash).await?;
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
pub(crate) async fn list(params: ListDashboardsParams) -> Result<Vec<Dashboard>, anyhow::Error> {
    let ds = dashboards::list(params).await?;
    Ok(ds)
}

#[tracing::instrument]
pub(crate) async fn delete(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
) -> Result<(), anyhow::Error> {
    distinct_values::batch_remove(OriginType::Dashboard, dashboard_id).await?;
    dashboards::delete(org_id, folder_id, dashboard_id).await?;
    Ok(())
}

#[tracing::instrument]
pub async fn reset() -> Result<(), anyhow::Error> {
    let dashboards = dashboards::list_all().await?;
    let ids: Vec<_> = dashboards
        .iter()
        .map(|(_, d)| d.dashboard_id().unwrap())
        .collect();
    for id in ids {
        distinct_values::batch_remove(OriginType::Dashboard, id).await?;
    }
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
