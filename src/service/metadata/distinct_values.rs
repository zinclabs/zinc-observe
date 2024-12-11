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

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use arrow_schema::{DataType, Field, Schema};
use config::{
    get_config,
    meta::stream::StreamType,
    utils::{json, schema::infer_json_schema_from_map, schema_ext::SchemaExt},
    FxIndexMap,
};
use infra::{
    errors::{Error, Result},
    schema::unwrap_partition_time_level,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::{
    sync::{mpsc, RwLock},
    time,
};

use crate::{
    common::meta::stream::SchemaRecords,
    service,
    service::{
        ingestion,
        metadata::{Metadata, MetadataItem},
    },
};

const CHANNEL_SIZE: usize = 10240;
pub const DISTINCT_STREAM_PREFIX: &str = "distinct_values";

pub(crate) static INSTANCE: Lazy<DistinctValues> = Lazy::new(DistinctValues::new);

type MemTable = FxIndexMap<String, FxIndexMap<DvItem, u32>>;

pub struct DistinctValues {
    channel: Arc<mpsc::Sender<DvEvent>>,
    shutdown: Arc<AtomicBool>,
    mem_table: Arc<RwLock<MemTable>>,
}

#[derive(Debug, Default, Eq, Hash, PartialEq, Clone, Serialize, Deserialize)]
pub struct DvItem {
    pub stream_type: StreamType,
    pub stream_name: String,
    pub value: Map<String, Value>,
}

#[derive(Debug)]
enum DvEventType {
    Add,
    Shutudown,
}

#[derive(Debug)]
struct DvEvent {
    org_id: String,
    item: DvItem,
    count: u32,
    ev_type: DvEventType,
}

impl DvEvent {
    pub fn new(org_id: &str, item: DvItem, count: u32) -> Self {
        Self {
            org_id: org_id.to_string(),
            item,
            count,
            ev_type: DvEventType::Add,
        }
    }
    pub fn shutdown() -> Self {
        Self {
            org_id: String::from(""),
            item: DvItem::default(),
            count: 0,
            ev_type: DvEventType::Shutudown,
        }
    }
}

impl Default for DistinctValues {
    fn default() -> Self {
        Self::new()
    }
}

impl DistinctValues {
    pub fn new() -> Self {
        tokio::task::spawn(async move { run_flush().await });
        Self {
            channel: handle_channel(),
            shutdown: Arc::new(AtomicBool::new(false)),
            mem_table: Arc::new(RwLock::new(FxIndexMap::default())),
        }
    }
}

fn handle_channel() -> Arc<mpsc::Sender<DvEvent>> {
    let (tx, mut rx) = mpsc::channel::<DvEvent>(CHANNEL_SIZE);
    tokio::task::spawn(async move {
        loop {
            let event = match rx.recv().await {
                Some(v) => v,
                None => {
                    log::info!("[DISTINCT_VALUES] event channel closed");
                    break;
                }
            };
            if let DvEventType::Shutudown = event.ev_type {
                if let Err(e) = INSTANCE.flush().await {
                    log::error!("[DISTINCT_VALUES] flush error: {}", e);
                }
                INSTANCE.shutdown.store(true, Ordering::Release);
                break;
            }
            let mut mem_table = INSTANCE.mem_table.write().await;
            let entry = mem_table.entry(event.org_id).or_default();
            let field_entry = entry.entry(event.item).or_default();
            *field_entry += event.count;
        }
        log::info!("[DISTINCT_VALUES] event loop exit");
    });
    Arc::new(tx)
}

impl Metadata for DistinctValues {
    fn generate_schema(&self) -> Arc<Schema> {
        // distinct values will always have _timestamp and
        // count, rest will be dynamically determined
        Arc::new(Schema::new(vec![
            Field::new(
                get_config().common.column_timestamp.as_str(),
                DataType::Int64,
                false,
            ),
            Field::new("count", DataType::Int64, false),
        ]))
    }

    async fn write(&self, org_id: &str, data: Vec<MetadataItem>) -> Result<()> {
        let mut group_items: FxIndexMap<DvItem, u32> = FxIndexMap::default();
        for item in data {
            if let MetadataItem::DistinctValues(item) = item {
                let count = group_items.entry(item).or_default();
                *count += 1;
            }
        }
        for (item, count) in group_items {
            self.channel
                .send(DvEvent::new(org_id, item, count))
                .await
                .map_err(|v| Error::Message(v.to_string()))?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        let cfg = get_config();
        let mut mem_table = self.mem_table.write().await;
        let mut new_table: MemTable = FxIndexMap::default();
        std::mem::swap(&mut new_table, &mut *mem_table);
        drop(mem_table);

        // write to wal
        let timestamp = chrono::Utc::now().timestamp_micros();
        let default_schema = self.generate_schema();

        // transpose the table
        let mut table: HashMap<_, Vec<(Map<String, Value>, u32)>> = HashMap::new();
        for (org_id, items) in new_table {
            for (item, count) in items {
                let key = (org_id.clone(), item.stream_name, item.stream_type);
                let entry = table.entry(key).or_default();
                entry.push((item.value, count));
            }
        }

        for ((org_id, stream_name, stream_type), items) in table {
            if items.is_empty() {
                continue;
            }

            let distinct_stream_name = format!(
                "{}_{}_{}",
                DISTINCT_STREAM_PREFIX,
                stream_type.as_str(),
                stream_name
            );
            // check for schema
            let db_schema =
                infra::schema::get(&org_id, &distinct_stream_name, StreamType::Metadata).await?;
            let mut _is_new = false;
            if db_schema.fields().is_empty() {
                _is_new = true;
                let schema = default_schema.as_ref().clone();
                if let Err(e) = service::db::schema::merge(
                    &org_id,
                    &distinct_stream_name,
                    StreamType::Metadata,
                    &schema,
                    Some(timestamp),
                )
                .await
                {
                    log::error!("[DISTINCT_VALUES] error while setting schema: {}", e);
                }
            }

            let inferred_schema =
                infer_json_schema_from_map(items.iter().map(|(v, _)| v), stream_type)?;

            let mut schema_key = db_schema.hash_key();
            let mut schema = inferred_schema;
            if _is_new || db_schema.fields.ne(&schema.fields) {
                match service::db::schema::merge(
                    &org_id,
                    &distinct_stream_name,
                    StreamType::Metadata,
                    &schema,
                    Some(timestamp),
                )
                .await
                {
                    Err(e) => {
                        log::error!(
                            "[DISTINCT_VALUES] error while updating schema for {org_id}/{stream_name} : {e}"
                        );
                        schema_key = schema.hash_key()
                    }
                    Ok(None) => schema_key = schema.hash_key(),
                    Ok(Some((s, _))) => {
                        schema_key = s.hash_key();
                        schema = s;
                    }
                }
            }

            let schema = Arc::new(schema);
            let mut buf: HashMap<String, SchemaRecords> = HashMap::new();
            for (item, count) in items {
                let mut data = json::to_value(item).unwrap();
                let data = data.as_object_mut().unwrap();
                data.insert("count".to_string(), json::Value::Number(count.into()));
                data.insert(
                    cfg.common.column_timestamp.clone(),
                    json::Value::Number(timestamp.into()),
                );
                let hour_key = ingestion::get_write_partition_key(
                    timestamp,
                    &vec![],
                    unwrap_partition_time_level(None, StreamType::Metadata),
                    data,
                    Some(&schema_key),
                );
                let data = json::Value::Object(data.clone());
                let data_size = json::to_vec(&data).unwrap_or_default().len();

                let hour_buf = buf.entry(hour_key).or_insert_with(|| SchemaRecords {
                    schema_key: schema_key.clone(),
                    schema: schema.clone(),
                    records: vec![],
                    records_size: 0,
                });
                hour_buf.records.push(Arc::new(data));
                hour_buf.records_size += data_size;
            }

            let writer = ingester::get_writer(
                0,
                &org_id,
                &StreamType::Metadata.to_string(),
                &distinct_stream_name,
            )
            .await;
            _ = ingestion::write_file(
                &writer,
                &distinct_stream_name,
                buf,
                !cfg.common.wal_fsync_disabled,
            )
            .await;

            #[cfg(feature = "enterprise")]
            {
                use o2_enterprise::enterprise::{
                    common::infra::config::get_config as get_o2_config,
                    openfga::authorizer::authz::set_ownership_if_not_exists,
                };

                // set ownership only in the first time
                if _is_new && get_o2_config().openfga.enabled {
                    set_ownership_if_not_exists(
                        &org_id,
                        &format!("{}:{}", StreamType::Metadata, distinct_stream_name),
                    )
                    .await;
                }
            }
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let tx = self.channel.clone();
        tx.send(DvEvent::shutdown())
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        let mut i = 0;
        while i < 10 {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            log::info!("[DISTINCT_VALUES] shutting down");
            i += 1;
        }
        Ok(())
    }
}

async fn run_flush() {
    let mut interval = time::interval(time::Duration::from_secs(
        get_config().limit.distinct_values_interval,
    ));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = INSTANCE.flush().await {
            log::error!("[DISTINCT_VALUES] error flush data to wal: {}", e);
        }
    }
}
