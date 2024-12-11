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

use std::{collections::HashMap, io::BufReader, sync::Arc};

use actix_web::{http, web};
use anyhow::{anyhow, Result};
use config::{
    cluster::LOCAL_NODE,
    meta::{
        alerts::alert::Alert,
        self_reporting::usage::UsageType,
        stream::{PartitioningDetails, StreamParams, StreamType},
    },
    metrics,
    utils::{flatten, json, schema::infer_json_schema, schema_ext::SchemaExt, time},
};
use datafusion::arrow::datatypes::Schema;
use infra::schema::{unwrap_partition_time_level, SchemaCache};

use super::get_exclude_labels;
use crate::{
    common::meta::{
        authz::Authz,
        ingestion::{IngestionResponse, StreamStatus},
        prom::{Metadata, HASH_LABEL, METADATA_LABEL, NAME_LABEL, TYPE_LABEL, VALUE_LABEL},
        stream::SchemaRecords,
    },
    service::{
        alerts::alert::AlertExt,
        db, format_stream_name,
        ingestion::{evaluate_trigger, get_write_partition_key, write_file, TriggerAlertData},
        pipeline::batch_execution::ExecutablePipeline,
        schema::check_for_schema,
        self_reporting::report_request_usage_stats,
    },
};

pub async fn ingest(org_id: &str, body: web::Bytes) -> Result<IngestionResponse> {
    let start = std::time::Instant::now();
    let started_at = chrono::Utc::now().timestamp_micros();

    if !LOCAL_NODE.is_ingester() {
        return Err(anyhow::anyhow!("not an ingester"));
    }

    if !db::file_list::BLOCKED_ORGS.is_empty()
        && db::file_list::BLOCKED_ORGS.contains(&org_id.to_string())
    {
        return Err(anyhow::anyhow!(
            "Quota exceeded for this organization [{}]",
            org_id
        ));
    }

    // check memtable
    if let Err(e) = ingester::check_memtable_size() {
        return Ok(IngestionResponse {
            code: http::StatusCode::SERVICE_UNAVAILABLE.into(),
            status: vec![],
            error: Some(e.to_string()),
        });
    }

    let mut stream_schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let mut stream_status_map: HashMap<String, StreamStatus> = HashMap::new();
    let mut stream_data_buf: HashMap<String, HashMap<String, SchemaRecords>> = HashMap::new();
    let mut stream_partitioning_map: HashMap<String, PartitioningDetails> = HashMap::new();

    // associated pipeline
    let mut stream_executable_pipelines: HashMap<String, Option<ExecutablePipeline>> =
        HashMap::new();
    let mut stream_pipeline_inputs: HashMap<String, Vec<(json::Value, String)>> = HashMap::new();

    // realtime alerts
    let mut stream_alerts_map: HashMap<String, Vec<Alert>> = HashMap::new();
    let mut stream_trigger_map: HashMap<String, Option<TriggerAlertData>> = HashMap::new();

    // records buffer
    let mut json_data_by_stream: HashMap<String, Vec<(json::Value, String)>> = HashMap::new();

    let reader: Vec<json::Value> = json::from_slice(&body)?;
    for record in reader.into_iter() {
        // JSON Flattening
        let mut record = flatten::flatten(record)?;
        // check data type
        let record = record.as_object_mut().unwrap();
        let stream_name = match record.get(NAME_LABEL).ok_or(anyhow!("missing __name__"))? {
            json::Value::String(s) => format_stream_name(s),
            _ => {
                return Err(anyhow::anyhow!("invalid __name__, need to be string"));
            }
        };
        let metrics_type = match record.get(TYPE_LABEL).ok_or(anyhow!("missing __type__"))? {
            json::Value::String(s) => s.clone(),
            _ => {
                return Err(anyhow::anyhow!("invalid __type__, need to be string"));
            }
        };

        // Start retrieve associated pipeline and initialize ExecutablePipeline
        if !stream_executable_pipelines.contains_key(&stream_name) {
            let exec_pl_option = crate::service::ingestion::get_stream_executable_pipeline(
                org_id,
                &stream_name,
                &StreamType::Metrics,
            )
            .await;
            stream_executable_pipelines.insert(stream_name.clone(), exec_pl_option);
        }
        // End pipeline params construction

        // check metrics type for Histogram & Summary
        if metrics_type.to_lowercase() == "histogram" || metrics_type.to_lowercase() == "summary" {
            if !stream_schema_map.contains_key(&stream_name) {
                let mut schema =
                    infra::schema::get(org_id, &stream_name, StreamType::Metrics).await?;
                if schema == Schema::empty() {
                    // create the metadata for the stream
                    let metadata = Metadata {
                        metric_family_name: stream_name.clone(),
                        metric_type: metrics_type.as_str().into(),
                        help: stream_name.clone().replace('_', " "),
                        unit: "".to_string(),
                    };
                    let mut extra_metadata: HashMap<String, String> = HashMap::new();
                    extra_metadata.insert(
                        METADATA_LABEL.to_string(),
                        json::to_string(&metadata).unwrap(),
                    );
                    schema = schema.with_metadata(extra_metadata);
                    db::schema::merge(
                        org_id,
                        &stream_name,
                        StreamType::Metrics,
                        &schema,
                        Some(chrono::Utc::now().timestamp_micros()),
                    )
                    .await?;
                }
                stream_schema_map.insert(stream_name.clone(), SchemaCache::new(schema));
            }
            continue;
        }

        let record = json::Value::Object(record.to_owned());

        // ready to be buffered for downstream processing
        if stream_executable_pipelines
            .get(&stream_name)
            .unwrap()
            .is_some()
        {
            // buffer to pipeline for batch processing
            stream_pipeline_inputs
                .entry(stream_name.to_owned())
                .or_default()
                .push((record, metrics_type));
        } else {
            // buffer to downstream processing directly
            json_data_by_stream
                .entry(stream_name.clone())
                .or_default()
                .push((record, metrics_type));
        }
    }

    // process records buffered for pipeline processing
    for (stream_name, exec_pl_option) in &stream_executable_pipelines {
        if let Some(exec_pl) = exec_pl_option {
            let Some(pipeline_inputs) = stream_pipeline_inputs.remove(stream_name) else {
                log::error!(
                    "[Ingestion]: Stream {} has pipeline, but inputs failed to be buffered. BUG",
                    stream_name
                );
                continue;
            };
            let (records, metric_types): (Vec<json::Value>, Vec<String>) =
                pipeline_inputs.into_iter().unzip();
            let count = records.len();
            match exec_pl.process_batch(org_id, records).await {
                Err(e) => {
                    let err_msg = format!(
                        "[Ingestion]: Stream {} pipeline batch processing failed: {}",
                        stream_name, e,
                    );
                    log::error!("{err_msg}");
                    // update status
                    let stream_status = stream_status_map
                        .entry(stream_name.clone())
                        .or_insert_with(|| StreamStatus::new(stream_name));
                    stream_status.status.failed += count as u32;
                    stream_status.status.error = err_msg;
                    continue;
                }
                Ok(pl_results) => {
                    for (stream_params, stream_pl_results) in pl_results {
                        if stream_params.stream_type != StreamType::Metrics {
                            continue;
                        }
                        // add partition keys
                        if !stream_partitioning_map.contains_key(stream_params.stream_name.as_str())
                        {
                            let partition_det =
                                crate::service::ingestion::get_stream_partition_keys(
                                    org_id,
                                    &StreamType::Metrics,
                                    &stream_params.stream_name,
                                )
                                .await;
                            stream_partitioning_map.insert(
                                stream_params.stream_name.to_string(),
                                partition_det.clone(),
                            );
                        }
                        for (idx, res) in stream_pl_results {
                            // buffer to downstream processing directly
                            json_data_by_stream
                                .entry(stream_params.stream_name.to_string())
                                .or_default()
                                .push((res, metric_types[idx].to_owned()));
                        }
                    }
                }
            }
        }
    }

    let cfg = config::get_config();
    for (stream_name, json_data) in json_data_by_stream {
        if !stream_partitioning_map.contains_key(&stream_name) {
            let partition_det = crate::service::ingestion::get_stream_partition_keys(
                org_id,
                &StreamType::Metrics,
                &stream_name,
            )
            .await;
            stream_partitioning_map.insert(stream_name.to_string(), partition_det);
        }
        // get partition key
        let partition_det = stream_partitioning_map.get(&stream_name).unwrap();
        let partition_keys = partition_det.partition_keys.clone();
        let partition_time_level =
            unwrap_partition_time_level(partition_det.partition_time_level, StreamType::Metrics);

        for (mut record, metric_type) in json_data {
            // Start get stream alerts
            if !stream_alerts_map.contains_key(&stream_name) {
                crate::service::ingestion::get_stream_alerts(
                    &[StreamParams {
                        org_id: org_id.to_owned().into(),
                        stream_name: stream_name.to_owned().into(),
                        stream_type: StreamType::Metrics,
                    }],
                    &mut stream_alerts_map,
                )
                .await;
            }
            // End get stream alert

            let record = record.as_object_mut().unwrap();

            // check timestamp & value
            let timestamp: i64 = match record.get(&cfg.common.column_timestamp) {
                None => chrono::Utc::now().timestamp_micros(),
                Some(json::Value::Number(s)) => {
                    time::parse_i64_to_timestamp_micros(s.as_f64().unwrap() as i64)
                }
                Some(_) => {
                    return Err(anyhow::anyhow!("invalid _timestamp, need to be number"));
                }
            };
            let value: f64 = match record.get(VALUE_LABEL).ok_or(anyhow!("missing value"))? {
                json::Value::Number(s) => s.as_f64().unwrap(),
                _ => {
                    return Err(anyhow::anyhow!("invalid value, need to be number"));
                }
            };
            // reset time & value
            record.insert(
                cfg.common.column_timestamp.clone(),
                json::Value::Number(timestamp.into()),
            );
            record.insert(
                VALUE_LABEL.to_string(),
                json::Number::from_f64(value).unwrap().into(),
            );
            // remove type from labels
            record.remove(TYPE_LABEL);
            // add hash
            let hash = super::signature_without_labels(record, &get_exclude_labels());
            record.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));

            // convert every label to string
            for (k, v) in record.iter_mut() {
                if k == NAME_LABEL
                    || k == TYPE_LABEL
                    || k == VALUE_LABEL
                    || k == &cfg.common.column_timestamp
                {
                    continue;
                }
                match v {
                    json::Value::String(_) => {}
                    _ => {
                        *v = json::Value::String(v.to_string());
                    }
                }
            }
            let record_str = json::to_string(&record).unwrap();

            // check schema
            if !stream_schema_map.contains_key(&stream_name) {
                let mut schema =
                    infra::schema::get(org_id, &stream_name, StreamType::Metrics).await?;
                if schema.fields().is_empty() {
                    let mut schema_reader = BufReader::new(record_str.as_bytes());
                    let inferred_schema =
                        infer_json_schema(&mut schema_reader, None, StreamType::Metrics).unwrap();
                    let metadata = Metadata {
                        metric_family_name: stream_name.clone(),
                        metric_type: metric_type.as_str().into(),
                        help: stream_name.clone().replace('_', " "),
                        unit: "".to_string(),
                    };
                    let mut extra_metadata: HashMap<String, String> = HashMap::new();
                    extra_metadata.insert(
                        METADATA_LABEL.to_string(),
                        json::to_string(&metadata).unwrap(),
                    );
                    schema = inferred_schema.with_metadata(extra_metadata);
                    db::schema::merge(
                        org_id,
                        &stream_name,
                        StreamType::Metrics,
                        &schema,
                        Some(timestamp),
                    )
                    .await?;
                    crate::common::utils::auth::set_ownership(
                        org_id,
                        &StreamType::Metrics.to_string(),
                        Authz::new(&stream_name),
                    )
                    .await;
                }
                stream_schema_map.insert(stream_name.clone(), SchemaCache::new(schema));
            }

            // check for schema evolution
            let _ = check_for_schema(
                org_id,
                &stream_name,
                StreamType::Metrics,
                &mut stream_schema_map,
                vec![record],
                timestamp,
            )
            .await;

            // write into buffer
            let schema = stream_schema_map
                .get(&stream_name)
                .unwrap()
                .schema()
                .as_ref()
                .clone()
                .with_metadata(HashMap::new());
            let schema_key = schema.hash_key();
            let hour_key = get_write_partition_key(
                timestamp,
                &partition_keys,
                partition_time_level,
                record,
                Some(&schema_key),
            );
            let stream_buf = stream_data_buf.entry(stream_name.to_string()).or_default();
            let hour_buf = stream_buf.entry(hour_key).or_insert_with(|| SchemaRecords {
                schema_key,
                schema: Arc::new(schema),
                records: vec![],
                records_size: 0,
            });
            hour_buf
                .records
                .push(Arc::new(json::Value::Object(record.to_owned())));
            hour_buf.records_size += record_str.len();

            // update status
            let stream_status = stream_status_map
                .entry(stream_name.clone())
                .or_insert_with(|| StreamStatus::new(&stream_name));
            stream_status.status.successful += 1;

            // realtime alert
            let need_trigger = !stream_trigger_map.contains_key(&stream_name);
            if need_trigger && !stream_alerts_map.is_empty() {
                // start check for alert trigger
                let key = format!("{}/{}/{}", org_id, StreamType::Metrics, stream_name);
                if let Some(alerts) = stream_alerts_map.get(&key) {
                    let mut trigger_alerts: TriggerAlertData = Vec::new();
                    let alert_end_time = chrono::Utc::now().timestamp_micros();
                    for alert in alerts {
                        if let Ok((Some(v), _)) =
                            alert.evaluate(Some(record), (None, alert_end_time)).await
                        {
                            trigger_alerts.push((alert.clone(), v));
                        }
                    }
                    stream_trigger_map.insert(stream_name.clone(), Some(trigger_alerts));
                }
            }
            // End check for alert trigger
        }
    }

    // write data to wal
    for (stream_name, stream_data) in stream_data_buf {
        // check if we are allowed to ingest
        if db::compact::retention::is_deleting_stream(
            org_id,
            StreamType::Metrics,
            &stream_name,
            None,
        ) {
            log::warn!("stream [{stream_name}] is being deleted");
            continue;
        }

        let writer =
            ingester::get_writer(0, org_id, &StreamType::Metrics.to_string(), &stream_name).await;
        // for performance issue, we will flush all when the app shutdown
        let fsync = false;
        let mut req_stats = write_file(&writer, &stream_name, stream_data, fsync).await;

        req_stats.response_time = start.elapsed().as_secs_f64();
        let fns_length: usize =
            stream_executable_pipelines
                .get(&stream_name)
                .map_or(0, |exec_pl_option| {
                    exec_pl_option
                        .as_ref()
                        .map_or(0, |exec_pl| exec_pl.num_of_func())
                });
        report_request_usage_stats(
            req_stats,
            org_id,
            &stream_name,
            StreamType::Metrics,
            UsageType::JsonMetrics,
            fns_length as _,
            started_at,
        )
        .await;
    }

    let time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/api/org/ingest/metrics/_json",
            "200",
            org_id,
            "",
            &StreamType::Metrics.to_string(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/api/org/ingest/metrics/_json",
            "200",
            org_id,
            "",
            &StreamType::Metrics.to_string(),
        ])
        .inc();

    // only one trigger per request, as it updates etcd
    for (_, entry) in stream_trigger_map {
        if let Some(entry) = entry {
            evaluate_trigger(entry).await;
        }
    }

    Ok(IngestionResponse::new(
        http::StatusCode::OK.into(),
        stream_status_map.values().map(|v| v.to_owned()).collect(),
    ))
}
