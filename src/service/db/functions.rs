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

use config::utils::json;

use crate::{
    common::{
        infra::config::{QUERY_FUNCTIONS, STREAM_FUNCTIONS},
        meta::functions::{StreamFunctionsList, Transform},
    },
    service::db,
};

pub async fn set(org_id: &str, name: &str, js_func: &Transform) -> Result<(), anyhow::Error> {
    let key = format!("/function/{org_id}/{name}");
    match db::put(
        &key,
        json::to_vec(js_func).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error saving function: {}", e);
            return Err(anyhow::anyhow!("Error saving function: {}", e));
        }
    }

    Ok(())
}

pub async fn get(org_id: &str, name: &str) -> Result<Transform, anyhow::Error> {
    let val = db::get(&format!("/function/{org_id}/{name}")).await?;
    Ok(json::from_slice(&val).unwrap())
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), anyhow::Error> {
    let key = format!("/function/{org_id}/{name}");
    match db::delete(&key, false, db::NEED_WATCH, None).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error deleting function: {}", e);
            return Err(anyhow::anyhow!("Error deleting function: {}", e));
        }
    }
    Ok(())
}

pub async fn list(org_id: &str) -> Result<Vec<Transform>, anyhow::Error> {
    Ok(db::list(&format!("/function/{org_id}/"))
        .await?
        .values()
        .map(|val| json::from_slice(val).unwrap())
        .collect())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/function/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching function");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_functions: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let org_id = &item_key[0..item_key.find('/').unwrap()];
                let item_value: Transform = if config::get_config().common.meta_store_external {
                    match db::get(&ev.key).await {
                        Ok(val) => match json::from_slice(&val) {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        },
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    }
                } else {
                    json::from_slice(&ev.value.unwrap()).unwrap()
                };
                if item_value.streams.is_some() {
                    for stream_fn in item_value.to_stream_transform() {
                        let mut group = STREAM_FUNCTIONS
                            .entry(format!(
                                "{}/{}/{}",
                                org_id, stream_fn.stream_type, stream_fn.stream
                            ))
                            .or_insert_with(|| StreamFunctionsList { list: vec![] });
                        if group.list.contains(&stream_fn) {
                            let stream_name =
                                group.list.iter().position(|x| x.eq(&stream_fn)).unwrap();
                            if stream_fn.is_removed {
                                group.list.remove(stream_name);
                            } else {
                                let _ = std::mem::replace(&mut group.list[stream_name], stream_fn);
                            }
                        } else if !stream_fn.is_removed {
                            group.list.push(stream_fn);
                        }
                    }
                } else {
                    QUERY_FUNCTIONS.insert(item_key.to_owned(), item_value);
                }
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                QUERY_FUNCTIONS.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/function/";
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: Transform = json::from_slice(&item_value).unwrap();
        let org_id = &item_key[0..item_key.find('/').unwrap()];
        if json_val.streams.is_some() {
            for stream_fn in json_val.to_stream_transform() {
                let mut group = STREAM_FUNCTIONS
                    .entry(format!(
                        "{}/{}/{}",
                        org_id, stream_fn.stream_type, stream_fn.stream
                    ))
                    .or_insert_with(|| StreamFunctionsList { list: vec![] });
                if !stream_fn.is_removed {
                    group.list.push(stream_fn);
                }
            }
            let mut func = json_val.clone();
            func.streams = None;
            QUERY_FUNCTIONS.insert(item_key.to_string(), func);
        } else {
            QUERY_FUNCTIONS.insert(item_key.to_string(), json_val);
        }
    }
    log::info!("Functions Cached");
    Ok(())
}

pub async fn reset() -> Result<(), anyhow::Error> {
    let key = "/function/";
    db::delete(key, true, db::NO_NEED_WATCH, None).await?;
    let key = "/transform/";
    db::delete(key, true, db::NO_NEED_WATCH, None).await?;
    Ok(())
}
