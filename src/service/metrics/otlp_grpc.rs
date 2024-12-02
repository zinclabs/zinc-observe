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

use actix_web::{http, HttpResponse};
use bytes::BytesMut;
use chrono::Utc;
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        alerts::alert,
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
    metrics::v1::{metric::Data, *},
};
use prost::Message;

use crate::{
    common::meta::{http::HttpResponse as MetaHttpResponse, prom::*, stream::SchemaRecords},
    service::{
        alerts::alert::AlertExt,
        db, format_stream_name,
        ingestion::{
            evaluate_trigger,
            grpc::{get_exemplar_val, get_metric_val, get_val},
            write_file, TriggerAlertData,
        },
        metrics::{format_label_name, get_exclude_labels},
        pipeline::batch_execution::ExecutablePipeline,
        schema::{check_for_schema, stream_schema_exists},
        self_reporting::report_request_usage_stats,
    },
};

pub async fn handle_grpc_request(
    org_id: &str,
    request: ExportMetricsServiceRequest,
    is_grpc: bool,
) -> Result<HttpResponse, anyhow::Error> {
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
    let mut stream_alerts_map: HashMap<String, Vec<alert::Alert>> = HashMap::new();
    let mut stream_trigger_map: HashMap<String, Option<TriggerAlertData>> = HashMap::new();

    let cfg = get_config();
    let mut response = ExportMetricsServiceResponse {
        partial_success: None,
    };

    // records buffer
    let mut json_data_by_stream: HashMap<String, Vec<json::Value>> = HashMap::new();

    for resource_metric in &request.resource_metrics {
        for scope_metric in &resource_metric.scope_metrics {
            for metric in &scope_metric.metrics {
                let metric_name = &format_stream_name(&metric.name);
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
                        .insert(metric_name.clone().to_owned(), partition_det.clone());
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
                if let Some(res) = &resource_metric.resource {
                    for item in &res.attributes {
                        rec[format_label_name(item.key.as_str())] = get_val(&item.value.as_ref());
                    }
                }
                if let Some(lib) = &scope_metric.scope {
                    rec["instrumentation_library_name"] =
                        serde_json::Value::String(lib.name.to_owned());
                    rec["instrumentation_library_version"] =
                        serde_json::Value::String(lib.version.to_owned());
                }
                rec[NAME_LABEL] = metric_name.to_owned().into();

                // metadata handling
                let mut metadata = Metadata {
                    metric_family_name: rec[NAME_LABEL].to_string(),
                    metric_type: MetricType::Unknown,
                    help: metric.description.to_owned(),
                    unit: metric.unit.to_owned(),
                };
                let mut prom_meta: HashMap<String, String> = HashMap::new();

                let records = match &metric.data {
                    Some(data) => match data {
                        Data::Gauge(gauge) => {
                            process_gauge(&mut rec, gauge, &mut metadata, &mut prom_meta)
                        }
                        Data::Sum(sum) => process_sum(&mut rec, sum, &mut metadata, &mut prom_meta),
                        Data::Histogram(hist) => {
                            process_histogram(&mut rec, hist, &mut metadata, &mut prom_meta)
                        }
                        Data::ExponentialHistogram(exp_hist) => process_exponential_histogram(
                            &mut rec,
                            exp_hist,
                            &mut metadata,
                            &mut prom_meta,
                        ),
                        Data::Summary(summary) => {
                            process_summary(&rec, summary, &mut metadata, &mut prom_meta)
                        }
                    },
                    None => vec![],
                };

                // update schema metadata
                if !schema_exists.has_metadata {
                    if let Err(e) =
                        update_setting(org_id, metric_name, StreamType::Metrics, prom_meta).await
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
                    rec = flatten::flatten(rec)?;

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
            // get json object
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

            let buf = metric_data_map
                .entry(local_metric_name.to_owned())
                .or_default();
            let schema = metric_schema_map
                .get(&local_metric_name)
                .unwrap()
                .schema()
                .as_ref()
                .clone()
                .with_metadata(HashMap::new());
            let schema_key = schema.hash_key();
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
                let key = format!(
                    "{}/{}/{}",
                    &org_id,
                    StreamType::Metrics,
                    local_metric_name.clone()
                );
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
        // for performance issue, we will flush all when the app shutdown
        // if let Err(e) = writer.sync().await {
        //     log::error!("ingestion error while syncing writer: {}", e);
        // }

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

    let ep = if is_grpc {
        "/grpc/otlp/metrics"
    } else {
        "/api/otlp/v1/metrics"
    };
    let time_took = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            ep,
            "200",
            org_id,
            "",
            StreamType::Metrics.to_string().as_str(),
        ])
        .observe(time_took);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            ep,
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

    let res = ExportMetricsServiceResponse {
        partial_success: None,
    };
    let mut out = BytesMut::with_capacity(res.encoded_len());
    res.encode(&mut out).expect("Out of memory");

    Ok(HttpResponse::Ok()
        .status(http::StatusCode::OK)
        .content_type("application/x-protobuf")
        .body(out))
}

fn process_gauge(
    rec: &mut json::Value,
    gauge: &Gauge,
    metadata: &mut Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    let mut records = vec![];

    // set metadata
    metadata.metric_type = MetricType::Gauge;
    prom_meta.insert(
        METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    for data_point in &gauge.data_points {
        process_data_point(rec, data_point);
        let val_map = rec.as_object_mut().unwrap();
        let hash = super::signature_without_labels(val_map, &get_exclude_labels());
        val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
        records.push(rec.clone());
    }
    records
}

fn process_sum(
    rec: &mut json::Value,
    sum: &Sum,
    metadata: &mut Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::Counter;
    prom_meta.insert(
        METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let mut records = vec![];
    process_aggregation_temporality(rec, sum.aggregation_temporality);
    rec["is_monotonic"] = sum.is_monotonic.to_string().into();
    for data_point in &sum.data_points {
        let mut dp_rec = rec.clone();
        process_data_point(&mut dp_rec, data_point);
        let val_map = dp_rec.as_object_mut().unwrap();
        let hash = super::signature_without_labels(val_map, &get_exclude_labels());
        val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
        records.push(dp_rec.clone());
    }
    records
}

fn process_histogram(
    rec: &mut json::Value,
    hist: &Histogram,
    metadata: &mut Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::Histogram;
    prom_meta.insert(
        METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let mut records = vec![];
    process_aggregation_temporality(rec, hist.aggregation_temporality);
    for data_point in &hist.data_points {
        let mut dp_rec = rec.clone();
        for mut bucket_rec in process_hist_data_point(&mut dp_rec, data_point) {
            let val_map = bucket_rec.as_object_mut().unwrap();
            let hash = super::signature_without_labels(val_map, &get_exclude_labels());
            val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
            records.push(bucket_rec);
        }
    }
    records
}

fn process_exponential_histogram(
    rec: &mut json::Value,
    hist: &ExponentialHistogram,
    metadata: &mut Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::ExponentialHistogram;
    prom_meta.insert(
        METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );
    let mut records = vec![];
    process_aggregation_temporality(rec, hist.aggregation_temporality);
    for data_point in &hist.data_points {
        let mut dp_rec = rec.clone();
        for mut bucket_rec in process_exp_hist_data_point(&mut dp_rec, data_point) {
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
    summary: &Summary,
    metadata: &mut Metadata,
    prom_meta: &mut HashMap<String, String>,
) -> Vec<serde_json::Value> {
    // set metadata
    metadata.metric_type = MetricType::Summary;
    prom_meta.insert(
        METADATA_LABEL.to_string(),
        json::to_string(&metadata).unwrap(),
    );

    let mut records = vec![];
    for data_point in &summary.data_points {
        let mut dp_rec = rec.clone();
        for mut bucket_rec in process_summary_data_point(&mut dp_rec, data_point) {
            let val_map = bucket_rec.as_object_mut().unwrap();
            let hash = super::signature_without_labels(val_map, &get_exclude_labels());
            val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
            records.push(bucket_rec);
        }
    }
    records
}

fn process_data_point(rec: &mut json::Value, data_point: &NumberDataPoint) {
    for attr in &data_point.attributes {
        rec[format_label_name(attr.key.as_str())] = get_val(&attr.value.as_ref());
    }
    rec[VALUE_LABEL] = get_metric_val(&data_point.value);
    rec[&get_config().common.column_timestamp] = (data_point.time_unix_nano / 1000).into();
    rec["start_time"] = data_point.start_time_unix_nano.to_string().into();
    rec["flag"] = if data_point.flags == 1 {
        DataPointFlags::NoRecordedValueMask.as_str_name()
    } else {
        DataPointFlags::DoNotUse.as_str_name()
    }
    .into();
    process_exemplars(rec, &data_point.exemplars);
}

fn process_hist_data_point(
    rec: &mut json::Value,
    data_point: &HistogramDataPoint,
) -> Vec<serde_json::Value> {
    let mut bucket_recs = vec![];

    for attr in &data_point.attributes {
        rec[format_label_name(attr.key.as_str())] = get_val(&attr.value.as_ref());
    }
    rec[&get_config().common.column_timestamp] = (data_point.time_unix_nano / 1000).into();
    rec["start_time"] = data_point.start_time_unix_nano.to_string().into();
    rec["flag"] = if data_point.flags == 1 {
        DataPointFlags::NoRecordedValueMask.as_str_name()
    } else {
        DataPointFlags::DoNotUse.as_str_name()
    }
    .into();
    process_exemplars(rec, &data_point.exemplars);
    // add count record
    let mut count_rec = rec.clone();
    count_rec[VALUE_LABEL] = (data_point.count as f64).into();
    count_rec[NAME_LABEL] = format!("{}_count", count_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(count_rec);

    // add sum record
    let mut sum_rec = rec.clone();
    sum_rec[VALUE_LABEL] = data_point.sum.into();
    sum_rec[NAME_LABEL] = format!("{}_sum", sum_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(sum_rec);

    // add min record
    let mut min_rec = rec.clone();
    min_rec[VALUE_LABEL] = data_point.min.into();
    min_rec[NAME_LABEL] = format!("{}_min", min_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(min_rec);

    // add max record
    let mut max_rec = rec.clone();
    max_rec[VALUE_LABEL] = data_point.max.into();
    max_rec[NAME_LABEL] = format!("{}_max", max_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(max_rec);

    // add bucket records
    let len = data_point.bucket_counts.len();
    for i in 0..len {
        let mut bucket_rec = rec.clone();
        bucket_rec[NAME_LABEL] = format!("{}_bucket", rec[NAME_LABEL].as_str().unwrap()).into();
        if let Some(val) = data_point.bucket_counts.get(i) {
            bucket_rec[VALUE_LABEL] = (*val as f64).into()
        }
        if let Some(val) = data_point.explicit_bounds.get(i) {
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
    data_point: &ExponentialHistogramDataPoint,
) -> Vec<serde_json::Value> {
    let mut bucket_recs = vec![];

    for attr in &data_point.attributes {
        rec[format_label_name(attr.key.as_str())] = get_val(&attr.value.as_ref());
    }
    rec[&get_config().common.column_timestamp] = (data_point.time_unix_nano / 1000).into();
    rec["start_time"] = data_point.start_time_unix_nano.to_string().into();
    rec["flag"] = if data_point.flags == 1 {
        DataPointFlags::NoRecordedValueMask.as_str_name()
    } else {
        DataPointFlags::DoNotUse.as_str_name()
    }
    .into();
    process_exemplars(rec, &data_point.exemplars);
    // add count record
    let mut count_rec = rec.clone();
    count_rec[VALUE_LABEL] = (data_point.count as f64).into();
    count_rec[NAME_LABEL] = format!("{}_count", count_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(count_rec);

    // add sum record
    let mut sum_rec = rec.clone();
    sum_rec[VALUE_LABEL] = data_point.sum.into();
    sum_rec[NAME_LABEL] = format!("{}_sum", sum_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(sum_rec);

    let base = 2 ^ (2 ^ -data_point.scale);
    // add negative bucket records
    if let Some(buckets) = &data_point.negative {
        let offset = buckets.offset;
        for (i, val) in buckets.bucket_counts.iter().enumerate() {
            let mut bucket_rec = rec.clone();
            bucket_rec[NAME_LABEL] = format!("{}_bucket", rec[NAME_LABEL].as_str().unwrap()).into();
            bucket_rec[VALUE_LABEL] = (*val as f64).into();
            bucket_rec["le"] = (base ^ (offset + (i as i32) + 1)).to_string().into();
            bucket_recs.push(bucket_rec);
        }
    }
    // add positive bucket records
    if let Some(buckets) = &data_point.positive {
        let offset = buckets.offset;
        for (i, val) in buckets.bucket_counts.iter().enumerate() {
            let mut bucket_rec = rec.clone();
            bucket_rec[NAME_LABEL] = format!("{}_bucket", rec[NAME_LABEL].as_str().unwrap()).into();
            bucket_rec[VALUE_LABEL] = (*val as f64).into();
            bucket_rec["le"] = (base ^ (offset + (i as i32) + 1)).to_string().into();
            bucket_recs.push(bucket_rec);
        }
    }

    bucket_recs
}

fn process_summary_data_point(
    rec: &mut json::Value,
    data_point: &SummaryDataPoint,
) -> Vec<serde_json::Value> {
    let mut bucket_recs = vec![];

    for attr in &data_point.attributes {
        rec[format_label_name(attr.key.as_str())] = get_val(&attr.value.as_ref());
    }
    rec[&get_config().common.column_timestamp] = (data_point.time_unix_nano / 1000).into();
    rec["start_time"] = data_point.start_time_unix_nano.to_string().into();
    rec["flag"] = if data_point.flags == 1 {
        DataPointFlags::NoRecordedValueMask.as_str_name()
    } else {
        DataPointFlags::DoNotUse.as_str_name()
    }
    .into();
    // add count record
    let mut count_rec = rec.clone();
    count_rec[VALUE_LABEL] = (data_point.count as f64).into();
    count_rec[NAME_LABEL] = format!("{}_count", count_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(count_rec);

    // add sum record
    let mut sum_rec = rec.clone();
    sum_rec[VALUE_LABEL] = data_point.sum.into();
    sum_rec[NAME_LABEL] = format!("{}_sum", sum_rec[NAME_LABEL].as_str().unwrap()).into();
    bucket_recs.push(sum_rec);

    // add bucket records
    for value in &data_point.quantile_values {
        let mut bucket_rec = rec.clone();
        bucket_rec[VALUE_LABEL] = value.value.into();
        bucket_rec["quantile"] = value.quantile.to_string().into();
        bucket_recs.push(bucket_rec);
    }
    bucket_recs
}

fn process_exemplars(rec: &mut json::Value, exemplars: &Vec<Exemplar>) {
    let mut exemplar_coll = vec![];
    for exemplar in exemplars {
        let mut exemplar_rec = json::json!({});
        for attr in &exemplar.filtered_attributes {
            exemplar_rec[attr.key.as_str()] = get_val(&attr.value.as_ref());
        }
        exemplar_rec[VALUE_LABEL] = get_exemplar_val(&exemplar.value);
        exemplar_rec[&get_config().common.column_timestamp] =
            (exemplar.time_unix_nano / 1000).into();

        match TraceId::from_bytes(exemplar.trace_id.as_slice().try_into().unwrap_or_default()) {
            TraceId::INVALID => {}
            _ => {
                exemplar_rec["trace_id"] =
                    TraceId::from_bytes(exemplar.trace_id.as_slice().try_into().unwrap())
                        .to_string()
                        .into();
            }
        };

        match SpanId::from_bytes(exemplar.span_id.as_slice().try_into().unwrap_or_default()) {
            SpanId::INVALID => {}
            _ => {
                exemplar_rec["span_id"] =
                    SpanId::from_bytes(exemplar.span_id.as_slice().try_into().unwrap())
                        .to_string()
                        .into();
            }
        };

        exemplar_coll.push(exemplar_rec)
    }
    rec["exemplars"] = exemplar_coll.into();
}

fn process_aggregation_temporality(rec: &mut json::Value, val: i32) {
    rec["aggregation_temporality"] = match val {
        0 => AggregationTemporality::Unspecified.as_str_name(),
        1 => AggregationTemporality::Delta.as_str_name(),
        2 => AggregationTemporality::Cumulative.as_str_name(),
        _ => AggregationTemporality::Unspecified.as_str_name(),
    }
    .into();
}
