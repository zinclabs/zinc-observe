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

use config::{cluster::LOCAL_NODE, meta::stream::StreamType, RwAHashMap};
use once_cell::sync::Lazy;

use crate::service::db;

static CACHES: Lazy<RwAHashMap<String, (i64, String)>> = Lazy::new(Default::default);

fn mk_key(org_id: &str, stream_type: StreamType, stream_name: &str) -> String {
    format!("/compact/files/{org_id}/{stream_type}/{stream_name}")
}

pub async fn get_offset_from_cache(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Option<(i64, String)> {
    let key = mk_key(org_id, stream_type, stream_name);
    let r = CACHES.read().await;
    r.get(&key).cloned()
}

pub async fn get_offset(org_id: &str, stream_type: StreamType, stream_name: &str) -> (i64, String) {
    let key = mk_key(org_id, stream_type, stream_name);
    let r = CACHES.read().await;
    if let Some(val) = r.get(&key) {
        return val.clone();
    }
    drop(r);

    let value = match db::get(&key).await {
        Ok(ret) => String::from_utf8_lossy(&ret).to_string(),
        Err(_) => String::from("0"),
    };
    let (offset, node) = if value.contains(';') {
        let mut parts = value.split(';');
        let offset: i64 = parts.next().unwrap().parse().unwrap();
        let node = parts.next().unwrap().to_string();
        (offset, node)
    } else {
        (value.parse().unwrap(), String::from(""))
    };
    // only cache the value if it's empty or it's from this node
    if node.is_empty() || LOCAL_NODE.uuid.eq(&node) {
        let mut w = CACHES.write().await;
        w.insert(key.clone(), (offset, node.clone()));
        drop(w);
    }
    (offset, node)
}

pub async fn set_offset(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    offset: i64,
    node: Option<&str>,
) -> Result<(), anyhow::Error> {
    let key = mk_key(org_id, stream_type, stream_name);
    let Some(node) = node else {
        // release this key from this node
        db::put(&key, offset.to_string().into(), db::NO_NEED_WATCH, None).await?;
        let mut w = CACHES.write().await;
        w.remove(&key);
        drop(w);
        return Ok(());
    };

    // set this key to this node
    let mut w = CACHES.write().await;
    w.insert(key, (offset, node.to_string()));
    drop(w);
    Ok(())
}

pub async fn del_offset(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    let key = mk_key(org_id, stream_type, stream_name);
    let mut w = CACHES.write().await;
    w.remove(&key);
    drop(w);
    db::delete_if_exists(&key, false, db::NO_NEED_WATCH)
        .await
        .map_err(Into::into)
}

pub async fn list_offset() -> Result<Vec<(String, i64)>, anyhow::Error> {
    let mut items = Vec::new();
    let key = "/compact/files/";
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let value = String::from_utf8_lossy(&item_value).to_string();
        let offset = if value.contains(';') {
            let mut parts = value.split(';');
            parts.next().unwrap().parse().unwrap()
        } else {
            value.parse().unwrap()
        };
        items.push((item_key.to_string(), offset));
    }
    Ok(items)
}

pub async fn sync_cache_to_db() -> Result<(), anyhow::Error> {
    let r = CACHES.read().await;
    for (key, (offset, node)) in r.iter() {
        if *offset > 0 {
            let val = if !node.is_empty() {
                format!("{};{}", offset, node)
            } else {
                offset.to_string()
            };
            db::put(key, val.into(), db::NO_NEED_WATCH, None).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compact_files() {
        const OFFSET: i64 = 100;
        set_offset(
            "default",
            "logs".into(),
            "compact_file",
            OFFSET,
            Some("LOCAL"),
        )
        .await
        .unwrap();
        sync_cache_to_db().await.unwrap();
        assert_eq!(
            get_offset("default", "logs".into(), "compact_file").await,
            (OFFSET, "LOCAL".to_string())
        );
        assert!(!list_offset().await.unwrap().is_empty());
    }
}
