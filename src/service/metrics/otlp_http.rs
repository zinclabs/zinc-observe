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

use actix_web::{http, web, HttpResponse};
use bytes::BytesMut;
use chrono::Utc;
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        alerts::alert::Alert,
        self_reporting::usage::UsageType,
        stream::{PartitioningDetails, StreamParams, StreamType},
    },
    metrics,
    utils::{flatten, json, schema_ext::SchemaExt},
};
use hashbrown::HashSet;
use infra::schema::{unwrap_partition_time_level, update_setting, SchemaCache};
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_proto::tonic::{
    collector::metrics::v1::{
        ExportMetricsPartialSuccess, ExportMetricsServiceRequest, ExportMetricsServiceResponse,
    },
    metrics::v1::*,
};
use prost::Message;

use crate::{
    common::meta::{
        self,
        http::HttpResponse as MetaHttpResponse,
        prom::{self, MetricType, HASH_LABEL, NAME_LABEL, VALUE_LABEL},
        stream::SchemaRecords,
    },
    handler::http::request::CONTENT_TYPE_JSON,
    service::{
        alerts::alert::AlertExt,
        db, format_stream_name,
        ingestion::{evaluate_trigger, get_val_for_attr, write_file, TriggerAlertData},
        metrics::{format_label_name, get_exclude_labels, otlp_grpc::handle_grpc_request},
        pipeline::batch_execution::ExecutablePipeline,
        schema::{check_for_schema, stream_schema_exists},
        self_reporting::report_request_usage_stats,
    },
};

const SERVICE: &str = "service";

pub async fn metrics_proto_handler(
    org_id: &str,
    body: web::Bytes,
) -> Result<HttpResponse, std::io::Error> {
    let request = ExportMetricsServiceRequest::decode(body).expect("Invalid protobuf");
    match handle_grpc_request(org_id, request, false).await {
        Ok(res) => Ok(res),
        Err(e) => {
            log::error!("error processing request/{org_id}/metrics/otlp: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            )
        }
    }
}

pub async fn metrics_json_handler(
    org_id: &str,
    body: web::Bytes,
) -> Result<HttpResponse, std::io::Error> {
    if !LOCAL_NODE.is_ingester() {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                "not an ingester".to_string(),
            )),
        );
    }

    if !db::file_list::BLOCKED_ORGS.is_empty()
        && db::file_list::BLOCKED_ORGS.contains(&org_id.to_string())
    {
        return Ok(HttpResponse::Forbidden().json(MetaHttpResponse::error(
            http::StatusCode::FORBIDDEN.into(),
            format!("Quota exceeded for this organization [{}]", org_id),
        )));
    }

    // check memtable
    if let Err(e) = ingester::check_memtable_size() {
        return Ok(
            HttpResponse::ServiceUnavailable().json(MetaHttpResponse::error(
                http::StatusCode::SERVICE_UNAVAILABLE.into(),
                e.to_string(),
            )),
        );
    }

    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

    let mut metric_data_map: HashMap<String, HashMap<String, SchemaRecords>> = HashMap::new();
    let mut metric_schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let mut schema_evolved: HashMap<String, bool> = HashMap::new();
    let mut stream_partitioning_map: HashMap<String, PartitioningDetails> = HashMap::new();

    // associated pipeline
    let mut stream_executable_pipelines: HashMap<String, Option<ExecutablePipeline>> =
        HashMap::new();
    let mut stream_pipeline_inputs: HashMap<String, Vec<json::Value>> = HashMap::new();

    // realtime alerts
    let mut stream_alerts_map: HashMap<String, Vec<Alert>> = HashMap::new();
    let mut stream_trigger_map: HashMap<String, Option<TriggerAlertData>> = HashMap::new();

    let body: json::Value = match json::from_slice(body.as_ref()) {
        Ok(v) => v,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Invalid json: {}", e),
            )));
        }
    };

    let res_metrics = match body.get("resourceMetrics") {
        Some(v) => match v.as_array() {
            Some(v) => v,
            None => {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    "Invalid json: the structure must be {{\"resourceMetrics\":[]}}".to_string(),
                )));
            }
        },
        None => {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Invalid json: the structure must be {{\"resourceMetrics\":[]}}".to_string(),
            )));
        }
    };

    let cfg = get_config();
    let mut response = ExportMetricsServiceResponse {
        partial_success: None,
    };

    // records buffer
    let mut json_data_by_stream: HashMap<String, Vec<json::Value>> = HashMap::new();

    for res_metric in res_metrics.iter() {
        let mut service_att_map: json::Map<String, json::Value> = json::Map::new();
        if res_metric.get("resource").is_some() {
            let resource = res_metric.get("resource").unwrap().as_object().unwrap();
            if resource.get("attributes").is_some() {
                let attributes = resource.get("attributes").unwrap().as_array().unwrap();
                for res_attr in attributes {
                    let local_attr = res_attr.as_object().unwrap();

                    service_att_map.insert(
                        format!(
                            "{}_{}",
                            SERVICE,
                            format_label_name(local_attr.get("key").unwrap().as_str().unwrap())
                        ),
                        get_val_for_attr(local_attr.get("value").unwrap()),
                    );
                }
            }
        }
        let inst_resources = if res_metric.get("scopeMetrics").is_some() {
            res_metric.get("scopeMetrics").unwrap().as_array().unwrap()
        } else {
            res_metric.get("scope_metrics").unwrap().as_array().unwrap()
        };
        for inst_metrics in inst_resources {
            if inst_metrics.get("metrics").is_some() {
                let metrics = inst_metrics.get("metrics").unwrap().as_array().unwrap();
                for metric in metrics {
                    // parse metadata
                    let metric_name =
                        &format_stream_name(metric.get("name").unwrap().as_str().unwrap());

                    // check for schema
                    let schema_exists = stream_schema_exists(
                        org_id,
                        metric_name,
                        StreamType::Metrics,
                        &mut metric_schema_map,
                    )
                    .await;

                    // get partition keys
                    if !stream_partitioning_map.contains_key(metric_name) {
                        let partition_det = crate::service::ingestion::get_stream_partition_keys(
                            org_id,
                            &StreamType::Metrics,
                            metric_name,
                        )
                        .await;
                        stream_partitioning_map
                            .insert(metric_name.to_owned(), partition_det.clone());
                    }

                    // Start get stream alerts
                    crate::service::ingestion::get_stream_alerts(
                        &[StreamParams {
                            org_id: org_id.to_owned().into(),
                            stream_name: metric_name.to_owned().into(),
                            stream_type: StreamType::Metrics,
                        }],
                        &mut stream_alerts_map,
                    )
                    .await;
                    // End get stream alert

                    // get stream pipeline
                    if !stream_executable_pipelines.contains_key(metric_name) {
                        let pipeline_params =
                            crate::service::ingestion::get_stream_executable_pipeline(
                                org_id,
                                metric_name,
                                &StreamType::Metrics,
                            )
                            .await;
                        stream_executable_pipelines.insert(metric_name.clone(), pipeline_params);
                    }

                    let mut rec = json::json!({});

                    rec[NAME_LABEL] = metric_name.to_owned().into();

                    for (key, value) in &service_att_map {
                        rec[key] = value.clone();
                    }

                    if let Some(lib) = inst_metrics.get("scope") {
                        let lib = lib.as_object().unwrap();
                        if let Some(v) = lib.get("name") {
                            rec["instrumentation_library_name"] =
                                serde_json::Value::String(v.as_str().unwrap().to_string());
                        }
                        if let Some(v) = lib.get("version") {
                            rec["instrumentation_library_version"] =
                                serde_json::Value::String(v.as_str().unwrap().to_string());
                        }
                    };

                    let mut metadata = prom::Metadata {
                        metric_family_name: metric_name.to_owned(),
                        metric_type: MetricType::Unknown,
                        help: metric
                            .get("description")
                            .unwrap_or(&json::json!(""))
                            .as_str()
                            .unwrap()
                            .to_owned(),
                        unit: metric.get("unit").unwrap().as_str().unwrap().to_owned(),
                    };
                    let mut prom_meta: HashMap<String, String> = HashMap::new();

                    let records = if metric.get("sum").is_some() {
                        let sum = metric.get("sum").unwrap().as_object().unwrap();
                        process_sum(&mut rec, sum, &mut metadata, &mut prom_meta)
                    } else if metric.get("histogram").is_some() {
                        let histogram = metric.get("histogram").unwrap().as_object().unwrap();
                        process_histogram(&mut rec, histogram, &mut metadata, &mut prom_meta)
                    } else if metric.get("summary").is_some() {
                        let summary = metric.get("summary").unwrap().as_object().unwrap();
                        process_summary(&rec, summary, &mut metadata, &mut prom_meta)
                    } else if metric.get("gauge").is_some() {
                        let gauge = metric.get("gauge").unwrap().as_object().unwrap();
                        process_gauge(&mut rec, gauge, &mut metadata, &mut prom_meta)
                    } else if metric.get("exponentialHistogram").is_some() {
                        let exp = metric
                            .get("exponentialHistogram")
                            .unwrap()
                            .as_object()
                            .unwrap();
                        process_exponential_histogram(&mut rec, exp, &mut metadata, &mut prom_meta)
                    } else {
                        continue;
                    };

                    // update schema metadata
                    if !schema_exists.has_metadata {
                        if let Err(e) =
                            update_setting(org_id, metric_name, StreamType::Metrics, prom_meta)
                                .await
                        {
                            log::error!(
                                "Failed to set metadata for metric: {} with error: {}",
                                metric_name,
                                e
                            );
                        }
                    }

                    for mut rec in records {
                        // flattening
                        rec = flatten::flatten(rec).expect("failed to flatten");
                        // get json object

                        let local_metric_name =
                            &format_stream_name(rec.get(NAME_LABEL).unwrap().as_str().unwrap());

                        if local_metric_name != metric_name {
                            // check for schema
                            stream_schema_exists(
                                org_id,
                                local_metric_name,
                                StreamType::Metrics,
                                &mut metric_schema_map,
                            )
                            .await;

                            // get partition keys
                            if !stream_partitioning_map.contains_key(local_metric_name) {
                                let partition_det =
                                    crate::service::ingestion::get_stream_partition_keys(
                                        org_id,
                                        &StreamType::Metrics,
                                        local_metric_name,
                                    )
                                    .await;
                                stream_partitioning_map
                                    .insert(local_metric_name.to_owned(), partition_det.clone());
                            }

                            // Start get stream alerts
                            crate::service::ingestion::get_stream_alerts(
                                &[StreamParams {
                                    org_id: org_id.to_owned().into(),
                                    stream_name: local_metric_name.to_owned().into(),
                                    stream_type: StreamType::Metrics,
                                }],
                                &mut stream_alerts_map,
                            )
                            .await;
                            // End get stream alert

                            // get stream pipeline
                            if !stream_executable_pipelines.contains_key(local_metric_name) {
                                let pipeline_params =
                                    crate::service::ingestion::get_stream_executable_pipeline(
                                        org_id,
                                        local_metric_name,
                                        &StreamType::Metrics,
                                    )
                                    .await;
                                stream_executable_pipelines
                                    .insert(local_metric_name.clone(), pipeline_params);
                            }
                        }

                        // ready to be buffered for downstream processing
                        if stream_executable_pipelines
                            .get(local_metric_name)
                            .unwrap()
                            .is_some()
                        {
                            stream_pipeline_inputs
                                .entry(local_metric_name.to_string())
                                .or_default()
                                .push(rec);
                        } else {
                            json_data_by_stream
                                .entry(local_metric_name.to_string())
                                .or_default()
                                .push(rec);
                        }
                    }
                }
            }
        }
    }

    // process records buffered for pipeline processing
    for (stream_name, exec_pl_option) in &stream_executable_pipelines {
        if let Some(exec_pl) = exec_pl_option {
            let Some(pipeline_inputs) = stream_pipeline_inputs.remove(stream_name) else {
                let err_msg = format!(
                    "[Ingestion]: Stream {} has pipeline, but inputs failed to be buffered. BUG",
                    stream_name
                );
                log::error!("{err_msg}");
                let partial_resp =
                    response
                        .partial_success
                        .get_or_insert(ExportMetricsPartialSuccess {
                            rejected_data_points: 0,
                            error_message: String::new(),
                        });
                partial_resp.error_message = err_msg;
                continue;
            };
            let count = pipeline_inputs.len();
            match exec_pl.process_batch(org_id, pipeline_inputs).await {
                Err(e) => {
                    let err_msg = format!(
                        "[Ingestion]: Stream {} pipeline batch processing failed: {}",
                        stream_name, e,
                    );
                    log::error!("{err_msg}");
                    // update status
                    let partial_resp =
                        response
                            .partial_success
                            .get_or_insert(ExportMetricsPartialSuccess {
                                rejected_data_points: 0,
                                error_message: String::new(),
                            });
                    partial_resp.rejected_data_points += count as i64;
                    partial_resp.error_message = err_msg;
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
                        for (_, res) in stream_pl_results {
                            // buffer to downstream processing directly
                            json_data_by_stream
                                .entry(stream_params.stream_name.to_string())
                                .or_default()
                                .push(res);
                        }
                    }
                }
            }
        }
    }

    for (local_metric_name, json_data) in json_data_by_stream {
        // get partition keys
        let partition_det = stream_partitioning_map.get(&local_metric_name).unwrap();
        let partition_keys = partition_det.partition_keys.clone();
        let partition_time_level =
            unwrap_partition_time_level(partition_det.partition_time_level, StreamType::Metrics);

        for mut rec in json_data {
            let val_map: &mut serde_json::Map<String, serde_json::Value> =
                rec.as_object_mut().unwrap();

            let timestamp = val_map
                .get(&cfg.common.column_timestamp)
                .unwrap()
                .as_i64()
                .unwrap_or(Utc::now().timestamp_micros());

            let value_str = json::to_string(&val_map).unwrap();

            // check for schema evolution
            let schema_fields = match metric_schema_map.get(&local_metric_name) {
                Some(schema) => schema
                    .schema()
                    .fields()
                    .iter()
                    .map(|f| f.name())
                    .collect::<HashSet<_>>(),
                None => HashSet::default(),
            };
            let mut need_schema_check = !schema_evolved.contains_key(&local_metric_name);
            for key in val_map.keys() {
                if !schema_fields.contains(&key) {
                    need_schema_check = true;
                    break;
                }
            }
            drop(schema_fields);
            if need_schema_check
                && check_for_schema(
                    org_id,
                    &local_metric_name,
                    StreamType::Metrics,
                    &mut metric_schema_map,
                    vec![val_map],
                    timestamp,
                )
                .await
                .is_ok()
            {
                schema_evolved.insert(local_metric_name.to_owned(), true);
            }

            let schema = metric_schema_map
                .get(&local_metric_name)
                .unwrap()
                .schema()
                .as_ref()
                .clone()
                .with_metadata(HashMap::new());
            let schema_key = schema.hash_key();

            let buf = metric_data_map
                .entry(local_metric_name.to_owned())
                .or_default();
            // get hour key
            let hour_key = crate::service::ingestion::get_write_partition_key(
                timestamp,
                &partition_keys,
                partition_time_level,
                val_map,
                Some(&schema_key),
            );
            let hour_buf = buf.entry(hour_key).or_insert_with(|| SchemaRecords {
                schema_key,
                schema: Arc::new(schema),
                records: vec![],
                records_size: 0,
            });
            hour_buf
                .records
                .push(Arc::new(json::Value::Object(val_map.to_owned())));
            hour_buf.records_size += value_str.len();

            // real time alert
            let need_trigger = !stream_trigger_map.contains_key(&local_metric_name);
            if need_trigger && !stream_alerts_map.is_empty() {
                // Start check for alert trigger
                let key = format!("{}/{}/{}", &org_id, StreamType::Metrics, local_metric_name);
                if let Some(alerts) = stream_alerts_map.get(&key) {
                    let mut trigger_alerts: TriggerAlertData = Vec::new();
                    let alert_end_time = chrono::Utc::now().timestamp_micros();
                    for alert in alerts {
                        if let Ok((Some(v), _)) =
                            alert.evaluate(Some(val_map), (None, alert_end_time)).await
                        {
                            trigger_alerts.push((alert.clone(), v));
                        }
                    }
                    stream_trigger_map.insert(local_metric_name.clone(), Some(trigger_alerts));
                }
            }
            // End check for alert trigger
        }
    }

    // write data to wal
    for (stream_name, stream_data) in metric_data_map {
        // stream_data could be empty if metric value is nan, check it
        if stream_data.is_empty() {
            continue;
        }

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

        // write to file
        let writer =
            ingester::get_writer(0, org_id, &StreamType::Metrics.to_string(), &stream_name).await;
        let mut req_stats = write_file(&writer, &stream_name, stream_data).await;

        let fns_length: usize =
            stream_executable_pipelines
                .get(&stream_name)
                .map_or(0, |exec_pl_option| {
                    exec_pl_option
                        .as_ref()
                        .map_or(0, |exec_pl| exec_pl.num_of_func())
                });
        req_stats.response_time = start.elapsed().as_secs_f64();
        report_request_usage_stats(
            req_stats,
            org_id,
            &stream_name,
            StreamType::Metrics,
            UsageType::Metrics,
            fns_length as _,
            started_at,
        )
        .await;
    }

    let time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/api/otlp/v1/metrics",
            "200",
            org_id,
            "",
            StreamType::Metrics.to_string().as_str(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/api/otlp/v1/metrics",
            "200",
            org_id,
            "",
            StreamType::Metrics.to_string().as_str(),
        ])
        .inc();

    // only one trigger per request, as it updates etcd
    for (_, entry) in stream_trigger_map {
        if let Some(entry) = entry {
            evaluate_trigger(entry).await;
        }
    }

    let mut out = BytesMut::with_capacity(response.encoded_len());
    response.encode(&mut out).expect("Out of memory");

    Ok(HttpResponse::Ok()
        .status(http::StatusCode::OK)
        .content_type(CONTENT_TYPE_JSON)
        .body(out))
}

fn process_sum(
    rec: &mut json::Value,
    sum: &json::Map<String, json::Value>,
    metadata: &mut prom::Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::Counter;
    prom_meta.insert(
        meta::prom::METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let mut records = vec![];
    process_aggregation_temporality(
        rec,
        sum.get("aggregationTemporality").unwrap().as_u64().unwrap(),
    );
    rec["is_monotonic"] = if sum.get("isMonotonic").is_some() {
        sum.get("isMonotonic")
    } else {
        sum.get("is_monotonic")
    }
    .unwrap()
    .as_bool()
    .unwrap()
    .to_string()
    .into();

    let empty_dp = Vec::new();
    let dp = if sum.get("dataPoints").unwrap().as_array().is_some() {
        sum.get("dataPoints").unwrap().as_array().unwrap()
    } else if sum.get("data_points").unwrap().as_array().is_some() {
        sum.get("data_points").unwrap().as_array().unwrap()
    } else {
        &empty_dp
    };

    for data_point in dp {
        let dp = data_point.as_object().unwrap();
        let mut dp_rec = rec.clone();
        process_data_point(&mut dp_rec, dp);
        let val_map = dp_rec.as_object_mut().unwrap();
        let hash = super::signature_without_labels(val_map, &get_exclude_labels());
        val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
        records.push(dp_rec.clone());
    }
    records
}

fn process_histogram(
    rec: &mut json::Value,
    hist: &json::Map<String, json::Value>,
    metadata: &mut prom::Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::Histogram;
    prom_meta.insert(
        meta::prom::METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let mut records = vec![];
    process_aggregation_temporality(
        rec,
        hist.get("aggregationTemporality")
            .unwrap()
            .as_u64()
            .unwrap(),
    );

    let dp = get_data_points(hist);

    for data_point in dp.unwrap_or(&vec![]) {
        let dp = data_point.as_object().unwrap();
        let mut dp_rec = rec.clone();
        for mut bucket_rec in process_hist_data_point(&mut dp_rec, dp) {
            let val_map = bucket_rec.as_object_mut().unwrap();
            let hash = super::signature_without_labels(val_map, &get_exclude_labels());
            val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
            records.push(bucket_rec);
        }
    }
    records
}

fn process_summary(
    rec: &json::Value,
    summary: &json::Map<String, json::Value>,
    metadata: &mut prom::Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::Summary;
    prom_meta.insert(
        meta::prom::METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let mut records = vec![];

    let dp = get_data_points(summary);

    for data_point in dp.unwrap_or(&vec![]) {
        let dp = data_point.as_object().unwrap();
        let mut dp_rec = rec.clone();
        for mut bucket_rec in process_summary_data_point(&mut dp_rec, dp) {
            let val_map = bucket_rec.as_object_mut().unwrap();
            let hash = super::signature_without_labels(val_map, &get_exclude_labels());
            val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
            records.push(bucket_rec);
        }
    }
    records
}

fn process_gauge(
    rec: &mut json::Value,
    gauge: &json::Map<String, json::Value>,
    metadata: &mut prom::Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    let mut records = vec![];

    // set metadata
    metadata.metric_type = MetricType::Gauge;
    prom_meta.insert(
        meta::prom::METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let dp = get_data_points(gauge);

    for data_point in dp.unwrap_or(&vec![]) {
        let dp = data_point.as_object().unwrap();
        process_data_point(rec, dp);
        let val_map = rec.as_object_mut().unwrap();
        let hash = super::signature_without_labels(val_map, &get_exclude_labels());
        val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
        records.push(rec.clone());
    }
    records
}

fn process_exponential_histogram(
    rec: &mut json::Value,
    hist: &json::Map<String, json::Value>,
    metadata: &mut prom::Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::ExponentialHistogram;
    prom_meta.insert(
        meta::prom::METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );
    let mut records = vec![];
    process_aggregation_temporality(
        rec,
        hist.get("aggregationTemporality")
            .unwrap()
            .as_u64()
            .unwrap(),
    );

    let dp = get_data_points(hist);

    for data_point in dp.unwrap_or(&vec![]) {
        let mut dp_rec = rec.clone();
        let dp = data_point.as_object().unwrap();
        for mut bucket_rec in process_exp_hist_data_point(&mut dp_rec, dp) {
            let val_map = bucket_rec.as_object_mut().unwrap();
            let hash = super::signature_without_labels(val_map, &get_exclude_labels());
            val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
            records.push(bucket_rec);
        }
    }
    records
}

fn get_data_points(
    metric: &serde_json::Map<String, serde_json::Value>,
) -> Option<&Vec<serde_json::Value>> {
    if metric.get("dataPoints").unwrap().as_array().is_some() {
        metric.get("dataPoints").unwrap().as_array()
    } else if metric.get("data_points").unwrap().as_array().is_some() {
        metric.get("data_points").unwrap().as_array()
    } else {
        None
    }
}

fn process_data_point(rec: &mut json::Value, data_point: &json::Map<String, json::Value>) {
    for attr in data_point
        .get("attributes")
        .unwrap()
        .as_array()
        .unwrap_or(&vec![])
    {
        let attr = attr.as_object().unwrap();
        if let Some(v) = attr.get("value") {
            rec[format_label_name(attr.get("key").unwrap().as_str().unwrap())] = get_val_for_attr(v)
        }
    }
    let ts = json::get_int_value(data_point.get("timeUnixNano").unwrap());
    if data_point.get("startTimeUnixNano").is_some() {
        rec["start_time"] =
            json::get_string_value(data_point.get("startTimeUnixNano").unwrap()).into();
    }
    rec[&get_config().common.column_timestamp] = (ts / 1000).into();

    set_data_point_value(rec, data_point);

    if let Some(v) = data_point.get("flags") {
        rec["flag"] = if v.as_u64().unwrap() == 1 {
            DataPointFlags::NoRecordedValueMask.as_str_name()
        } else {
            DataPointFlags::DoNotUse.as_str_name()
        }
        .into();
    }

    process_exemplars(rec, data_point);
}

fn process_hist_data_point(
    rec: &mut json::Value,
    data_point: &json::Map<String, json::Value>,
) -> Vec<serde_json::Value> {
    let mut bucket_recs = vec![];

    for attr in data_point
        .get("attributes")
        .unwrap()
        .as_array()
        .unwrap_or(&vec![])
    {
        let attr = attr.as_object().unwrap();
        if let Some(v) = attr.get("value") {
            rec[format_label_name(attr.get("key").unwrap().as_str().unwrap())] =
                get_val_for_attr(v);
        }
    }
    let ts = json::get_int_value(data_point.get("timeUnixNano").unwrap());
    if data_point.get("startTimeUnixNano").is_some() {
        rec["start_time"] =
            json::get_string_value(data_point.get("startTimeUnixNano").unwrap()).into();
    }
    rec[&get_config().common.column_timestamp] = (ts / 1000).into();
    if let Some(v) = data_point.get("flags") {
        rec["flag"] = if v.as_u64().unwrap() == 1 {
            DataPointFlags::NoRecordedValueMask.as_str_name()
        } else {
            DataPointFlags::DoNotUse.as_str_name()
        }
        .into();
    }
    process_exemplars(rec, data_point);

    // add 4 types of record
    for stream in ["count", "sum", "min", "max"] {
        let mut new_rec = rec.clone();
        new_rec[VALUE_LABEL] = match data_point.get(stream) {
            Some(val) => json::get_float_value(val).into(),
            None => None::<f64>.into(),
        };
        new_rec[NAME_LABEL] = format!("{}_{stream}", new_rec[NAME_LABEL].as_str().unwrap()).into();
        bucket_recs.push(new_rec);
    }

    // add bucket records
    let buckets = data_point.get("bucketCounts").unwrap().as_array().unwrap();
    let explicit_bounds = data_point
        .get("explicitBounds")
        .unwrap()
        .as_array()
        .unwrap();
    let len = buckets.len();
    for i in 0..len {
        let mut bucket_rec = rec.clone();
        bucket_rec[NAME_LABEL] = format!("{}_bucket", rec[NAME_LABEL].as_str().unwrap()).into();
        if let Some(val) = buckets.get(i) {
            bucket_rec[VALUE_LABEL] = json::get_float_value(val).into();
        }
        if let Some(val) = explicit_bounds.get(i) {
            bucket_rec["le"] = (*val.to_string()).into()
        }
        if i == len - 1 {
            bucket_rec["le"] = f64::INFINITY.to_string().into();
        }
        bucket_recs.push(bucket_rec);
    }
    bucket_recs
}

fn process_exp_hist_data_point(
    rec: &mut json::Value,
    data_point: &json::Map<String, json::Value>,
) -> Vec<serde_json::Value> {
    let mut bucket_recs = vec![];

    for attr in data_point
        .get("attributes")
        .unwrap()
        .as_array()
        .unwrap_or(&vec![])
    {
        let attr = attr.as_object().unwrap();
        if let Some(v) = attr.get("value") {
            rec[format_label_name(attr.get("key").unwrap().as_str().unwrap())] =
                get_val_for_attr(v);
        }
    }
    let ts = json::get_int_value(data_point.get("timeUnixNano").unwrap());
    if data_point.get("startTimeUnixNano").is_some() {
        rec["start_time"] =
            json::get_string_value(data_point.get("startTimeUnixNano").unwrap()).into();
    }
    rec[&get_config().common.column_timestamp] = (ts / 1000).into();
    if let Some(v) = data_point.get("flags") {
        rec["flag"] = if v.as_u64().unwrap() == 1 {
            DataPointFlags::NoRecordedValueMask.as_str_name()
        } else {
            DataPointFlags::DoNotUse.as_str_name()
        }
        .into();
    }
    process_exemplars(rec, data_point);

    // add count record
    let mut count_rec = rec.clone();
    count_rec[VALUE_LABEL] = json::get_float_value(data_point.get("count").unwrap()).into();
    count_rec[NAME_LABEL] = format!("{}_count", count_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(count_rec);

    // add sum record
    let mut sum_rec = rec.clone();
    sum_rec[VALUE_LABEL] = json::get_float_value(data_point.get("sum").unwrap()).into();
    sum_rec[NAME_LABEL] = format!("{}_sum", sum_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(sum_rec);

    let base = 2 ^ (2 ^ -data_point.get("scale").unwrap().as_i64().unwrap());
    // add negative bucket records
    if let Some(buckets) = data_point.get("negative") {
        let buckets = buckets.as_object().unwrap();
        let offset = buckets.get("offset").unwrap().as_i64().unwrap();
        for (i, val) in buckets
            .get("bucket_counts")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let mut bucket_rec = rec.clone();
            bucket_rec[NAME_LABEL] = format!("{}_bucket", rec[NAME_LABEL].as_str().unwrap()).into();
            bucket_rec[VALUE_LABEL] = json::get_float_value(val).into();
            bucket_rec["le"] = (base ^ (offset + (i as i64) + 1)).to_string().into();
            bucket_recs.push(bucket_rec);
        }
    }
    // add positive bucket records
    if let Some(buckets) = data_point.get("positive") {
        let buckets = buckets.as_object().unwrap();
        let offset = buckets.get("offset").unwrap().as_i64().unwrap();
        for (i, val) in buckets
            .get("bucket_counts")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let mut bucket_rec = rec.clone();
            bucket_rec[VALUE_LABEL] = json::get_float_value(val).into();
            bucket_rec["le"] = (base ^ (offset + (i as i64) + 1)).to_string().into();
            bucket_recs.push(bucket_rec);
        }
    }

    bucket_recs
}

fn process_summary_data_point(
    rec: &mut json::Value,
    data_point: &json::Map<String, json::Value>,
) -> Vec<serde_json::Value> {
    let mut bucket_recs = vec![];

    for attr in data_point
        .get("attributes")
        .unwrap()
        .as_array()
        .unwrap_or(&vec![])
    {
        let attr = attr.as_object().unwrap();
        if let Some(v) = attr.get("value") {
            rec[format_label_name(attr.get("key").unwrap().as_str().unwrap())] =
                get_attribute_value(v).into();
        }
    }
    let ts = json::get_int_value(data_point.get("timeUnixNano").unwrap());
    if data_point.get("startTimeUnixNano").is_some() {
        rec["start_time"] =
            json::get_string_value(data_point.get("startTimeUnixNano").unwrap()).into();
    }
    rec[&get_config().common.column_timestamp] = (ts / 1000).into();
    if let Some(v) = data_point.get("flags") {
        rec["flag"] = if v.as_u64().unwrap() == 1 {
            DataPointFlags::NoRecordedValueMask.as_str_name()
        } else {
            DataPointFlags::DoNotUse.as_str_name()
        }
        .into();
    }
    // add count record
    let mut count_rec = rec.clone();
    count_rec[VALUE_LABEL] = json::get_float_value(data_point.get("count").unwrap()).into();
    count_rec[NAME_LABEL] = format!("{}_count", count_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(count_rec);

    // add sum record
    let mut sum_rec = rec.clone();
    sum_rec[VALUE_LABEL] = json::get_float_value(data_point.get("sum").unwrap()).into();
    sum_rec[NAME_LABEL] = format!("{}_sum", sum_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(sum_rec);

    // add bucket records
    let buckets = data_point
        .get("quantileValues")
        .unwrap()
        .as_array()
        .unwrap();
    for value in buckets {
        let mut bucket_rec = rec.clone();
        let value = value.as_object().unwrap();
        bucket_rec[VALUE_LABEL] = json::get_float_value(value.get("value").unwrap()).into();
        bucket_rec["quantile"] = value.get("quantile").unwrap().clone();
        bucket_recs.push(bucket_rec);
    }
    bucket_recs
}

fn process_aggregation_temporality(rec: &mut json::Value, val: u64) {
    rec["aggregation_temporality"] = match val {
        0 => AggregationTemporality::Unspecified.as_str_name(),
        1 => AggregationTemporality::Delta.as_str_name(),
        2 => AggregationTemporality::Cumulative.as_str_name(),
        _ => AggregationTemporality::Unspecified.as_str_name(),
    }
    .into();
}

fn process_exemplars(rec: &mut json::Value, data_point: &json::Map<String, json::Value>) {
    let exemplars = match data_point.get("exemplars") {
        Some(v) => match v.as_array() {
            Some(v) => v,
            None => {
                return;
            }
        },
        None => {
            return;
        }
    };
    let mut exemplar_coll = vec![];
    for exemplar in exemplars {
        let exemp = exemplar.as_object().unwrap();
        let mut exemplar_rec = json::json!({});
        for attr in exemp
            .get("filteredAttributes")
            .unwrap()
            .as_array()
            .unwrap_or(&vec![])
        {
            let attr = attr.as_object().unwrap();
            if let Some(v) = attr.get("value") {
                exemplar_rec[attr.get("key").unwrap().as_str().unwrap()] =
                    get_metric_value(v).into();
            }
        }
        set_data_point_value(&mut exemplar_rec, exemp);
        exemplar_rec[&get_config().common.column_timestamp] =
            (exemplar.get("timeUnixNano").unwrap().as_u64().unwrap() / 1000).into();

        let trace_id_bytes = hex::decode(exemplar.get("traceId").unwrap().as_str().unwrap())
            .expect("Failed to decode hex string");

        let mut trace_id_array = [0u8; 16];
        trace_id_array.copy_from_slice(&trace_id_bytes);

        match TraceId::from_bytes(trace_id_array) {
            TraceId::INVALID => {}
            _ => {
                exemplar_rec["trace_id"] = TraceId::from_bytes(trace_id_array).to_string().into();
            }
        };

        let span_id_bytes = hex::decode(exemplar.get("spanId").unwrap().as_str().unwrap())
            .expect("Failed to decode hex string");

        let mut span_id_array = [0u8; 8];
        span_id_array.copy_from_slice(&span_id_bytes);

        match SpanId::from_bytes(span_id_array) {
            SpanId::INVALID => {}
            _ => {
                exemplar_rec["span_id"] = SpanId::from_bytes(span_id_array).to_string().into();
            }
        };

        exemplar_coll.push(exemplar_rec)
    }
    rec["exemplars"] = exemplar_coll.into();
}

fn set_data_point_value(rec: &mut json::Value, data_point: &json::Map<String, json::Value>) {
    if data_point.get("asInt").is_some() {
        rec[VALUE_LABEL] = (data_point.get("asInt").unwrap().as_i64().unwrap() as f64).into();
    } else {
        rec[VALUE_LABEL] = json::get_float_value(data_point.get("asDouble").unwrap()).into();
    }
}

fn get_attribute_value(val: &json::Value) -> String {
    if val.get("stringValue").is_some() {
        val.get("stringValue").unwrap().to_string()
    } else if val.get("boolValue").is_some() {
        val.get("boolValue").unwrap().to_string()
    } else if val.get("intValue").is_some() {
        val.get("intValue").unwrap().to_string()
    } else if val.get("doubleValue").is_some() {
        val.get("doubleValue").unwrap().to_string()
    } else if val.get("arrayValue").is_some() {
        val.get("arrayValue").unwrap().to_string()
    } else if val.get("kvlistValue").is_some() {
        val.get("kvlistValue").unwrap().to_string()
    } else if val.get("bytesValue").is_some() {
        val.get("bytesValue").unwrap().to_string()
    } else {
        "".to_string()
    }
}

fn get_metric_value(val: &json::Value) -> f64 {
    if val.get("stringValue").is_some() {
        val.get("stringValue")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    } else if val.get("intValue").is_some() {
        val.get("intValue").unwrap().as_i64().unwrap() as f64
    } else if val.get("doubleValue").is_some() {
        val.get("doubleValue").unwrap().as_f64().unwrap()
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_process_hist_data_point() {
        let cases = vec![
            json!({
                "attributes": [
                    {"key": "attr1", "value": {"vk1": "vv1"}},
                    {"key": "attr2", "value": {"vk2": "vv2"}}
                ],
                "timeUnixNano": 1620000000000i64,
                "startTimeUnixNano": 1610000000000i64,
                "count": 10,
                "sum": 100,
                "min": 1,
                "max": 10,
                "bucketCounts": [1, 2, 3],
                "explicitBounds": [0.1, 0.2, 0.3]
            })
            .as_object()
            .unwrap()
            .clone(),
            json!({
                "attributes": [],
                "timeUnixNano": 1620000000000i64,
                "bucketCounts": [1],
                "explicitBounds": [0.1]
            })
            .as_object()
            .unwrap()
            .clone(),
            json!({
                "attributes": [],
                "timeUnixNano": 0,
                "bucketCounts": [],
                "explicitBounds": []
            })
            .as_object()
            .unwrap()
            .clone(),
        ];

        let result = cases
            .iter()
            .map(|dp| process_hist_data_point(&mut json!({"__name__": "test"}), dp))
            .collect::<Vec<_>>();
        // first case's 4 types record values
        assert_eq!(result[0][0]["value"], json!(10.0));
        assert_eq!(result[0][1]["value"], json!(100.0));
        assert_eq!(result[0][2]["value"], json!(1.0));
        assert_eq!(result[0][3]["value"], json!(10.0));

        let expected_lens = vec![7, 5, 4];
        assert_eq!(
            result.iter().map(Vec::len).collect::<Vec<_>>(),
            expected_lens
        );
    }
}
