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

use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use config::{
    get_config,
    ider::SnowflakeIdGenerator,
    meta::stream::{PartitionTimeLevel, StreamSettings, StreamType},
    utils::{json, schema_ext::SchemaExt},
    RwAHashMap, RwHashMap, BLOOM_FILTER_DEFAULT_FIELDS, SQL_FULL_TEXT_SEARCH_FIELDS,
};
use datafusion::arrow::datatypes::{DataType, Field, FieldRef, Schema, SchemaRef};
use futures::{StreamExt, TryStreamExt};
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::{
    db as infra_db,
    errors::{DbError, Error, Result},
};

pub mod history;

pub static STREAM_SCHEMAS: Lazy<RwAHashMap<String, Vec<(i64, Schema)>>> =
    Lazy::new(Default::default);
pub static STREAM_SCHEMAS_COMPRESSED: Lazy<RwAHashMap<String, Vec<(i64, bytes::Bytes)>>> =
    Lazy::new(Default::default);
pub static STREAM_SCHEMAS_LATEST: Lazy<RwAHashMap<String, SchemaCache>> =
    Lazy::new(Default::default);
pub static STREAM_SETTINGS: Lazy<RwAHashMap<String, StreamSettings>> = Lazy::new(Default::default);
/// Used for filtering records when a stream is configured to store original unflattened records
/// use a RwHashMap instead of RwAHashMap because of high write ratio as
/// SnowflakeIdGenerator::generate() requires a &mut
pub static STREAM_RECORD_ID_GENERATOR: Lazy<RwHashMap<String, SnowflakeIdGenerator>> =
    Lazy::new(Default::default);

pub async fn init() -> Result<()> {
    history::init().await?;
    Ok(())
}

pub fn mk_key(org_id: &str, stream_type: StreamType, stream_name: &str) -> String {
    format!("/schema/{org_id}/{stream_type}/{stream_name}")
}

pub async fn get(org_id: &str, stream_name: &str, stream_type: StreamType) -> Result<Schema> {
    let key = mk_key(org_id, stream_type, stream_name);
    let cache_key = key.strip_prefix("/schema/").unwrap();

    let r = STREAM_SCHEMAS_LATEST.read().await;
    if let Some(schema) = r.get(cache_key) {
        return Ok(schema.schema().as_ref().clone());
    }
    drop(r);
    // if not found in cache, get from db
    get_from_db(org_id, stream_name, stream_type).await
}

pub async fn get_cache(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
) -> Result<SchemaCache> {
    let key = mk_key(org_id, stream_type, stream_name);
    let cache_key = key.strip_prefix("/schema/").unwrap();

    let r = STREAM_SCHEMAS_LATEST.read().await;
    if let Some(schema) = r.get(cache_key) {
        return Ok(schema.clone());
    }
    drop(r);
    // if not found in cache, get from db
    let schema = get_from_db(org_id, stream_name, stream_type).await?;
    Ok(SchemaCache::new(schema))
}

pub async fn get_from_db(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
) -> Result<Schema> {
    let key = mk_key(org_id, stream_type, stream_name);
    let db = infra_db::get_db().await;
    Ok(match db.get(&key).await {
        Err(e) => {
            if let Error::DbError(DbError::KeyNotExists(_)) = e {
                Schema::empty()
            } else {
                return Err(Error::Message(format!("Error getting schema: {e}")));
            }
        }
        Ok(v) => {
            let schemas: Result<Vec<Schema>> = json::from_slice(&v).map_err(|e| e.into());
            if let Ok(mut schemas) = schemas {
                if schemas.is_empty() {
                    Schema::empty()
                } else {
                    schemas.remove(schemas.len() - 1)
                }
            } else {
                json::from_slice(&v)?
            }
        }
    })
}

#[tracing::instrument(name = "infra:schema:get_versions", skip_all)]
pub async fn get_versions(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    time_range: Option<(i64, i64)>,
) -> Result<Vec<Schema>> {
    let key = mk_key(org_id, stream_type, stream_name);
    let cache_key = key.strip_prefix("/schema/").unwrap();

    let cfg = get_config();
    let (min_ts, max_ts) = time_range.unwrap_or((0, 0));
    if cfg.common.schema_cache_compress_enabled {
        let mut last_schema_index = None;
        let r = STREAM_SCHEMAS_COMPRESSED.read().await;
        if let Some(versions) = r.get(cache_key) {
            let mut compressed_schemas = Vec::new();

            for (index, (start_dt, data)) in versions.iter().enumerate() {
                if *start_dt >= min_ts && (max_ts == 0 || *start_dt <= max_ts) {
                    compressed_schemas.push(data.clone());
                    if last_schema_index.is_none() {
                        last_schema_index = Some(index);
                    }
                }
            }

            if let Some(last_index) = last_schema_index {
                if last_index > 0 {
                    if let Some((_, data)) = versions.get(last_index - 1) {
                        // older version of schema before start_dt hence added in start
                        compressed_schemas.insert(0, data.clone());
                    }
                }
            } else {
                // this is latest version of schema hence added in end
                compressed_schemas.push(versions.last().unwrap().1.clone());
            }
            let schemas = futures::stream::iter(compressed_schemas)
                .map(|data| async move {
                    let de_bytes = zstd::decode_all(data.as_ref())?;
                    let mut schemas: Vec<Schema> = json::from_slice(&de_bytes)?;
                    Ok::<Option<Schema>, Error>(schemas.pop())
                })
                .buffer_unordered(cfg.limit.cpu_num)
                .try_collect::<Vec<Option<Schema>>>()
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
            return Ok(schemas.into_iter().flatten().collect());
        }
        drop(r);
    } else {
        let mut last_schema_index = None;
        let r = STREAM_SCHEMAS.read().await;
        if let Some(versions) = r.get(cache_key) {
            let mut schemas = Vec::new();

            for (index, (start_dt, data)) in versions.iter().enumerate() {
                if *start_dt >= min_ts && (max_ts == 0 || *start_dt <= max_ts) {
                    schemas.push(data.clone());
                    if last_schema_index.is_none() {
                        last_schema_index = Some(index);
                    }
                }
            }

            if let Some(last_index) = last_schema_index {
                if last_index > 0 {
                    if let Some((_, data)) = versions.get(last_index - 1) {
                        // older version of schema before start_dt should be added in start
                        schemas.insert(0, data.clone());
                    }
                }
            } else {
                // this is latest version of schema hence added in end
                schemas.push(versions.last().unwrap().1.clone());
            }

            return Ok(schemas);
        }
        drop(r);
    }

    let db = infra_db::get_db().await;
    Ok(match db.get(&key).await {
        Err(e) => {
            if let Error::DbError(DbError::KeyNotExists(_)) = e {
                vec![]
            } else {
                return Err(Error::Message(format!(
                    "Error getting schema versions: {e}",
                )));
            }
        }
        Ok(v) => {
            let schemas: Result<Vec<Schema>> = json::from_slice(&v).map_err(|e| e.into());
            if let Ok(schemas) = schemas {
                schemas
            } else {
                vec![json::from_slice(&v)?]
            }
        }
    })
}

pub async fn get_settings(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
) -> Option<StreamSettings> {
    let key = format!("{}/{}/{}", org_id, stream_type, stream_name);
    let r = STREAM_SETTINGS.read().await;
    if let Some(v) = r.get(&key) {
        return Some(v.clone());
    }
    // if not found in cache, get from db
    get(org_id, stream_name, stream_type)
        .await
        .ok()
        .and_then(|schema| unwrap_stream_settings(&schema))
}

pub fn unwrap_stream_settings(schema: &Schema) -> Option<StreamSettings> {
    if schema.metadata().is_empty() {
        return None;
    }
    schema
        .metadata()
        .get("settings")
        .map(|v| StreamSettings::from(v.as_str()))
}

pub fn unwrap_stream_created_at(schema: &Schema) -> Option<i64> {
    schema
        .metadata()
        .get("created_at")
        .and_then(|v| v.parse().ok())
}

pub fn unwrap_partition_time_level(
    level: Option<PartitionTimeLevel>,
    stream_type: StreamType,
) -> PartitionTimeLevel {
    let level = level.unwrap_or_default();
    if level != PartitionTimeLevel::Unset {
        level
    } else {
        let cfg = get_config();
        match stream_type {
            StreamType::Logs => PartitionTimeLevel::from(cfg.limit.logs_file_retention.as_str()),
            StreamType::Metrics => {
                PartitionTimeLevel::from(cfg.limit.metrics_file_retention.as_str())
            }
            StreamType::Traces => {
                PartitionTimeLevel::from(cfg.limit.traces_file_retention.as_str())
            }
            _ => PartitionTimeLevel::default(),
        }
    }
}

pub fn get_stream_setting_fts_fields(settings: &Option<StreamSettings>) -> Vec<String> {
    let default_fields = SQL_FULL_TEXT_SEARCH_FIELDS.clone();
    match settings {
        Some(settings) => {
            let mut fields = settings.full_text_search_keys.clone();
            fields.extend(default_fields);
            fields.sort();
            fields.dedup();
            fields
        }
        None => default_fields,
    }
}

pub fn get_stream_setting_bloom_filter_fields(settings: &Option<StreamSettings>) -> Vec<String> {
    let default_fields = BLOOM_FILTER_DEFAULT_FIELDS.clone();
    match settings {
        Some(settings) => {
            let mut fields = settings.bloom_filter_fields.clone();
            fields.extend(default_fields);
            fields.sort();
            fields.dedup();
            fields
        }
        None => default_fields,
    }
}

pub fn get_stream_setting_index_fields(settings: &Option<StreamSettings>) -> Vec<String> {
    match settings {
        Some(settings) => {
            let mut fields = settings.index_fields.clone();
            fields.sort();
            fields.dedup();
            fields
        }
        None => vec![],
    }
}

pub fn get_stream_setting_index_updated_at(
    settings: &Option<StreamSettings>,
    created_at: Option<i64>,
) -> i64 {
    let created_at = match created_at {
        Some(created_at) => created_at,
        None => {
            log::warn!("created_at not found in schema metadata");
            Utc::now().timestamp_micros()
        }
    };
    match settings {
        Some(settings) => {
            if settings.index_updated_at > 0 {
                settings.index_updated_at
            } else {
                created_at
            }
        }
        None => created_at,
    }
}

pub async fn merge(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    schema: &Schema,
    min_ts: Option<i64>,
) -> Result<Option<(Schema, Vec<Field>)>> {
    let start_dt = min_ts;
    let key = mk_key(org_id, stream_type, stream_name);
    let inferred_schema = schema.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let db = infra_db::get_db().await;
    db.get_for_update(
        &key.clone(),
        infra_db::NEED_WATCH,
        None,
        Box::new(move |value| {
            match value {
                None => Ok(Some((
                    None,
                    Some((
                        key,
                        json::to_vec(&vec![{
                            // there is no schema, just set the new schema
                            let schema_metadata = inferred_schema.metadata();
                            let inferred_schema = if schema_metadata.contains_key("created_at")
                                && schema_metadata.contains_key("start_dt")
                            {
                                inferred_schema
                            } else {
                                let start_dt =
                                    start_dt.unwrap_or_else(|| Utc::now().timestamp_micros());
                                let mut schema_metadata = inferred_schema.metadata().clone();
                                if !schema_metadata.contains_key("created_at") {
                                    schema_metadata
                                        .insert("created_at".to_string(), start_dt.to_string());
                                }
                                if !schema_metadata.contains_key("start_dt") {
                                    schema_metadata
                                        .insert("start_dt".to_string(), start_dt.to_string());
                                }
                                inferred_schema.with_metadata(schema_metadata)
                            };
                            tx.send(Some((inferred_schema.clone(), vec![]))).unwrap();
                            inferred_schema
                        }])
                        .unwrap()
                        .into(),
                        start_dt,
                    )),
                ))),
                Some(value) => {
                    // there is schema, merge the schema
                    // parse latest schema
                    let mut schemas: Vec<Schema> = json::from_slice(&value)?;
                    let latest_schema = match schemas.last_mut() {
                        Some(s) => s,
                        None => {
                            return Err(Error::Message(format!(
                                "Error parsing latest schema for schema: {}",
                                key
                            )));
                        }
                    };
                    // merge schema
                    let (is_schema_changed, field_datatype_delta, merged_fields) =
                        get_merge_schema_changes(latest_schema, &inferred_schema);

                    if !is_schema_changed {
                        tx.send(Some((latest_schema.clone(), field_datatype_delta)))
                            .unwrap();
                        return Ok(None); // no change, return
                    }
                    let metadata = latest_schema.metadata().clone();
                    let final_schema = Schema::new(merged_fields).with_metadata(metadata);

                    // Casting of data to existing schema isnt new version, we remove records
                    // with zo_cast metadata
                    let schema_version_changes = field_datatype_delta
                        .iter()
                        .filter(|f| f.metadata().get("zo_cast").is_none())
                        .collect::<Vec<_>>();
                    let need_new_version = !schema_version_changes.is_empty();

                    if need_new_version && start_dt.is_some() {
                        // update old version end_dt
                        let mut metadata = latest_schema.metadata().clone();
                        metadata.insert("end_dt".to_string(), start_dt.unwrap().to_string());
                        let prev_schema = vec![latest_schema.clone().with_metadata(metadata)];
                        let mut new_metadata = latest_schema.metadata().clone();
                        new_metadata.insert("start_dt".to_string(), start_dt.unwrap().to_string());
                        let new_schema = vec![final_schema.clone().with_metadata(new_metadata)];
                        tx.send(Some((final_schema, field_datatype_delta))).unwrap();
                        Ok(Some((
                            Some(json::to_vec(&prev_schema).unwrap().into()),
                            Some((key, json::to_vec(&new_schema).unwrap().into(), start_dt)),
                        )))
                    } else {
                        // just update the latest schema
                        tx.send(Some((final_schema.clone(), field_datatype_delta)))
                            .unwrap();
                        Ok(Some((
                            Some(json::to_vec(&vec![final_schema]).unwrap().into()),
                            None,
                        )))
                    }
                }
            }
        }),
    )
    .await?;
    rx.await.map_err(|e| Error::Message(e.to_string()))
}

pub async fn update_setting(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    metadata: std::collections::HashMap<String, String>,
) -> Result<()> {
    let key = mk_key(org_id, stream_type, stream_name);
    let db = infra_db::get_db().await;
    db.get_for_update(
        &key.clone(),
        infra_db::NEED_WATCH,
        None,
        Box::new(move |value| {
            let (latest_schema, not_exists) = match value {
                None => (Schema::empty(), true),
                Some(value) => {
                    let mut schemas: Vec<Schema> = json::from_slice(&value)?;
                    if schemas.is_empty() {
                        (Schema::empty(), false)
                    } else {
                        (schemas.remove(schemas.len() - 1), false)
                    }
                }
            };
            let mut schema_metadata = latest_schema.metadata().clone();
            for (k, v) in metadata.iter() {
                schema_metadata.insert(k.clone(), v.clone());
            }
            let start_dt = match schema_metadata.get("created_at") {
                Some(v) => v.parse().unwrap(),
                None => Utc::now().timestamp_micros(),
            };
            if !schema_metadata.contains_key("created_at") {
                schema_metadata.insert("created_at".to_string(), start_dt.to_string());
            }
            if !schema_metadata.contains_key("start_dt") {
                schema_metadata.insert("start_dt".to_string(), start_dt.to_string());
            }
            let new_schema = vec![latest_schema.with_metadata(schema_metadata)];
            if not_exists {
                Ok(Some((
                    None,
                    Some((
                        key,
                        json::to_vec(&new_schema).unwrap().into(),
                        Some(start_dt),
                    )),
                )))
            } else {
                Ok(Some((
                    Some(json::to_vec(&new_schema).unwrap().into()),
                    None,
                )))
            }
        }),
    )
    .await?;

    Ok(())
}

pub async fn delete_fields(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    deleted_fields: Vec<String>,
) -> Result<()> {
    let key = mk_key(org_id, stream_type, stream_name);
    let db = infra_db::get_db().await;
    db.get_for_update(
        &key.clone(),
        infra_db::NEED_WATCH,
        None,
        Box::new(move |value| {
            let Some(value) = value else {
                return Ok(None);
            };
            let mut schemas: Vec<Schema> = json::from_slice(&value)?;
            let latest_schema = if schemas.is_empty() {
                return Ok(None);
            } else {
                schemas.remove(schemas.len() - 1)
            };
            let start_dt = Utc::now().timestamp_micros();
            // update previous version schema
            let mut latest_metadata = latest_schema.metadata().clone();
            latest_metadata.insert("end_dt".to_string(), start_dt.to_string());
            let prev_schema = vec![latest_schema.clone().with_metadata(latest_metadata)];
            // new version schema
            let mut new_metadata = latest_schema.metadata().clone();
            new_metadata.insert("start_dt".to_string(), start_dt.to_string());
            let fields = latest_schema
                .fields()
                .iter()
                .filter_map(|f| {
                    if deleted_fields.contains(&f.name().to_string()) {
                        None
                    } else {
                        Some(f.clone())
                    }
                })
                .collect::<Vec<_>>();

            let mut settings = unwrap_stream_settings(&latest_schema).unwrap_or_default();

            if let Some(schema_fields) = settings.defined_schema_fields {
                let defined_schema_fields = schema_fields
                    .iter()
                    .filter_map(|f| {
                        if deleted_fields.contains(f) {
                            None
                        } else {
                            Some(f.clone())
                        }
                    })
                    .collect::<Vec<_>>();
                if defined_schema_fields.is_empty() {
                    settings.defined_schema_fields = None;
                } else {
                    settings.defined_schema_fields = Some(defined_schema_fields);
                }
            };
            new_metadata.insert("settings".to_string(), json::to_string(&settings).unwrap());
            let new_schema = vec![Schema::new_with_metadata(fields, new_metadata)];
            Ok(Some((
                Some(json::to_vec(&prev_schema).unwrap().into()),
                Some((
                    key,
                    json::to_vec(&new_schema).unwrap().into(),
                    Some(start_dt),
                )),
            )))
        }),
    )
    .await?;

    Ok(())
}

pub async fn delete(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    start_dt: Option<i64>,
) -> Result<()> {
    let key = format!("/schema/{org_id}/{stream_type}/{stream_name}");
    let db = infra_db::get_db().await;
    match db.delete(&key, false, infra_db::NEED_WATCH, start_dt).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error deleting schema: {}", e);
            return Err(Error::Message(format!("Error deleting schema: {e}")));
        }
    }
    Ok(())
}

pub fn get_merge_schema_changes(
    schema: &Schema,
    inferred_schema: &Schema,
) -> (bool, Vec<Field>, Vec<Field>) {
    let mut is_schema_changed = false;
    let mut field_datatype_delta: Vec<_> = vec![];

    let mut merged_fields = schema.fields().iter().collect::<Vec<_>>();
    let mut merged_fields_chk = hashbrown::HashMap::with_capacity(merged_fields.len());
    for (i, f) in merged_fields.iter().enumerate() {
        merged_fields_chk.insert(f.name(), i);
    }

    for item in inferred_schema.fields.iter() {
        let item_name = item.name();
        let item_data_type = item.data_type();

        match merged_fields_chk.get(item_name) {
            None => {
                is_schema_changed = true;
                merged_fields.push(item);
                merged_fields_chk.insert(item_name, merged_fields.len() - 1);
            }
            Some(idx) => {
                let existing_field = &merged_fields[*idx];
                if existing_field.data_type() != item_data_type {
                    if !get_config().common.widening_schema_evolution {
                        field_datatype_delta.push(existing_field.as_ref().clone());
                    } else if is_widening_conversion(existing_field.data_type(), item_data_type) {
                        is_schema_changed = true;
                        merged_fields[*idx] = item;
                        field_datatype_delta.push((**item).clone());
                    } else {
                        let mut meta = existing_field.metadata().clone();
                        meta.insert("zo_cast".to_owned(), true.to_string());
                        field_datatype_delta
                            .push(existing_field.as_ref().clone().with_metadata(meta));
                    }
                }
            }
        }
    }
    if !is_schema_changed {
        (false, field_datatype_delta, vec![])
    } else {
        (
            true,
            field_datatype_delta,
            merged_fields
                .into_iter()
                .map(|f| f.as_ref().clone())
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SchemaCache {
    schema: SchemaRef,
    fields_map: HashMap<String, usize>,
    hash_key: String,
}

impl SchemaCache {
    pub fn new(schema: Schema) -> Self {
        Self::new_from_arc(Arc::new(schema))
    }

    pub fn new_from_arc(schema: Arc<Schema>) -> Self {
        let hash_key = schema.hash_key();
        let fields_map = schema
            .fields()
            .iter()
            .enumerate()
            .map(|(i, f)| (f.name().to_owned(), i))
            .collect();
        Self {
            schema,
            fields_map,
            hash_key,
        }
    }

    pub fn hash_key(&self) -> &str {
        &self.hash_key
    }

    pub fn schema(&self) -> &Arc<Schema> {
        &self.schema
    }

    pub fn fields_map(&self) -> &HashMap<String, usize> {
        &self.fields_map
    }

    pub fn contains_field(&self, field_name: &str) -> bool {
        self.fields_map.contains_key(field_name)
    }

    pub fn field_with_name(&self, field_name: &str) -> Option<&FieldRef> {
        self.fields_map
            .get(field_name)
            .and_then(|i| self.schema.fields().get(*i))
    }
}

pub fn is_widening_conversion(from: &DataType, to: &DataType) -> bool {
    let allowed_type = match from {
        DataType::Boolean => vec![DataType::Utf8],
        DataType::Int8 => vec![
            DataType::Utf8,
            DataType::Int16,
            DataType::Int32,
            DataType::Int64,
            DataType::Float16,
            DataType::Float32,
            DataType::Float64,
        ],
        DataType::Int16 => vec![
            DataType::Utf8,
            DataType::Int32,
            DataType::Int64,
            DataType::Float16,
            DataType::Float32,
            DataType::Float64,
        ],
        DataType::Int32 => vec![
            DataType::Utf8,
            DataType::Int64,
            DataType::UInt32,
            DataType::UInt64,
            DataType::Float32,
            DataType::Float64,
        ],
        DataType::Int64 => vec![DataType::Utf8, DataType::UInt64, DataType::Float64],
        DataType::UInt8 => vec![
            DataType::Utf8,
            DataType::UInt16,
            DataType::UInt32,
            DataType::UInt64,
        ],
        DataType::UInt16 => vec![DataType::Utf8, DataType::UInt32, DataType::UInt64],
        DataType::UInt32 => vec![DataType::Utf8, DataType::UInt64],
        DataType::UInt64 => vec![DataType::Utf8],
        DataType::Float16 => vec![DataType::Utf8, DataType::Float32, DataType::Float64],
        DataType::Float32 => vec![DataType::Utf8, DataType::Float64],
        DataType::Float64 => vec![DataType::Utf8],
        _ => vec![DataType::Utf8],
    };
    allowed_type.contains(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_widening_conversion() {
        assert!(is_widening_conversion(&DataType::Int8, &DataType::Int32));
    }

    #[test]
    fn test_get_stream_setting_fts_fields() {
        let schema = Schema::new(vec![Field::new("f.c", DataType::Int32, false)]);
        let settings = unwrap_stream_settings(&schema);
        let res = get_stream_setting_fts_fields(&settings);
        assert!(!res.is_empty());
    }
}
