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

#[cfg(feature = "enterprise")]
use config::utils::json;
use infra::errors;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config;

#[cfg(feature = "enterprise")]
use crate::table::distinct_values::BatchDeleteMessage;
use crate::table::distinct_values::{DistinctFieldRecord, OriginType};

pub async fn add(record: DistinctFieldRecord) -> Result<(), errors::Error> {
    #[cfg(feature = "enterprise")]
    emit_put_event(&record).await?;
    crate::table::distinct_values::add(record).await
}

pub async fn remove(record: DistinctFieldRecord) -> Result<(), errors::Error> {
    #[cfg(feature = "enterprise")]
    emit_delete_event(&record).await?;
    crate::table::distinct_values::remove(record).await
}

pub async fn batch_remove(origin: OriginType, origin_id: &str) -> Result<(), errors::Error> {
    #[cfg(feature = "enterprise")]
    emit_batch_delete_event(&origin, origin_id).await?;
    crate::table::distinct_values::batch_remove(origin, origin_id).await
}

/// Sends event to super cluster queue for a new distinct values entry.
#[cfg(feature = "enterprise")]
pub async fn emit_put_event(record: &DistinctFieldRecord) -> Result<(), errors::Error> {
    if get_config().super_cluster.enabled {
        let value = json::to_vec(&record)?.into();
        let key = format!("/distinct_values/{}", record.org_name);
        o2_enterprise::enterprise::super_cluster::queue::distinct_values_put(&key, value, true)
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
    }
    Ok(())
}

/// Sends event to super cluster queue for a deleted distinct values entry.
#[cfg(feature = "enterprise")]
pub async fn emit_delete_event(record: &DistinctFieldRecord) -> Result<(), infra::errors::Error> {
    if get_config().super_cluster.enabled {
        let value = json::to_vec(&record)?.into();
        let key = format!("/distinct_values/{}", record.org_name);
        o2_enterprise::enterprise::super_cluster::queue::distinct_values_delete(
            &key, value, false, true,
        )
        .await
        .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
    }
    Ok(())
}

/// Sends event to super cluster queue for a deleted distinct values batch.
#[cfg(feature = "enterprise")]
pub async fn emit_batch_delete_event(
    origin_type: &OriginType,
    id: &str,
) -> Result<(), infra::errors::Error> {
    if get_config().super_cluster.enabled {
        let value = json::to_vec(&BatchDeleteMessage {
            origin_type: *origin_type,
            id: id.to_owned(),
        })?
        .into();
        let key = format!("/distinct_values/{}", id);
        o2_enterprise::enterprise::super_cluster::queue::distinct_values_delete(
            &key, value, true, true,
        )
        .await
        .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
    }
    Ok(())
}
