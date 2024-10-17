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

use config::{meta::stream::StreamType, utils::json};

use crate::{
    common::{infra::config::STREAM_PIPELINES, meta::pipelines::PipeLine},
    service::db,
};

pub async fn set(org_id: &str, pipeline: &PipeLine) -> Result<(), anyhow::Error> {
    let key = format!(
        "/pipeline/{org_id}/{}/{}/{}",
        pipeline.stream_type, pipeline.stream_name, pipeline.name
    );
    match db::put(
        &key,
        json::to_vec(pipeline).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error saving pipeline: {}", e);
            return Err(anyhow::anyhow!("Error saving pipeline: {}", e));
        }
    }

    Ok(())
}

pub async fn get(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<PipeLine, anyhow::Error> {
    let val = db::get(&format!(
        "/pipeline/{org_id}/{stream_type}/{stream_name}/{name}"
    ))
    .await?;
    Ok(json::from_slice(&val).unwrap())
}

pub async fn delete(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<(), anyhow::Error> {
    let key = format!("/pipeline/{org_id}/{stream_type}/{stream_name}/{name}");
    match db::delete(&key, false, db::NEED_WATCH, None).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error deleting pipeline: {}", e);
            return Err(anyhow::anyhow!("Error deleting pipeline: {}", e));
        }
    }
    Ok(())
}

pub async fn list(org_id: &str) -> Result<Vec<PipeLine>, anyhow::Error> {
    Ok(db::list(&format!("/pipeline/{org_id}/"))
        .await?
        .values()
        .map(|val| json::from_slice(val).unwrap())
        .collect())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/pipeline/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching pipeline");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_pipelines: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let org_id = &item_key[0..item_key.find('/').unwrap()];
                let item_value: PipeLine = if config::get_config().common.meta_store_external {
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
                let key = format!(
                    "{org_id}/{}/{}",
                    item_value.stream_type, item_value.stream_name
                );
                STREAM_PIPELINES.insert(key.to_owned(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let removal_key = match item_key.rfind('/') {
                    Some(index) => &item_key[0..index],
                    None => item_key,
                };

                STREAM_PIPELINES.remove(removal_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/pipeline/";
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: PipeLine = json::from_slice(&item_value).unwrap();
        let org_id = &item_key[0..item_key.find('/').unwrap()];
        let key = format!("{org_id}/{}/{}", json_val.stream_type, json_val.stream_name);

        STREAM_PIPELINES.insert(key.to_string(), json_val);
    }

    log::info!("Pipelines Cached");
    Ok(())
}

pub async fn reset() -> Result<(), anyhow::Error> {
    let key = "/pipeline/";
    db::delete(key, true, db::NO_NEED_WATCH, None).await?;
    Ok(())
}
