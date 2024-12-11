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

use actix_web::web;
use chrono::{TimeZone, Utc};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        alerts::alert,
        self_reporting::usage::UsageType,
        stream::{PartitioningDetails, StreamParams, StreamType},
    },
    metrics,
    utils::{json, schema_ext::SchemaExt, time::parse_i64_to_timestamp_micros},
    FxIndexMap,
};
use datafusion::arrow::datatypes::Schema;
use hashbrown::HashSet;
use infra::{
    cache::stats,
    errors::{Error, Result},
    schema::{unwrap_partition_time_level, update_setting, SchemaCache},
};
use promql_parser::{label::MatchOp, parser};
use prost::Message;
use proto::prometheus_rpc;

use crate::{
    common::{
        infra::config::{METRIC_CLUSTER_LEADER, METRIC_CLUSTER_MAP},
        meta::{prom::*, stream::SchemaRecords},
    },
    service::{
        alerts::alert::AlertExt,
        db, format_stream_name,
        ingestion::{evaluate_trigger, write_file, TriggerAlertData},
        metrics::format_label_name,
        pipeline::batch_execution::ExecutablePipeline,
        schema::{check_for_schema, stream_schema_exists},
        search as search_service,
        self_reporting::report_request_usage_stats,
    },
};

pub async fn remote_write(
    org_id: &str,
    body: web::Bytes,
) -> std::result::Result<(), anyhow::Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();
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
        return Err(anyhow::Error::msg(e.to_string()));
    }

    let cfg = get_config();
    let dedup_enabled = cfg.common.metrics_dedup_enabled;
    let election_interval = cfg.limit.metrics_leader_election_interval * 1000000;
    let mut last_received: i64 = 0;
    let mut has_entry = false;
    let mut accept_record: bool;
    let mut cluster_name = String::new();
    let mut metric_data_map: HashMap<String, HashMap<String, SchemaRecords>> = HashMap::new();
    let mut metric_schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let mut schema_evolved: HashMap<String, bool> = HashMap::new();
    let mut stream_partitioning_map: HashMap<String, PartitioningDetails> = HashMap::new();

    // associated pipeline
    let mut stream_executable_pipelines: HashMap<String, Option<ExecutablePipeline>> =
        HashMap::new();
    let mut stream_pipeline_inputs: HashMap<String, Vec<(json::Value, i64)>> = HashMap::new();

    // realtime alerts
    let mut stream_alerts_map: HashMap<String, Vec<alert::Alert>> = HashMap::new();
    let mut stream_trigger_map: HashMap<String, Option<TriggerAlertData>> = HashMap::new();

    let decoded = snap::raw::Decoder::new()
        .decompress_vec(&body)
        .map_err(|e| anyhow::anyhow!("Invalid snappy compressed data: {}", e.to_string()))?;
    let request = prometheus_rpc::WriteRequest::decode(bytes::Bytes::from(decoded))
        .map_err(|e| anyhow::anyhow!("Invalid protobuf: {}", e.to_string()))?;

    // records buffer
    let mut json_data_by_stream: HashMap<String, Vec<(json::Value, i64)>> = HashMap::new();

    // parse metadata
    for item in request.metadata {
        let metric_name = format_stream_name(&item.metric_family_name.clone());
        let metadata = Metadata {
            metric_family_name: item.metric_family_name.clone(),
            metric_type: item.r#type().into(),
            help: item.help.clone(),
            unit: item.unit.clone(),
        };
        let mut extra_metadata: HashMap<String, String> = HashMap::new();
        extra_metadata.insert(
            METADATA_LABEL.to_string(),
            json::to_string(&metadata).unwrap(),
        );
        let stream_schema = infra::schema::get(org_id, &metric_name, StreamType::Metrics)
            .await
            .unwrap_or(Schema::empty());
        let schema_metadata = stream_schema.metadata();
        // check if need to update metadata
        let mut need_update = false;
        for (k, v) in extra_metadata.iter() {
            if schema_metadata.contains_key(k) && schema_metadata.get(k).unwrap() == v {
                continue;
            }
            need_update = true;
            break;
        }
        if need_update {
            if let Err(e) =
                update_setting(org_id, &metric_name, StreamType::Metrics, extra_metadata).await
            {
                log::error!(
                    "Error updating metadata for stream: {}, err: {}",
                    metric_name,
                    e
                );
            }
        }
    }

    // maybe empty, we can return immediately
    if request.timeseries.is_empty() {
        let time = start.elapsed().as_secs_f64();
        metrics::HTTP_RESPONSE_TIME
            .with_label_values(&[
                "/prometheus/api/v1/write",
                "200",
                org_id,
                "",
                &StreamType::Metrics.to_string(),
            ])
            .observe(time);
        metrics::HTTP_INCOMING_REQUESTS
            .with_label_values(&[
                "/prometheus/api/v1/write",
                "200",
                org_id,
                "",
                &StreamType::Metrics.to_string(),
            ])
            .inc();
        return Ok(());
    }

    // parse timeseries
    let mut first_line = true;
    for mut event in request.timeseries {
        // get labels
        let mut replica_label = String::new();

        let labels: FxIndexMap<String, String> = event
            .labels
            .drain(..)
            .filter(|label| {
                if label.name == cfg.prom.ha_replica_label {
                    if !has_entry {
                        replica_label = label.value.clone();
                    }
                    false
                } else if label.name == cfg.prom.ha_cluster_label {
                    if !has_entry && cluster_name.is_empty() {
                        cluster_name = format!("{}/{}", org_id, label.value.clone());
                    }
                    false
                } else {
                    true
                }
            })
            .map(|label| (format_label_name(&label.name), label.value))
            .collect();

        let metric_name = match labels.get(NAME_LABEL) {
            Some(v) => v.to_owned(),
            None => continue,
        };

        // parse samples
        for sample in event.samples {
            let mut sample_val = sample.value;
            // revisit in future
            if sample_val.is_infinite() {
                if sample_val == f64::INFINITY || sample_val > f64::MAX {
                    sample_val = f64::MAX;
                } else if sample_val == f64::NEG_INFINITY || sample_val < f64::MIN {
                    sample_val = f64::MIN;
                }
            } else if sample_val.is_nan() {
                // skip the entry from adding to store
                continue;
            }
            let metric = Metric {
                labels: &labels,
                value: sample_val,
            };

            if first_line && dedup_enabled && !cluster_name.is_empty() {
                let lock = METRIC_CLUSTER_LEADER.read().await;
                match lock.get(&cluster_name) {
                    Some(leader) => {
                        last_received = leader.last_received;
                        has_entry = true;
                    }
                    None => {
                        has_entry = false;
                    }
                }
                drop(lock);
                accept_record = if !replica_label.is_empty() {
                    prom_ha_handler(
                        has_entry,
                        &cluster_name,
                        &replica_label,
                        last_received,
                        election_interval,
                    )
                    .await
                } else {
                    true
                };
                has_entry = true;
                first_line = false;
            } else {
                accept_record = true
            }
            if !accept_record {
                // do not accept any entries for request
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/prometheus/api/v1/write",
                        "200",
                        org_id,
                        "",
                        &StreamType::Metrics.to_string(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/prometheus/api/v1/write",
                        "200",
                        org_id,
                        "",
                        &StreamType::Metrics.to_string(),
                    ])
                    .inc();
                return Ok(());
            }

            // check for schema
            let _schema_exists = stream_schema_exists(
                org_id,
                &metric_name,
                StreamType::Metrics,
                &mut metric_schema_map,
            )
            .await;

            // get partition keys
            if !stream_partitioning_map.contains_key(&metric_name) {
                let partition_det = crate::service::ingestion::get_stream_partition_keys(
                    org_id,
                    &StreamType::Metrics,
                    &metric_name,
                )
                .await;
                stream_partitioning_map.insert(metric_name.clone(), partition_det.clone());
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
            if !stream_executable_pipelines.contains_key(&metric_name) {
                let pipeline_params = crate::service::ingestion::get_stream_executable_pipeline(
                    org_id,
                    &metric_name,
                    &StreamType::Metrics,
                )
                .await;
                stream_executable_pipelines.insert(metric_name.clone(), pipeline_params);
            }

            let value: json::Value = json::to_value(&metric).unwrap();
            let timestamp = parse_i64_to_timestamp_micros(sample.timestamp);

            // ready to be buffered for downstream processing
            if stream_executable_pipelines
                .get(&metric_name)
                .unwrap()
                .is_some()
            {
                // buffer to pipeline for batch processing
                stream_pipeline_inputs
                    .entry(metric_name.to_owned())
                    .or_default()
                    .push((value, timestamp));
            } else {
                // buffer to downstream processing directly
                json_data_by_stream
                    .entry(metric_name.clone())
                    .or_default()
                    .push((value, timestamp));
            }
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
            let (records, timestamps): (Vec<json::Value>, Vec<i64>) =
                pipeline_inputs.into_iter().unzip();
            match exec_pl.process_batch(org_id, records).await {
                Err(e) => {
                    log::error!(
                        "[Ingestion]: Stream {} pipeline batch processing failed: {}",
                        stream_name,
                        e,
                    );
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
                                .push((res, timestamps[idx]));
                        }
                    }
                }
            }
        }
    }

    for (stream_name, json_data) in json_data_by_stream {
        // get partition keys
        let partition_det = stream_partitioning_map.get(&stream_name).unwrap();
        let partition_keys = partition_det.partition_keys.clone();
        let partition_time_level =
            unwrap_partition_time_level(partition_det.partition_time_level, StreamType::Metrics);

        for (mut value, timestamp) in json_data {
            let val_map = value.as_object_mut().unwrap();
            let hash = super::signature_without_labels(val_map, &[VALUE_LABEL]);
            val_map.insert(HASH_LABEL.to_string(), json::Value::String(hash.into()));
            val_map.insert(
                cfg.common.column_timestamp.clone(),
                json::Value::Number(timestamp.into()),
            );
            let value_str = config::utils::json::to_string(&val_map).unwrap();

            // check for schema evolution
            let schema_fields = match metric_schema_map.get(&stream_name) {
                Some(schema) => schema
                    .schema()
                    .fields()
                    .iter()
                    .map(|f| f.name())
                    .collect::<HashSet<_>>(),
                None => HashSet::default(),
            };
            let mut need_schema_check = !schema_evolved.contains_key(&stream_name);
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
                    &stream_name,
                    StreamType::Metrics,
                    &mut metric_schema_map,
                    vec![val_map],
                    timestamp,
                )
                .await
                .is_ok()
            {
                schema_evolved.insert(stream_name.clone(), true);
            }

            let schema = metric_schema_map
                .get(&stream_name)
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
            let buf = metric_data_map.entry(stream_name.to_owned()).or_default();
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
            let need_trigger = !stream_trigger_map.contains_key(&stream_name);
            if need_trigger && !stream_alerts_map.is_empty() {
                // Start check for alert trigger
                let key = format!(
                    "{}/{}/{}",
                    &org_id,
                    StreamType::Metrics,
                    stream_name.clone()
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
                    stream_trigger_map.insert(stream_name.clone(), Some(trigger_alerts));
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
        // for performance issue, we will flush all when the app shutdown
        let fsync = false;
        let mut req_stats = write_file(&writer, &stream_name, stream_data, fsync).await;

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
            UsageType::PrometheusRemoteWrite,
            fns_length as u16,
            started_at,
        )
        .await;
    }

    let time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/prometheus/api/v1/write",
            "200",
            org_id,
            "",
            &StreamType::Metrics.to_string(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/prometheus/api/v1/write",
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

    Ok(())
}

pub(crate) async fn get_metadata(org_id: &str, req: RequestMetadata) -> Result<ResponseMetadata> {
    if req.limit == Some(0) {
        return Ok(hashbrown::HashMap::new());
    }

    let stream_type = StreamType::Metrics;

    if let Some(metric_name) = req.metric {
        let schema = infra::schema::get(org_id, &metric_name, stream_type)
            .await
            // `db::schema::get` never fails, so it's safe to unwrap
            .unwrap();
        let mut resp = hashbrown::HashMap::new();
        if schema != Schema::empty() {
            resp.insert(
                metric_name,
                get_metadata_object(&schema).map_or_else(Vec::new, |obj| vec![obj]),
            );
        };
        return Ok(resp);
    }

    match db::schema::list(org_id, Some(stream_type), true).await {
        Err(error) => {
            tracing::error!(%stream_type, ?error, "failed to get metrics' stream schemas");
            Err(Error::Message(format!(
                "failed to get metrics' stream schemas: {error}"
            )))
        }
        Ok(mut stream_schemas) => {
            stream_schemas.sort_by(|a, b| a.stream_name.cmp(&b.stream_name));
            let histogram_summary = stream_schemas
                .iter()
                .filter_map(|v| match super::get_prom_metadata_from_schema(&v.schema) {
                    None => None,
                    Some(v) => {
                        if v.metric_type == MetricType::Histogram
                            || v.metric_type == MetricType::Summary
                        {
                            Some(v.metric_family_name)
                        } else {
                            None
                        }
                    }
                })
                .collect::<Vec<_>>();
            let mut histogram_summary_sub = Vec::with_capacity(histogram_summary.len() * 3);
            for name in histogram_summary.iter() {
                histogram_summary_sub.push(format!("{}_bucket", name));
                histogram_summary_sub.push(format!("{}_count", name));
                histogram_summary_sub.push(format!("{}_sum", name));
            }
            let metric_names = stream_schemas.into_iter().filter_map(|schema| {
                if histogram_summary_sub.contains(&schema.stream_name) {
                    None
                } else {
                    get_metadata_object(&schema.schema).map(|meta| (schema.stream_name, vec![meta]))
                }
            });
            Ok(match req.limit {
                None => metric_names.collect(),
                Some(limit) => metric_names.take(limit).collect(),
            })
        }
    }
}

// HACK: the implementation returns at most one metadata object per metric.
// This differs from Prometheus, which [supports] multiple metadata objects per
// metric.
//
// [supports]: https://prometheus.io/docs/prometheus/latest/querying/api/#querying-metric-metadata
fn get_metadata_object(schema: &Schema) -> Option<MetadataObject> {
    schema.metadata.get(METADATA_LABEL).map(|s| {
        serde_json::from_str::<Metadata>(s)
            .unwrap_or_else(|error| {
                tracing::error!(%error, input = ?s, "failed to parse metadata");
                panic!("BUG: failed to parse {METADATA_LABEL}")
            })
            .into()
    })
}

pub(crate) async fn get_series(
    org_id: &str,
    selector: Option<parser::VectorSelector>,
    start: i64,
    end: i64,
) -> Result<Vec<serde_json::Value>> {
    let metric_name = match selector.as_ref().and_then(try_into_metric_name) {
        Some(name) => name,
        None => {
            // HACK: in the ideal world we would have queried all the metric streams
            return Ok(vec![]);
        }
    };

    let schema = infra::schema::get(org_id, &metric_name, StreamType::Metrics)
        .await
        // `db::schema::get` never fails, so it's safe to unwrap
        .unwrap();

    let cfg = get_config();
    // Comma-separated list of label names
    let label_names = schema
        .fields()
        .iter()
        .map(|f| f.name().as_str())
        .filter(|&s| s != cfg.common.column_timestamp && s != VALUE_LABEL && s != HASH_LABEL)
        .collect::<Vec<_>>()
        .join("\", \"");
    if label_names.is_empty() {
        return Ok(vec![]);
    }

    let mut sql = format!("SELECT DISTINCT({HASH_LABEL}), \"{label_names}\" FROM {metric_name}");
    let mut sql_where = Vec::new();
    if let Some(selector) = selector {
        for mat in selector.matchers.matchers.iter() {
            if mat.name == cfg.common.column_timestamp
                || mat.name == VALUE_LABEL
                || schema.field_with_name(&mat.name).is_err()
            {
                continue;
            }
            match &mat.op {
                MatchOp::Equal => {
                    sql_where.push(format!("{} = '{}'", mat.name, mat.value));
                }
                MatchOp::NotEqual => {
                    sql_where.push(format!("{} != '{}'", mat.name, mat.value));
                }
                MatchOp::Re(_re) => {
                    sql_where.push(format!("re_match({}, '{}')", mat.name, mat.value));
                }
                MatchOp::NotRe(_re) => {
                    sql_where.push(format!("re_not_match({}, '{}')", mat.name, mat.value));
                }
            }
        }
        if !sql_where.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&sql_where.join(" AND "));
        }
    }

    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql,
            from: 0,
            size: 1000,
            start_time: start,
            end_time: end,
            ..Default::default()
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions: vec![],
        clusters: vec![],
        timeout: 0,
        search_type: None,
        search_event_context: None,
    };
    let series = match search_service::search("", org_id, StreamType::Metrics, None, &req).await {
        Err(err) => {
            log::error!("search series error: {err}");
            return Err(err);
        }
        Ok(resp) => resp
            .hits
            .into_iter()
            .map(|mut val| {
                if let Some(map) = val.as_object_mut() {
                    map.remove(HASH_LABEL);
                }
                val
            })
            .collect(),
    };
    Ok(series)
}

pub(crate) async fn get_labels(
    org_id: &str,
    selector: Option<parser::VectorSelector>,
    start: i64,
    end: i64,
) -> Result<Vec<String>> {
    let opt_metric_name = selector.as_ref().and_then(try_into_metric_name);
    let stream_schemas = match db::schema::list(org_id, Some(StreamType::Metrics), true).await {
        Err(_) => return Ok(vec![]),
        Ok(schemas) => schemas,
    };
    let mut label_names = hashbrown::HashSet::new();
    let cfg = get_config();
    for schema in stream_schemas {
        if let Some(ref metric_name) = opt_metric_name {
            if *metric_name != schema.stream_name {
                // Client has requested a particular metric name, but this stream is
                // not it.
                continue;
            }
        }
        let stats = stats::get_stream_stats(org_id, &schema.stream_name, StreamType::Metrics);
        if stats.time_range_intersects(start, end) {
            let field_names = schema
                .schema
                .fields()
                .iter()
                .map(|f| f.name())
                .filter(|&s| {
                    s != &cfg.common.column_timestamp && s != VALUE_LABEL && s != HASH_LABEL
                })
                .cloned();
            label_names.extend(field_names);
        }
    }
    let mut label_names = label_names.into_iter().collect::<Vec<_>>();
    label_names.sort();
    Ok(label_names)
}

// XXX-TODO: filter the results in accordance with `selector.matchers`
pub(crate) async fn get_label_values(
    org_id: &str,
    label_name: String,
    selector: Option<parser::VectorSelector>,
    start: i64,
    end: i64,
) -> Result<Vec<String>> {
    let opt_metric_name = selector.as_ref().and_then(try_into_metric_name);
    let stream_type = StreamType::Metrics;

    if label_name == NAME_LABEL {
        // This special case doesn't require any SQL to be executed. All we have
        // to do is to collect stream names that satisfy selection criteria
        // (i.e., `selector` and `start`/`end`) and return them.
        let stream_schemas = db::schema::list(org_id, Some(stream_type), true)
            .await
            .unwrap_or_default();
        let mut label_values = Vec::with_capacity(stream_schemas.len());
        for schema in stream_schemas {
            if let Some(ref metric_name) = opt_metric_name {
                if *metric_name != schema.stream_name {
                    // Client has requested a particular metric name, but this stream is
                    // not it.
                    continue;
                }
            }
            let stats = match super::get_prom_metadata_from_schema(&schema.schema) {
                None => stats::get_stream_stats(org_id, &schema.stream_name, stream_type),
                Some(metadata) => {
                    if metadata.metric_type == MetricType::Histogram
                        || metadata.metric_type == MetricType::Summary
                    {
                        stats::get_stream_stats(
                            org_id,
                            &format!("{}_sum", schema.stream_name),
                            stream_type,
                        )
                    } else {
                        stats::get_stream_stats(org_id, &schema.stream_name, stream_type)
                    }
                }
            };
            if stats.time_range_intersects(start, end) {
                label_values.push(schema.stream_name)
            }
        }
        label_values.sort();
        return Ok(label_values);
    }

    let metric_name = match opt_metric_name {
        Some(name) => name,
        None => {
            // HACK: in the ideal world we would have queried all the metric streams
            // and collected label names from them.
            return Ok(vec![]);
        }
    };

    let schema = infra::schema::get(org_id, &metric_name, stream_type)
        .await
        // `db::schema::get` never fails, so it's safe to unwrap
        .unwrap();
    if schema.fields().is_empty() {
        return Ok(vec![]);
    }
    if schema.field_with_name(&label_name).is_err() {
        return Ok(vec![]);
    }
    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql: format!("SELECT DISTINCT({label_name}) FROM {metric_name}"),
            from: 0,
            size: 1000,
            start_time: start,
            end_time: end,
            ..Default::default()
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions: vec![],
        clusters: vec![],
        timeout: 0,
        search_type: None,
        search_event_context: None,
    };
    let mut label_values = match search_service::search("", org_id, stream_type, None, &req).await {
        Ok(resp) => resp
            .hits
            .iter()
            .filter_map(|v| v.as_object().unwrap().get(&label_name))
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>(),
        Err(err) => {
            log::error!("search values error: {:?}", err);
            return Err(err);
        }
    };
    label_values.sort();
    label_values.dedup();
    Ok(label_values)
}

pub(crate) fn try_into_metric_name(selector: &parser::VectorSelector) -> Option<String> {
    match &selector.name {
        Some(name) => {
            // `match[]` argument contains a metric name, e.g.
            // `match[]=zo_response_code{method="GET"}`
            Some(name.clone())
        }
        None => {
            // `match[]` argument does not contain a metric name.
            // Check if there is `__name__` among the matchers,
            // e.g. `match[]={__name__="zo_response_code",method="GET"}`
            selector
                .matchers
                .find_matchers(NAME_LABEL)
                .first()
                .map(|m| m.value.clone())
        }
    }
}

async fn prom_ha_handler(
    has_entry: bool,
    cluster_name: &str,
    replica_label: &str,
    last_received: i64,
    election_interval: i64,
) -> bool {
    let mut _accept_record = false;
    let curr_ts = Utc::now().timestamp_micros();
    if !has_entry {
        METRIC_CLUSTER_MAP
            .write()
            .await
            .insert(cluster_name.to_owned(), vec![]);
        log::info!("Making {} leader for {} ", replica_label, cluster_name);
        METRIC_CLUSTER_LEADER.write().await.insert(
            cluster_name.to_owned(),
            ClusterLeader {
                name: replica_label.to_owned(),
                last_received: curr_ts,
                updated_by: LOCAL_NODE.uuid.clone(),
            },
        );
        _accept_record = true;
    } else {
        let mut lock = METRIC_CLUSTER_LEADER.write().await;
        let leader = lock.get_mut(cluster_name).unwrap();
        if replica_label.eq(&leader.name) {
            _accept_record = true;
            leader.last_received = curr_ts;
            // log::info!(  "Updating last received data for {} to {}",
            // &leader.name, Utc.timestamp_nanos(last_received * 1000));
        } else if curr_ts - last_received > election_interval {
            // elect new leader as didnt receive data for last 30 secs
            log::info!(
                "Electing {} new leader for {} as last received data from {} at {} ",
                replica_label,
                cluster_name,
                &leader.name,
                Utc.timestamp_nanos(last_received * 1000)
            );
            leader.name = replica_label.to_owned();
            leader.last_received = curr_ts;
            _accept_record = true;
        } else {
            // log::info!(
            //     "Rejecting entry from {}  as leader is {}",
            //     replica_label,
            //     &leader.name,
            // );
            _accept_record = false;
        }
    }

    let mut lock = METRIC_CLUSTER_MAP.write().await;
    let replica_list = lock.entry(cluster_name.to_owned()).or_default();
    let replica_list_db = if !replica_list.contains(&replica_label.to_owned()) {
        replica_list.push(replica_label.to_owned());
        replica_list.clone()
    } else {
        vec![]
    };
    drop(lock);

    if !replica_list_db.is_empty() {
        let _ = db::metrics::set_prom_cluster_info(cluster_name, &replica_list_db).await;
    }

    _accept_record
}
