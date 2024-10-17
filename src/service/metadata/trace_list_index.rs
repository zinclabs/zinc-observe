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
    hash::Hash,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use arrow_schema::{DataType, Field, Schema};
use config::{
    get_config,
    meta::stream::{StreamPartition, StreamSettings, StreamType},
    utils::{json, schema_ext::SchemaExt},
};
use infra::schema::unwrap_partition_time_level;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{
    common::meta::stream::SchemaRecords,
    service::{
        db, ingestion,
        metadata::{Metadata, MetadataItem},
        stream,
    },
};

const STREAM_NAME: &str = "trace_list_index";

static PARTITION_KEYS: Lazy<[StreamPartition; 1]> =
    Lazy::new(|| [StreamPartition::new("service_name")]);

pub(crate) static INSTANCE: Lazy<TraceListIndex> = Lazy::new(TraceListIndex::new);

pub struct TraceListIndex {
    schema: Arc<Schema>,
    db_schema_init: AtomicBool,
}

#[derive(Debug, Default, Eq, Hash, PartialEq, Clone, Serialize, Deserialize)]
pub struct TraceListItem {
    pub _timestamp: i64,
    pub stream_name: String,
    pub service_name: String,
    pub trace_id: String,
}

impl Metadata for TraceListIndex {
    fn generate_schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(
                get_config().common.column_timestamp.as_str(),
                DataType::Int64,
                false,
            ),
            Field::new("stream_name", DataType::Utf8, false),
            Field::new("service_name", DataType::Utf8, false),
            Field::new("trace_id", DataType::Utf8, false),
        ]))
    }
    async fn write(&self, org_id: &str, items: Vec<MetadataItem>) -> infra::errors::Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        // write to wal
        let timestamp = chrono::Utc::now().timestamp_micros();
        let schema_key = self.schema.hash_key();

        if !self.db_schema_init.load(Ordering::Relaxed) {
            self.set_db_schema(org_id).await?
        }

        let mut buf: HashMap<String, SchemaRecords> = HashMap::new();
        for item in items {
            let item = match item {
                MetadataItem::TraceListIndexer(item) => item,
                _ => {
                    continue;
                }
            };

            let mut data = json::to_value(item).unwrap();
            let data = data.as_object_mut().unwrap();
            let hour_key = ingestion::get_write_partition_key(
                timestamp,
                PARTITION_KEYS.to_vec().as_ref(),
                unwrap_partition_time_level(None, StreamType::Metadata),
                data,
                Some(&schema_key),
            );
            let data = json::Value::Object(data.clone());
            let data_size = json::to_vec(&data).unwrap_or_default().len();

            let hour_buf = buf.entry(hour_key).or_insert_with(|| SchemaRecords {
                schema_key: schema_key.clone(),
                schema: self.schema.clone(),
                records: vec![],
                records_size: 0,
            });

            hour_buf.records.push(Arc::new(data));
            hour_buf.records_size += data_size;
        }

        let writer =
            ingester::get_writer(org_id, &StreamType::Metadata.to_string(), STREAM_NAME).await;
        _ = ingestion::write_file(&writer, STREAM_NAME, buf).await;
        if let Err(e) = writer.sync().await {
            log::error!("[TraceListIndex] error while syncing writer: {}", e);
        }

        #[cfg(feature = "enterprise")]
        {
            use o2_enterprise::enterprise::{
                common::infra::config::get_config as get_o2_config,
                openfga::authorizer::authz::set_ownership_if_not_exists,
            };

            if get_o2_config().openfga.enabled {
                set_ownership_if_not_exists(
                    org_id,
                    &format!("{}:{}", StreamType::Metadata, STREAM_NAME),
                )
                .await;
            }
        }

        Ok(())
    }
    async fn flush(&self) -> infra::errors::Result<()> {
        Ok(()) // do nothing
    }
    async fn stop(&self) -> infra::errors::Result<()> {
        if let Err(e) = self.flush().await {
            log::error!("[TraceListIndex] flush error: {}", e);
        }
        Ok(())
    }
}

impl Default for TraceListIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceListIndex {
    pub fn new() -> Self {
        let mut res = Self {
            schema: Arc::new(Schema {
                fields: Default::default(),
                metadata: Default::default(),
            }),
            db_schema_init: AtomicBool::new(false),
        };

        res.schema = res.generate_schema();
        res
    }

    async fn set_db_schema(&self, org_id: &str) -> infra::errors::Result<()> {
        // check for schema
        let db_schema = infra::schema::get(org_id, STREAM_NAME, StreamType::Metadata)
            .await
            .unwrap();
        if db_schema.fields().is_empty() {
            let timestamp = chrono::Utc::now().timestamp_micros();
            let schema = self.schema.as_ref().clone();
            if let Err(e) = db::schema::merge(
                org_id,
                STREAM_NAME,
                StreamType::Metadata,
                &schema,
                Some(timestamp),
            )
            .await
            {
                log::error!("[TraceListIndex] error while setting schema: {}", e);
            }

            let settings = StreamSettings {
                partition_time_level: None,
                partition_keys: PARTITION_KEYS.to_vec(),
                full_text_search_keys: vec![],
                index_fields: vec![],
                bloom_filter_fields: vec!["trace_id".to_string()],
                data_retention: 0,
                flatten_level: None,
                max_query_range: 0,
                defined_schema_fields: None,
                store_original_data: false,
            };

            stream::save_stream_settings(org_id, STREAM_NAME, StreamType::Metadata, settings)
                .await?;
        }

        self.db_schema_init.store(true, Ordering::Release);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use config::{meta::stream::StreamType, utils::json};
    use infra::schema::unwrap_partition_time_level;

    use crate::{
        common::meta::stream::SchemaRecords,
        service::{
            ingestion,
            metadata::{
                trace_list_index::{TraceListIndex, TraceListItem, STREAM_NAME},
                Metadata, MetadataItem,
            },
        },
    };

    #[tokio::test]
    async fn test_write() {
        let t = TraceListIndex::new();
        let data = vec![MetadataItem::TraceListIndexer(TraceListItem::default())];

        let res = t.write("default", data).await;
        assert_eq!((), res.unwrap());
    }

    #[tokio::test]
    async fn test_trace_list_index_write_file() {
        let t = TraceListIndex::new();
        let mut buf: HashMap<String, SchemaRecords> = HashMap::new();
        let item = TraceListItem {
            stream_name: "default".to_string(),
            service_name: "oojaeger".to_string(),
            trace_id: "b09e986672880927996155acd4ef113c".to_string(),
            _timestamp: 1711267573271714542,
        };
        let schema_key = "9d384d5af30d1657";
        let timestamp = chrono::Utc::now().timestamp_micros();
        let mut data = json::to_value(item).unwrap();
        let data = data.as_object_mut().unwrap();
        data.insert(
            config::get_config().common.column_timestamp.clone(),
            json::Value::Number(timestamp.into()),
        );
        let hour_key = ingestion::get_write_partition_key(
            timestamp,
            &vec![],
            unwrap_partition_time_level(None, StreamType::Metadata),
            data,
            Some(schema_key),
        );
        let data = json::Value::Object(data.clone());
        let data_size = json::to_vec(&data).unwrap_or_default().len();
        let schema = t.generate_schema();
        let hour_buf = buf.entry(hour_key).or_insert_with(|| SchemaRecords {
            schema_key: schema_key.to_string(),
            schema,
            records: vec![],
            records_size: 0,
        });
        hour_buf.records.push(Arc::new(data));
        hour_buf.records_size += data_size;

        let writer = ingester::get_writer(
            "openobserve",
            &StreamType::Metadata.to_string(),
            STREAM_NAME,
        )
        .await;
        for (key, val) in buf.iter() {
            println!(
                "key: {key} val: {:?} schema: {}, records_size: {}, records: {:?}",
                val.schema_key, val.schema, val.records_size, val.records
            );
        }
        let r = ingestion::write_file(&writer, STREAM_NAME, buf).await;
        println!("r: {:?}", r);
    }
}
