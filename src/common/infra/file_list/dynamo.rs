// Copyright 2023 Zinc Labs Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use async_once::AsyncOnce;
use async_trait::async_trait;
use aws_sdk_dynamodb::{
    config::Region,
    operation::query::QueryOutput,
    types::{
        AttributeDefinition, AttributeValue, BillingMode, DeleteRequest, GlobalSecondaryIndex,
        KeySchemaElement, KeyType, Projection, ProjectionType, PutRequest, ScalarAttributeType,
        Select, WriteRequest,
    },
    Client,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use std::{
    cmp::{max, min},
    collections::HashMap,
};
use tokio_stream::StreamExt;

use crate::common::{
    infra::{
        config::CONFIG,
        errors::{Error, Result},
    },
    meta::{
        common::{FileKey, FileMeta},
        stream::{PartitionTimeLevel, StreamStats},
        StreamType,
    },
};

lazy_static! {
    static ref CLIENT: AsyncOnce<Client> = AsyncOnce::new(async { connect().await });
}

async fn connect() -> Client {
    if CONFIG.common.local_mode {
        let region = Region::new("us-west-2");
        let shared_config = aws_config::from_env()
            .region(region)
            .endpoint_url("http://localhost:8000");
        Client::new(&shared_config.load().await)
    } else {
        Client::new(&aws_config::load_from_env().await)
    }
}

pub struct DynamoFileList {
    table: String,
}

impl DynamoFileList {
    pub fn new() -> Self {
        Self {
            table: CONFIG.common.file_list_dynamo_table_name.clone(),
        }
    }
}

impl Default for DynamoFileList {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FileList for DynamoFileList {
    async fn add(&self, file: &str, meta: &FileMeta) -> Result<()> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let org_id = stream_key[..stream_key.find('/').unwrap()].to_string();
        let file_name = format!("{date_key}/{file_name}");
        CLIENT
            .get()
            .await
            .put_item()
            .table_name(&self.table)
            .item("org", AttributeValue::S(org_id))
            .item("stream", AttributeValue::S(stream_key))
            .item("file", AttributeValue::S(file_name))
            .item("deleted", AttributeValue::Bool(false))
            .item("min_ts", AttributeValue::N(meta.min_ts.to_string()))
            .item("max_ts", AttributeValue::N(meta.max_ts.to_string()))
            .item("records", AttributeValue::N(meta.records.to_string()))
            .item(
                "original_size",
                AttributeValue::N(meta.original_size.to_string()),
            )
            .item(
                "compressed_size",
                AttributeValue::N(meta.compressed_size.to_string()),
            )
            .item(
                "created_at",
                AttributeValue::N(Utc::now().timestamp_micros().to_string()),
            )
            .send()
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        Ok(())
    }

    async fn remove(&self, file: &str) -> Result<()> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let file_name = format!("{date_key}/{file_name}");
        let mut item = HashMap::new();
        item.insert("stream".to_string(), AttributeValue::S(stream_key));
        item.insert("file".to_string(), AttributeValue::S(file_name));
        CLIENT
            .get()
            .await
            .delete_item()
            .table_name(&self.table)
            .set_key(Some(item))
            .send()
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        Ok(())
    }

    async fn batch_add(&self, files: &[FileKey]) -> Result<()> {
        for batch in files.chunks(25) {
            let mut reqs: Vec<WriteRequest> = Vec::with_capacity(batch.len());
            for file in batch {
                let req = PutRequest::builder().set_item(Some(file.into())).build();
                reqs.push(WriteRequest::builder().put_request(req).build());
            }
            CLIENT
                .get()
                .await
                .batch_write_item()
                .request_items(&self.table, reqs)
                .send()
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    async fn batch_remove(&self, files: &[String]) -> Result<()> {
        for batch in files.chunks(25) {
            let mut reqs: Vec<WriteRequest> = Vec::with_capacity(batch.len());
            for file in batch {
                let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
                let file_name = format!("{date_key}/{file_name}");
                let mut item = HashMap::new();
                item.insert("stream".to_string(), AttributeValue::S(stream_key));
                item.insert("file".to_string(), AttributeValue::S(file_name));
                let req = DeleteRequest::builder().set_key(Some(item)).build();
                reqs.push(WriteRequest::builder().delete_request(req).build());
            }
            CLIENT
                .get()
                .await
                .batch_write_item()
                .request_items(&self.table, reqs)
                .send()
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    async fn get(&self, file: &str) -> Result<FileMeta> {
        let (stream_key, date_key, file_name) = super::parse_file_key_columns(file)?;
        let file_name = format!("{date_key}/{file_name}");

        let client = CLIENT.get().await;
        let resp = client
            .query()
            .table_name(&self.table)
            .key_condition_expression("#stream = :stream AND #file = :file")
            .expression_attribute_names("#stream", "stream".to_string())
            .expression_attribute_values(":stream", AttributeValue::S(stream_key))
            .expression_attribute_names("#file", "file".to_string())
            .expression_attribute_values(":file", AttributeValue::S(file_name))
            .select(Select::AllAttributes)
            .send()
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
        let items = resp.items().unwrap();
        if items.is_empty() {
            return Err(Error::Message("file not found".to_string()));
        }
        let file_key = FileKey::from(items.first().unwrap());
        Ok(file_key.meta)
    }

    async fn contains(&self, file: &str) -> Result<bool> {
        Ok(self.get(file).await.is_ok())
    }

    async fn list(&self) -> Result<Vec<(String, FileMeta)>> {
        return Ok(vec![]); // disallow list all data
    }

    async fn query(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        time_level: PartitionTimeLevel,
        time_range: (i64, i64),
    ) -> Result<Vec<(String, FileMeta)>> {
        let (time_start, mut time_end) = time_range;
        if time_start == 0 {
            return Err(Error::Message(
                "Disallow empty time range query".to_string(),
            ));
        }
        if time_end == 0 {
            time_end = Utc::now().timestamp_micros();
        }

        let t1: DateTime<Utc> = Utc.timestamp_nanos(time_start * 1000);
        let t2: DateTime<Utc> = Utc.timestamp_nanos(time_end * 1000) + Duration::hours(1);
        let (file_start, file_end) = if time_level == PartitionTimeLevel::Daily {
            (
                t1.format("%Y/%m/%d/00/").to_string(),
                t2.format("%Y/%m/%d/%H/").to_string(),
            )
        } else {
            (
                t1.format("%Y/%m/%d/%H/").to_string(),
                t2.format("%Y/%m/%d/%H/").to_string(),
            )
        };

        let stream_key = format!("{org_id}/{stream_type}/{stream_name}");

        let client = CLIENT.get().await;
        let resp: std::result::Result<Vec<QueryOutput>, _> = client
            .query()
            .table_name(&self.table)
            .key_condition_expression("#stream = :stream AND #file BETWEEN :file1 AND :file2 AND #min_ts <= :ts1 AND #max_ts >= :ts2")
            .expression_attribute_names("#stream", "stream".to_string())
            .expression_attribute_names("#file", "file".to_string())
            .expression_attribute_names("#min_ts", "min_ts".to_string())
            .expression_attribute_names("#max_ts", "max_ts".to_string())
            .expression_attribute_values(":stream", AttributeValue::S(stream_key))
            .expression_attribute_values(":file1", AttributeValue::S(file_start))
            .expression_attribute_values(":file2", AttributeValue::S(file_end))
            .expression_attribute_values(":ts1", AttributeValue::N(time_end.to_string()))
            .expression_attribute_values(":ts2", AttributeValue::S(time_start.to_string()))
            .select(Select::AllAttributes)
            .into_paginator()
            .page_size(1000)
            .send()
            .collect()
            .await;
        let resp = resp.map_err(|e| Error::Message(e.to_string()))?;

        // filter by time range
        let resp: Vec<_> = resp
            .iter()
            .filter(|v| v.count() > 0)
            .flat_map(|v| v.items().unwrap())
            .filter_map(|v| {
                let file = FileKey::from(v);
                if file.meta.min_ts <= time_end && file.meta.max_ts <= time_start {
                    Some((file.key.to_owned(), file.meta.to_owned()))
                } else {
                    None
                }
            })
            .collect();
        Ok(resp)
    }

    async fn get_max_pk_value(&self) -> Result<i64> {
        Ok(0)
    }

    async fn stats(
        &self,
        org_id: &str,
        stream_type: Option<StreamType>,
        stream_name: Option<&str>,
        pk_value: Option<(i64, i64)>,
    ) -> Result<Vec<(String, StreamStats)>> {
        let (time_start, time_end) = pk_value.unwrap_or((0, 0));
        let client = CLIENT.get().await;
        let query = if stream_type.is_some() && stream_name.is_some() {
            let stream_key = format!("{org_id}/{}/{}", stream_type.unwrap(), stream_name.unwrap());
            if time_start == 0 && time_end == 0 {
                client
                    .query()
                    .table_name(&self.table)
                    .index_name("org-created-at-index")
                    .key_condition_expression("#org = :org AND #stream = :stream")
                    .expression_attribute_names("#org", "org".to_string())
                    .expression_attribute_names("#stream", "stream".to_string())
                    .expression_attribute_values(":org", AttributeValue::S(org_id.to_string()))
                    .expression_attribute_values(":stream", AttributeValue::S(stream_key))
            } else {
                client.query()
            .table_name(&self.table).index_name("org-created-at-index")
            .key_condition_expression("#org = :org AND #stream = :stream AND #created_at1 > :ts1 AND #created_at2 <= :ts2")
            .expression_attribute_names("#org", "org".to_string())
            .expression_attribute_names("#stream", "stream".to_string())
            .expression_attribute_names("#created_at1", "created_at".to_string())
            .expression_attribute_names("#created_at2", "created_at".to_string())
            .expression_attribute_values(":org", AttributeValue::S(org_id.to_string()))
            .expression_attribute_values(":stream", AttributeValue::S(stream_key))
            .expression_attribute_values(":ts1", AttributeValue::S(time_start.to_string()))
            .expression_attribute_values(":ts2", AttributeValue::S(time_end.to_string()))
            }
        } else if time_start == 0 && time_end == 0 {
            client
                .query()
                .table_name(&self.table)
                .index_name("org-created-at-index")
                .key_condition_expression("#org = :org")
                .expression_attribute_names("#org", "org".to_string())
                .expression_attribute_values(":org", AttributeValue::S(org_id.to_string()))
        } else {
            client
                .query()
                .table_name(&self.table)
                .index_name("org-created-at-index")
                .key_condition_expression(
                    "#org = :org AND #created_at1 > :ts1 AND #created_at2 <= :ts2",
                )
                .expression_attribute_names("#org", "org".to_string())
                .expression_attribute_names("#created_at1", "created_at".to_string())
                .expression_attribute_names("#created_at2", "created_at".to_string())
                .expression_attribute_values(":org", AttributeValue::S(org_id.to_string()))
                .expression_attribute_values(":ts1", AttributeValue::S(time_start.to_string()))
                .expression_attribute_values(":ts2", AttributeValue::S(time_end.to_string()))
        };

        let resp: std::result::Result<Vec<QueryOutput>, _> = query
            .select(Select::AllAttributes)
            .into_paginator()
            .page_size(1000)
            .send()
            .collect()
            .await;
        let resp = resp.map_err(|e| Error::Message(e.to_string()))?;
        let resp: Vec<_> = resp
            .iter()
            .filter(|v| v.count() > 0)
            .flat_map(|v| v.items().unwrap())
            .map(|v| {
                let file = FileKey::from(v);
                (file.key.to_owned(), file.meta.to_owned())
            })
            .collect();

        // calculate stats
        let mut stats = HashMap::new();
        for (file, meta) in resp {
            let stream_stats = stats.entry(file).or_insert_with(StreamStats::default);
            stream_stats.file_num += 1;
            stream_stats.doc_time_min = min(stream_stats.doc_time_min, meta.min_ts);
            stream_stats.doc_time_max = max(stream_stats.doc_time_max, meta.max_ts);
            stream_stats.doc_num += meta.records;
            stream_stats.storage_size += meta.original_size as f64;
            stream_stats.compressed_size += meta.compressed_size as f64;
        }

        Ok(stats.into_iter().collect())
    }

    async fn get_stream_stats(
        &self,
        _org_id: &str,
        _stream_type: Option<StreamType>,
        _stream_name: Option<&str>,
    ) -> Result<Vec<(String, StreamStats)>> {
        Ok(vec![])
    }

    async fn set_stream_stats(
        &self,
        _org_id: &str,
        _streams: &[(String, StreamStats)],
    ) -> Result<()> {
        Ok(())
    }

    async fn len(&self) -> usize {
        0 // TODO
    }

    async fn is_empty(&self) -> bool {
        false // TODO
    }

    async fn clear(&self) -> Result<()> {
        Ok(()) // TODO
    }
}

pub async fn create_table() -> Result<()> {
    create_table_file_list().await?;
    create_table_stream_stats().await?;
    Ok(())
}

pub async fn create_table_index() -> Result<()> {
    create_table_file_list_index().await?;
    create_table_stream_stats_index().await?;
    Ok(())
}

pub async fn create_table_file_list() -> Result<()> {
    let client = CLIENT.get().await.clone();
    let table_name = &CONFIG.common.file_list_dynamo_table_name;
    let tables = client
        .list_tables()
        .send()
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    if tables
        .table_names()
        .unwrap_or(&[])
        .contains(&table_name.to_string())
    {
        return Ok(());
    }

    let key_schema = vec![
        KeySchemaElement::builder()
            .attribute_name("stream")
            .key_type(KeyType::Hash)
            .build(),
        KeySchemaElement::builder()
            .attribute_name("file")
            .key_type(KeyType::Range)
            .build(),
    ];
    let attribute_definitions = vec![
        AttributeDefinition::builder()
            .attribute_name("org")
            .attribute_type(ScalarAttributeType::S)
            .build(),
        AttributeDefinition::builder()
            .attribute_name("stream")
            .attribute_type(ScalarAttributeType::S)
            .build(),
        AttributeDefinition::builder()
            .attribute_name("file")
            .attribute_type(ScalarAttributeType::S)
            .build(),
        AttributeDefinition::builder()
            .attribute_name("created_at")
            .attribute_type(ScalarAttributeType::N)
            .build(),
    ];

    let index_created = GlobalSecondaryIndex::builder()
        .index_name("org-created-at-index")
        .set_key_schema(Some(vec![
            KeySchemaElement::builder()
                .attribute_name("org")
                .key_type(KeyType::Hash)
                .build(),
            KeySchemaElement::builder()
                .attribute_name("created_at")
                .key_type(KeyType::Range)
                .build(),
        ]))
        .set_projection(Some(
            Projection::builder()
                .projection_type(ProjectionType::All)
                .build(),
        ))
        .build();

    client
        .create_table()
        .table_name(table_name)
        .set_key_schema(Some(key_schema))
        .set_attribute_definitions(Some(attribute_definitions))
        .set_global_secondary_indexes(Some(vec![index_created]))
        .billing_mode(BillingMode::PayPerRequest)
        .send()
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

    log::info!("Table {} created successfully", table_name);

    Ok(())
}

pub async fn create_table_stream_stats() -> Result<()> {
    let client = CLIENT.get().await.clone();
    let table_name = &CONFIG.common.stream_stats_dynamo_table_name;
    let tables = client
        .list_tables()
        .send()
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    if tables
        .table_names()
        .unwrap_or(&[])
        .contains(&table_name.to_string())
    {
        return Ok(());
    }

    let key_schema = vec![
        KeySchemaElement::builder()
            .attribute_name("org")
            .key_type(KeyType::Hash)
            .build(),
        KeySchemaElement::builder()
            .attribute_name("stream")
            .key_type(KeyType::Range)
            .build(),
    ];
    let attribute_definitions = vec![
        AttributeDefinition::builder()
            .attribute_name("org")
            .attribute_type(ScalarAttributeType::S)
            .build(),
        AttributeDefinition::builder()
            .attribute_name("stream")
            .attribute_type(ScalarAttributeType::S)
            .build(),
    ];
    client
        .create_table()
        .table_name(table_name)
        .set_key_schema(Some(key_schema))
        .set_attribute_definitions(Some(attribute_definitions))
        .billing_mode(BillingMode::PayPerRequest)
        .send()
        .await
        .map_err(|e| Error::Message(e.to_string()))?;

    log::info!("Table {} created successfully", table_name);

    Ok(())
}

pub async fn create_table_file_list_index() -> Result<()> {
    Ok(())
}

pub async fn create_table_stream_stats_index() -> Result<()> {
    Ok(())
}
