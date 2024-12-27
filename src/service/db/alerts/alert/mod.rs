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

//! This module provides methods for accessing alerts in the database.
//!
//! By default, the methods in this module will use the new `alerts` table to
//! access and store alerts. However if the `USE_META_ALERTS` environment
//! variable is set to `"true"` then the old `meta` tale will be used to access
//! and set alerts instead. This functionality is provided temporarily to allow
//! users to revert to using the `meta` table if they encounter problems. Once
//! it has been validated that the new `alerts` table functions correctly then
//! alerts will be removed from the `meta` table and the ability to use the
//! `meta` table to access and store alerts will be removed.

mod new;
mod old;

use config::meta::{
    alerts::alert::{Alert, ListAlertsParams},
    folder::Folder,
    stream::StreamType,
};
use infra::db::{connect_to_orm, ORM_CLIENT};
use sea_orm::{ConnectionTrait, TransactionTrait};
use svix_ksuid::Ksuid;

pub async fn get_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<Option<Alert>, infra::errors::Error> {
    new::get_by_id(conn, org_id, alert_id).await
}

pub async fn get_by_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<Option<Alert>, infra::errors::Error> {
    if should_use_meta_alerts() {
        old::get_by_name(org_id, stream_type, stream_name, name).await
    } else {
        new::get_by_name(org_id, stream_type, stream_name, name).await
    }
}

pub async fn set(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    alert: Alert,
    create: bool,
) -> Result<(), infra::errors::Error> {
    if should_use_meta_alerts() {
        old::set(org_id, stream_type, stream_name, &alert, create).await
    } else {
        new::set(org_id, stream_type, stream_name, alert, create).await
    }
}

pub async fn set_without_updating_trigger(org_id: &str, alert: Alert) -> Result<(), anyhow::Error> {
    if should_use_meta_alerts() {
        old::set_without_updating_trigger(org_id, alert.stream_type, &alert.stream_name, &alert)
            .await?;
        Ok(())
    } else {
        new::set_without_updating_trigger(org_id, alert).await
    }
}

pub async fn create<C: TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    alert: Alert,
) -> Result<Alert, infra::errors::Error> {
    new::create(conn, org_id, folder_id, alert).await
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: Option<&str>,
    alert: Alert,
) -> Result<Alert, infra::errors::Error> {
    new::update(conn, org_id, folder_id, alert).await
}

pub async fn delete_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<(), infra::errors::Error> {
    new::delete_by_id(conn, org_id, alert_id).await
}

pub async fn delete_by_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<(), infra::errors::Error> {
    if should_use_meta_alerts() {
        old::delete_by_name(org_id, stream_type, stream_name, name).await
    } else {
        new::delete_by_name(org_id, stream_type, stream_name, name).await
    }
}

pub async fn list(
    org_id: &str,
    stream_type: Option<StreamType>,
    stream_name: Option<&str>,
) -> Result<Vec<Alert>, infra::errors::Error> {
    if should_use_meta_alerts() {
        old::list(org_id, stream_type, stream_name).await
    } else {
        let params = ListAlertsParams::new(org_id).in_folder("default");
        let params = if let Some(stream_name) = stream_name {
            params.for_stream(stream_type.unwrap_or_default(), Some(stream_name))
        } else {
            params
        };

        let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
        let alerts = new::list(client, params)
            .await?
            .into_iter()
            .map(|(_f, a)| a)
            .collect();
        Ok(alerts)
    }
}

pub async fn list_with_folders<C: ConnectionTrait>(
    conn: &C,
    params: ListAlertsParams,
) -> Result<Vec<(Folder, Alert)>, infra::errors::Error> {
    new::list(conn, params).await
}

pub async fn watch() -> Result<(), anyhow::Error> {
    if should_use_meta_alerts() {
        old::watch().await
    } else {
        new::watch().await
    }
}

pub async fn cache() -> Result<(), anyhow::Error> {
    if should_use_meta_alerts() {
        old::cache().await
    } else {
        new::cache().await
    }
}

pub async fn reset() -> Result<(), anyhow::Error> {
    if should_use_meta_alerts() {
        old::reset().await
    } else {
        new::reset().await
    }
}

/// Returns `true` if the `USE_META_ALERTS` environment variable is set to
/// `"true"`, indicating that alerts should be accessed from and stored in the
/// old `meta` table. Returns `false` otherwise, indicating that alerts should
/// be stored from and stored in the new `alerts` table.
fn should_use_meta_alerts() -> bool {
    std::env::var("USE_META_ALERTS")
        .ok()
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false)
}
