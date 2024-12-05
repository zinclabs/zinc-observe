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

use config::{meta::alerts::templates::Template, utils::json};
use itertools::Itertools;

use super::json as db_json;
use crate::{
    common::{infra::config::ALERTS_TEMPLATES, meta::organization::DEFAULT_ORG},
    service::db,
};

pub async fn get(org_id: &str, name: &str) -> Result<Template, anyhow::Error> {
    let map_key = format!("{org_id}/{name}");
    if let Some(v) = ALERTS_TEMPLATES.get(&map_key) {
        return Ok(v.value().clone());
    }
    let default_org_key = format!("{DEFAULT_ORG}/{name}");
    if let Some(v) = ALERTS_TEMPLATES.get(&default_org_key) {
        return Ok(v.value().clone());
    }

    let key = format!("/templates/{org_id}/{name}");
    if let Ok(val) = db::get(&key).await {
        let db_model: db_json::Template = json::from_slice(&val).unwrap();
        return Ok(db_model.into());
    }
    let key = format!("/templates/{DEFAULT_ORG}/{name}");
    let val = &db::get(&key).await?;
    let db_model: db_json::Template = json::from_slice(val).unwrap();
    Ok(db_model.into())
}

pub async fn set(org_id: &str, mut template: Template) -> Result<(), anyhow::Error> {
    template.is_default = Some(org_id == DEFAULT_ORG);
    let key = format!("/templates/{org_id}/{}", template.name);
    let db_model: db_json::Template = template.into();
    Ok(db::put(
        &key,
        json::to_vec(&db_model).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await?)
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), anyhow::Error> {
    let key = format!("/templates/{org_id}/{name}");
    Ok(db::delete(&key, false, db::NEED_WATCH, None).await?)
}

pub async fn list(org_id: &str) -> Result<Vec<Template>, anyhow::Error> {
    let cache = ALERTS_TEMPLATES.clone();
    if !cache.is_empty() {
        return Ok(cache
            .iter()
            .filter_map(|template| {
                let k = template.key();
                (k.starts_with(&format!("{org_id}/")) || k.starts_with(&format!("{DEFAULT_ORG}/")))
                    .then(|| template.value().clone())
            })
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .collect());
    }

    let key = format!("/templates/{org_id}/");
    let ret = db::list_values(key.as_str()).await?;
    let mut items = Vec::new();
    for item_value in ret {
        let json_val: db_json::Template = json::from_slice(&item_value).unwrap();
        let template: Template = json_val.into();
        items.push(template);
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/templates/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching alert templates");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alert_templates: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value: Template = if config::get_config().common.meta_store_external {
                    match db::get(&ev.key).await {
                        Ok(val) => match json::from_slice::<db_json::Template>(&val) {
                            Ok(val) => val.into(),
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
                    // This assumes that templates stored in events can be
                    // deserialized using the same logic as templates stored in
                    // events. If that ever changes we would need to use
                    // different deserialization logic.
                    let db_model: db_json::Template = json::from_slice(&ev.value.unwrap()).unwrap();
                    db_model.into()
                };
                ALERTS_TEMPLATES.insert(item_key.to_owned(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                ALERTS_TEMPLATES.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/templates/";
    let ret = db::list(key).await?;
    for (item_key, item_value) in ret {
        let item_key = item_key.strip_prefix(key).unwrap();
        let json_val: db_json::Template = json::from_slice(&item_value).unwrap();
        let template: Template = json_val.into();
        ALERTS_TEMPLATES.insert(item_key.to_owned(), template);
    }
    log::info!("Alert templates Cached");
    Ok(())
}
