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

use std::sync::Arc;

use config::{meta::stream::StreamType, RwHashSet};
use once_cell::sync::Lazy;

use crate::service::db;

static CACHE: Lazy<RwHashSet<String>> = Lazy::new(Default::default);

#[inline]
fn mk_key(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: Option<(&str, &str)>,
) -> String {
    match date_range {
        None => format!("{org_id}/{stream_type}/{stream_name}/all"),
        Some((start, end)) => format!("{org_id}/{stream_type}/{stream_name}/{start},{end}"),
    }
}

// delete data from stream
// if date_range is empty, delete all data
// date_range is a tuple of (start, end), eg: (2023-01-02, 2023-01-03)
pub async fn delete_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: Option<(&str, &str)>,
) -> Result<(), anyhow::Error> {
    let key = mk_key(org_id, stream_type, stream_name, date_range);

    // write in cache
    if CACHE.contains(&key) {
        return Ok(()); // already in cache, just skip
    }

    let db_key = format!("/compact/delete/{key}");
    CACHE.insert(key);

    Ok(db::put(&db_key, "OK".into(), db::NEED_WATCH, None).await?)
}

// set the stream is processing by the node
pub async fn process_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: Option<(&str, &str)>,
    node: &str,
) -> Result<(), anyhow::Error> {
    let key = mk_key(org_id, stream_type, stream_name, date_range);
    let db_key = format!("/compact/delete/{key}");
    Ok(db::put(&db_key, node.to_string().into(), db::NEED_WATCH, None).await?)
}

// get the stream processing information
pub async fn get_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: Option<(&str, &str)>,
) -> String {
    let key = mk_key(org_id, stream_type, stream_name, date_range);
    let db_key = format!("/compact/delete/{key}");
    match db::get(&db_key).await {
        Ok(ret) => String::from_utf8_lossy(&ret).to_string(),
        Err(_) => String::from(""),
    }
}

// check if stream is deleting from cache
pub fn is_deleting_stream(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: Option<(&str, &str)>,
) -> bool {
    CACHE.contains(&mk_key(org_id, stream_type, stream_name, date_range))
}

pub async fn delete_stream_done(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    date_range: Option<(&str, &str)>,
) -> Result<(), anyhow::Error> {
    let key = mk_key(org_id, stream_type, stream_name, date_range);
    db::delete_if_exists(&format!("/compact/delete/{key}"), false, db::NEED_WATCH)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // remove in cache
    CACHE.remove(&key);

    Ok(())
}

pub async fn list() -> Result<Vec<String>, anyhow::Error> {
    let mut items = Vec::new();
    let key = "/compact/delete/";
    let ret = db::list(key).await?;
    for (item_key, _) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        items.push(item_key.to_string());
    }
    Ok(items)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/compact/delete/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching stream deleting");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_stream_deleting: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                CACHE.insert(item_key.to_string());
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                CACHE.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/compact/delete/";
    let ret = db::list(key).await?;
    for (item_key, _) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        CACHE.insert(item_key.to_string());
    }
    Ok(())
}
