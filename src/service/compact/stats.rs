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

use config::cluster::LOCAL_NODE;
use infra::{dist_lock, file_list as infra_file_list};

use crate::{common::infra::cluster::get_node_by_uuid, service::db};

pub async fn update_stats_from_file_list() -> Result<Option<(i64, i64)>, anyhow::Error> {
    // get last offset
    let (mut offset, node) = db::compact::stats::get_offset().await;
    if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        return Ok(None);
    }

    // before starting, set current node to lock the job
    if config::get_config().common.meta_store_external
        && (node.is_empty() || LOCAL_NODE.uuid.ne(&node))
    {
        offset = match update_stats_lock_node().await {
            Ok(Some(offset)) => offset,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        }
    }

    // get latest offset
    let latest_pk = infra_file_list::get_max_pk_value().await?;
    let pk_value = if offset == 0 && latest_pk == 0 {
        None
    } else {
        Some((offset, latest_pk))
    };

    // get stats from file_list
    let orgs = db::schema::list_organizations_from_cache().await;
    for org_id in orgs {
        let stream_stats = infra_file_list::stats(&org_id, None, None, pk_value).await?;
        if !stream_stats.is_empty() {
            infra_file_list::set_stream_stats(&org_id, &stream_stats).await?;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // update offset
    db::compact::stats::set_offset(latest_pk, Some(&LOCAL_NODE.uuid.clone())).await?;

    Ok(pk_value)
}

async fn update_stats_lock_node() -> Result<Option<i64>, anyhow::Error> {
    let lock_key = "/compact/stream_stats/offset".to_string();
    let locker = dist_lock::lock(&lock_key, 0, None).await?;
    // check the working node for the organization again, maybe other node locked it
    // first
    let (offset, node) = db::compact::stats::get_offset().await;
    if !node.is_empty() && LOCAL_NODE.uuid.ne(&node) && get_node_by_uuid(&node).await.is_some() {
        dist_lock::unlock(&locker).await?;
        return Ok(None);
    }

    // bind the job to this node
    let ret = db::compact::stats::set_offset(offset, Some(&LOCAL_NODE.uuid.clone())).await;
    // already bind to this node, we can unlock now
    dist_lock::unlock(&locker).await?;
    if let Err(e) = ret {
        Err(e)
    } else {
        Ok(Some(offset))
    }
}
