// Copyright 2024 Zinc Labs Inc.
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

use ::datafusion::arrow::datatypes::Schema;
use arrow::array::{
    make_builder, new_null_array, ArrayBuilder, ArrayRef, RecordBatch, StringArray, StringBuilder,
};
use arrow_schema::{DataType, Field};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        cluster::Role,
        stream::{FileKey, PartitionTimeLevel, StreamType},
    },
    utils::{
        json,
        parquet::{read_recordbatch_from_bytes, write_recordbatch_to_parquet},
    },
    FxIndexMap,
};
use hashbrown::HashSet;
use infra::{file_list as infra_file_list, storage};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use tokio::sync::{mpsc, Semaphore};

use crate::{common::infra::cluster::get_node_from_consistent_hash, service::db};

static PROCESSING_FILES: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));

pub async fn run_generate(worker_tx: mpsc::Sender<FileKey>) -> Result<(), anyhow::Error> {
    let semaphore = std::sync::Arc::new(Semaphore::new(get_config().limit.file_merge_thread_num));
    let orgs = db::schema::list_organizations_from_cache().await;
    let stream_types = [StreamType::Logs];
    for org_id in orgs {
        // check backlist
        if !db::file_list::BLOCKED_ORGS.is_empty() && db::file_list::BLOCKED_ORGS.contains(&org_id)
        {
            continue;
        }
        for stream_type in stream_types {
            let streams = db::schema::list_streams_from_cache(&org_id, stream_type).await;
            let mut tasks = Vec::with_capacity(streams.len());
            for stream_name in streams {
                // check if this stream need flatten
                let stream_setting =
                    infra::schema::get_settings(&org_id, &stream_name, stream_type)
                        .await
                        .unwrap_or_default();
                let defined_schema_fields =
                    stream_setting.defined_schema_fields.unwrap_or_default();
                if defined_schema_fields.is_empty() {
                    continue;
                }

                // check running node
                let Some(node_name) =
                    get_node_from_consistent_hash(&stream_name, &Role::FlattenCompactor, None)
                        .await
                else {
                    continue; // no compactor node
                };
                if LOCAL_NODE.name.ne(&node_name) {
                    continue; // not this node
                }

                let org_id = org_id.clone();
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let worker_tx = worker_tx.clone();
                let task = tokio::task::spawn(async move {
                    if let Err(e) =
                        generate_by_stream(worker_tx, &org_id, stream_type, &stream_name).await
                    {
                        log::error!(
                            "[FLATTEN_COMPACTOR] generate_by_stream [{}/{}/{}] error: {}",
                            org_id,
                            stream_type,
                            stream_name,
                            e
                        );
                    }
                    drop(permit);
                });
                tasks.push(task);
            }
            for task in tasks {
                task.await?;
            }
        }
    }

    Ok(())
}

pub async fn generate_by_stream(
    worker_tx: mpsc::Sender<FileKey>,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();

    // get pending flatten files for this stream
    let files = infra_file_list::query(
        org_id,
        stream_type,
        stream_name,
        PartitionTimeLevel::Hourly,
        None,
        Some(false),
    )
    .await?;

    if files.is_empty() {
        return Ok(()); // no files need to generate
    }

    log::info!(
        "[FLATTEN_COMPACTOR] generate_by_stream [{}/{}/{}] got files: {}, took: {} ms",
        org_id,
        stream_type,
        stream_name,
        files.len(),
        start.elapsed().as_millis()
    );

    for (file, meta) in files {
        if PROCESSING_FILES.read().contains(&file) {
            continue;
        }
        let file = FileKey {
            key: file,
            meta,
            deleted: false,
            segment_ids: None,
        };
        // add into queue
        PROCESSING_FILES.write().insert(file.key.clone());
        worker_tx.send(file).await?;
    }

    PROCESSING_FILES.write().shrink_to_fit();
    Ok(())
}

pub async fn generate_file(file: &FileKey) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();
    log::debug!("[FLATTEN_COMPACTOR] generate flatten file for {}", file.key);

    let data = storage::get(&file.key).await?;
    let (_, batches) = read_recordbatch_from_bytes(&data)
        .await
        .map_err(|e| anyhow::anyhow!("read_recordbatch_from_bytes error: {}", e))?;
    let new_batches = generate_vertical_partition_recordbatch(&batches)
        .map_err(|e| anyhow::anyhow!("generate_vertical_partition_recordbatch error: {}", e))?;

    if new_batches.is_empty() {
        storage::del(&[&file.key]).await?;
        return Ok(());
    }
    let columns = file.key.splitn(9, '/').collect::<Vec<&str>>();
    if columns.len() < 9 {
        return Err(anyhow::anyhow!(
            "[FLATTEN_COMPACTOR] Invalid file path: {}",
            file.key
        ));
    }
    let org_id = columns[1];
    let stream_type = StreamType::from(columns[2]);
    let stream_name = columns[3];
    let stream_setting = infra::schema::get_settings(org_id, stream_name, stream_type)
        .await
        .unwrap_or_default();
    let bloom_filter_fields = stream_setting.bloom_filter_fields;
    let full_text_search_fields = stream_setting.full_text_search_keys;
    let new_file = format!(
        "files{}/{}",
        get_config().common.column_all,
        file.key.strip_prefix("files/").unwrap()
    );
    let new_schema = new_batches.first().unwrap().schema();
    let new_data = write_recordbatch_to_parquet(
        new_schema,
        &new_batches,
        &bloom_filter_fields,
        &full_text_search_fields,
        &file.meta,
    )
    .await
    .map_err(|e| anyhow::anyhow!("write_recordbatch_to_parquet error: {}", e))?;
    // upload filee
    storage::put(&new_file, new_data.into()).await?;
    // delete from queue
    PROCESSING_FILES.write().remove(&file.key);
    log::info!(
        "[FLATTEN_COMPACTOR] generated flatten new file {}, took {} ms",
        new_file,
        start.elapsed().as_millis()
    );
    // update file list
    infra_file_list::update_flattened(&file.key, true).await?;
    Ok(())
}

fn generate_vertical_partition_recordbatch(
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, anyhow::Error> {
    if batches.is_empty() {
        return Ok(Vec::new());
    }
    let schema = batches.first().unwrap().schema();
    let batches = arrow::compute::concat_batches(&schema, batches)?;
    let records_len = batches.num_rows();
    if records_len == 0 {
        return Ok(Vec::new());
    }
    let Ok(all_field_idx) = schema.index_of(&get_config().common.column_all) else {
        return Ok(vec![batches]);
    };

    let mut builders: FxIndexMap<String, Box<dyn ArrayBuilder>> = Default::default();
    let Some(all_values) = batches
        .column(all_field_idx)
        .as_any()
        .downcast_ref::<StringArray>()
    else {
        return Ok(vec![batches]);
    };

    let mut inserted_fields = HashSet::with_capacity(128);
    for i in 0..records_len {
        inserted_fields.clear();
        let value = all_values.value(i);
        let data = if value.is_empty() {
            json::Value::Object(Default::default())
        } else {
            json::from_str(value).map_err(|e| {
                anyhow::anyhow!("parse all fields value error: {}, value: {}", e, value)
            })?
        };
        let items = data.as_object().unwrap();
        for (key, val) in items {
            let builder = builders.entry(key.to_string()).or_insert_with(|| {
                let mut builder = make_builder(&DataType::Utf8, records_len);
                if i > 0 {
                    let b = builder
                        .as_any_mut()
                        .downcast_mut::<StringBuilder>()
                        .unwrap();
                    for _ in 0..i {
                        b.append_null();
                    }
                }
                builder
            });
            let b = builder
                .as_any_mut()
                .downcast_mut::<StringBuilder>()
                .unwrap();
            if val.is_null() {
                b.append_null();
            } else {
                b.append_value(json::get_string_value(val));
            }
            inserted_fields.insert(key.to_string());
        }
        for (key, builder) in builders.iter_mut() {
            if !inserted_fields.contains(key) {
                let b = builder
                    .as_any_mut()
                    .downcast_mut::<StringBuilder>()
                    .unwrap();
                b.append_null();
            }
        }
    }

    let mut fields = schema.fields().iter().cloned().collect::<Vec<_>>();
    let mut cols: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len() + builders.len());
    for i in 0..schema.fields().len() {
        if i == all_field_idx {
            cols.push(new_null_array(&DataType::Utf8, records_len));
        } else {
            cols.push(batches.column(i).clone());
        }
    }
    for (key, builder) in builders.iter_mut() {
        fields.push(Arc::new(Field::new(key, DataType::Utf8, true)));
        cols.push(builder.finish());
    }

    let schema = Arc::new(Schema::new(fields));
    Ok(vec![RecordBatch::try_new(schema, cols)?])
}
