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

use std::{collections::HashMap, io::Error, sync::Arc, time::Instant};

use actix_web::{http, web, HttpResponse};
use bytes::BytesMut;
use chrono::{Duration, Utc};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        alerts::alert::Alert,
        otlp::OtlpRequestType,
        self_reporting::usage::{RequestStats, UsageType},
        stream::{PartitionTimeLevel, StreamParams, StreamPartition, StreamType},
    },
    metrics,
    utils::{flatten, json, schema_ext::SchemaExt},
    DISTINCT_FIELDS,
};
use hashbrown::HashSet;
use infra::schema::{unwrap_partition_time_level, SchemaCache};
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_proto::tonic::{
    collector::trace::v1::{
        ExportTracePartialSuccess, ExportTraceServiceRequest, ExportTraceServiceResponse,
    },
    trace::v1::{status::StatusCode, Status},
};
use prost::Message;
use serde_json::Map;

use super::{
    logs::O2IngestJsonData, metadata::distinct_values::DISTINCT_STREAM_PREFIX,
    pipeline::batch_execution::ExecutablePipelineTraceInputs,
};
use crate::{
    common::meta::{
        http::HttpResponse as MetaHttpResponse,
        stream::SchemaRecords,
        traces::{Event, Span, SpanLink, SpanLinkContext, SpanRefType},
    },
    service::{
        alerts::alert::AlertExt,
        db, format_stream_name,
        ingestion::{evaluate_trigger, grpc::get_val, write_file, TriggerAlertData},
        metadata::{
            distinct_values::DvItem, trace_list_index::TraceListItem, write, MetadataItem,
            MetadataType,
        },
        schema::{check_for_schema, stream_schema_exists},
        self_reporting::report_request_usage_stats,
    },
};

const PARENT_SPAN_ID: &str = "reference.parent_span_id";
const PARENT_TRACE_ID: &str = "reference.parent_trace_id";
const REF_TYPE: &str = "reference.ref_type";
const SERVICE_NAME: &str = "service.name";
const SERVICE: &str = "service";
const BLOCK_FIELDS: [&str; 4] = ["_timestamp", "duration", "start_time", "end_time"];
// ref https://opentelemetry.io/docs/specs/otel/trace/api/#retrieving-the-traceid-and-spanid
const SPAN_ID_BYTES_COUNT: usize = 8;
const TRACE_ID_BYTES_COUNT: usize = 16;
const ATTR_STATUS_CODE: &str = "status_code";
const ATTR_STATUS_MESSAGE: &str = "status_message";

pub async fn otlp_proto(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
) -> Result<HttpResponse, Error> {
    let request = match ExportTraceServiceRequest::decode(body) {
        Ok(v) => v,
        Err(e) => {
            log::error!("[TRACES:OTLP] Invalid proto: {}", e);
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Invalid proto: {}", e),
            )));
        }
    };
    match handle_otlp_request(
        org_id,
        request,
        OtlpRequestType::HttpProtobuf,
        in_stream_name,
    )
    .await
    {
        Ok(v) => Ok(v),
        Err(e) => {
            log::error!(
                "[TRACES:OTLP] Error while handling grpc trace request: {}",
                e
            );
            Err(e)
        }
    }
}

pub async fn otlp_json(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
) -> Result<HttpResponse, Error> {
    let request = match serde_json::from_slice::<ExportTraceServiceRequest>(body.as_ref()) {
        Ok(req) => req,
        Err(e) => {
            log::error!("[TRACES:OTLP] Invalid json: {}", e);
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Invalid json: {}", e),
            )));
        }
    };
    match handle_otlp_request(org_id, request, OtlpRequestType::HttpJson, in_stream_name).await {
        Ok(v) => Ok(v),
        Err(e) => {
            log::error!(
                "[TRACES:OTLP] Error while handling http trace request: {}",
                e
            );
            Err(e)
        }
    }
}

pub async fn handle_otlp_request(
    org_id: &str,
    request: ExportTraceServiceRequest,
    req_type: OtlpRequestType,
    in_stream_name: Option<&str>,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

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
        log::error!(
            "[TRACES:OTLP] ingestion error while checking memtable size: {}",
            e
        );
        return Ok(
            HttpResponse::ServiceUnavailable().json(MetaHttpResponse::error(
                http::StatusCode::SERVICE_UNAVAILABLE.into(),
                e.to_string(),
            )),
        );
    }

    let cfg = get_config();
    let traces_stream_name = match in_stream_name {
        Some(name) => format_stream_name(name),
        None => "default".to_owned(),
    };
    let min_ts = (Utc::now()
        - Duration::try_hours(cfg.limit.ingest_allowed_upto)
            .expect("configuration error: too large ingest_allowed_upto"))
    .timestamp_micros();

    // Start retrieving associated pipeline and construct pipeline params
    let executable_pipeline = crate::service::ingestion::get_stream_executable_pipeline(
        org_id,
        &traces_stream_name,
        &StreamType::Traces,
    )
    .await;
    let mut stream_pipeline_inputs = ExecutablePipelineTraceInputs::new();
    // End pipeline params construction

    let mut service_name: String = traces_stream_name.to_string();
    let res_spans = request.resource_spans;
    let mut json_data_by_stream = HashMap::new();
    let mut span_metrics = Vec::with_capacity(res_spans.len());
    let mut partial_success = ExportTracePartialSuccess::default();
    for res_span in res_spans {
        let mut service_att_map: HashMap<String, json::Value> = HashMap::new();
        if let Some(resource) = res_span.resource {
            for res_attr in resource.attributes {
                if res_attr.key.eq(SERVICE_NAME) {
                    let loc_service_name = get_val(&res_attr.value.as_ref());
                    if let Some(name) = loc_service_name.as_str() {
                        service_name = name.to_string();
                        service_att_map.insert(res_attr.key, loc_service_name);
                    }
                } else {
                    service_att_map.insert(
                        format!("{}.{}", SERVICE, res_attr.key),
                        get_val(&res_attr.value.as_ref()),
                    );
                }
            }
        }
        let inst_resources = res_span.scope_spans;
        for inst_span in inst_resources {
            let spans = inst_span.spans;
            for span in spans {
                if span.trace_id.len() != TRACE_ID_BYTES_COUNT {
                    log::error!("[TRACES:OTLP] skipping span with invalid trace id");
                    partial_success.rejected_spans += 1;
                    continue;
                }
                let trace_id: String =
                    TraceId::from_bytes(span.trace_id.try_into().unwrap()).to_string();
                if span.span_id.len() != SPAN_ID_BYTES_COUNT {
                    log::error!(
                        "[TRACES:OTLP] skipping span with invalid span id, trace_id: {}",
                        trace_id
                    );
                    partial_success.rejected_spans += 1;
                    continue;
                }
                let span_id: String =
                    SpanId::from_bytes(span.span_id.try_into().unwrap()).to_string();
                let mut span_ref = HashMap::new();
                if !span.parent_span_id.is_empty()
                    && span.parent_span_id.len() == SPAN_ID_BYTES_COUNT
                {
                    span_ref.insert(PARENT_TRACE_ID.to_string(), trace_id.clone());
                    span_ref.insert(
                        PARENT_SPAN_ID.to_string(),
                        SpanId::from_bytes(span.parent_span_id.try_into().unwrap()).to_string(),
                    );
                    span_ref.insert(REF_TYPE.to_string(), format!("{:?}", SpanRefType::ChildOf));
                }
                let start_time: u64 = span.start_time_unix_nano;
                let end_time: u64 = span.end_time_unix_nano;
                let mut span_att_map: HashMap<String, json::Value> = HashMap::new();
                for span_att in span.attributes {
                    let mut key = span_att.key;
                    if BLOCK_FIELDS.contains(&key.as_str()) {
                        key = format!("attr_{}", key);
                    }
                    span_att_map.insert(key, get_val(&span_att.value.as_ref()));
                }

                // special addition for https://github.com/openobserve/openobserve/issues/4851
                // we set the status (error/non-error) properly, but skip the message
                // however, that can be useful when debugging with traces, so we
                // extract that as an attribute here.
                if let Some(ref status) = span.status {
                    span_att_map.insert(ATTR_STATUS_CODE.into(), status.code.into());
                    span_att_map.insert(ATTR_STATUS_MESSAGE.into(), status.message.clone().into());
                }

                let mut events = vec![];
                let mut event_att_map: HashMap<String, json::Value> = HashMap::new();
                for event in span.events {
                    for event_att in event.attributes {
                        event_att_map.insert(event_att.key, get_val(&event_att.value.as_ref()));
                    }
                    events.push(Event {
                        name: event.name,
                        _timestamp: event.time_unix_nano,
                        attributes: event_att_map.clone(),
                    })
                }

                let mut links = vec![];
                let mut link_att_map: HashMap<String, json::Value> = HashMap::new();
                for link in span.links {
                    for link_att in link.attributes {
                        link_att_map.insert(link_att.key, get_val(&link_att.value.as_ref()));
                    }
                    if link.span_id.len() != SPAN_ID_BYTES_COUNT {
                        log::error!(
                            "[TRACES:OTLP] skipping link with invalid span id, trace_id: {}",
                            trace_id
                        );
                        continue;
                    }
                    let span_id: String =
                        SpanId::from_bytes(link.span_id.try_into().unwrap()).to_string();
                    if link.trace_id.len() != TRACE_ID_BYTES_COUNT {
                        log::error!(
                            "[TRACES:OTLP] skipping link with invalid trace id, trace_id: {}",
                            trace_id
                        );
                        continue;
                    }
                    let trace_id: String =
                        TraceId::from_bytes(link.trace_id.try_into().unwrap()).to_string();
                    links.push(SpanLink {
                        context: SpanLinkContext {
                            span_id,
                            trace_id,
                            trace_flags: Some(link.flags),
                            trace_state: Some(link.trace_state),
                        },
                        attributes: link_att_map.clone(),
                        dropped_attributes_count: link.dropped_attributes_count,
                    })
                }

                let timestamp = (start_time / 1000) as i64;
                if timestamp < min_ts {
                    log::error!(
                        "[TRACES:OTLP] skipping span with timestamp older than allowed retention period, trace_id: {}",
                        trace_id
                    );
                    partial_success.rejected_spans += 1;
                    continue;
                }

                let local_val = Span {
                    trace_id: trace_id.clone(),
                    span_id,
                    span_kind: span.kind.to_string(),
                    span_status: get_span_status(span.status),
                    operation_name: span.name.clone(),
                    start_time,
                    end_time,
                    duration: (end_time - start_time) / 1000, // microseconds
                    reference: span_ref,
                    service_name: service_name.clone(),
                    attributes: span_att_map,
                    service: service_att_map.clone(),
                    flags: 1, // TODO add appropriate value
                    events: json::to_string(&events).unwrap(),
                    links: json::to_string(&links).unwrap(),
                };
                let span_status_for_spanmetric = local_val.span_status.clone();

                let mut value: json::Value = json::to_value(local_val).unwrap();

                if executable_pipeline.is_some() {
                    stream_pipeline_inputs.add_input(
                        value,
                        timestamp,
                        service_name.to_owned(),
                        span.name,
                        span_status_for_spanmetric,
                        span.kind.to_string(),
                        ((end_time - start_time) / 1_000_000) as f64,
                    );
                } else {
                    // JSON Flattening
                    value = flatten::flatten(value).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                    })?;

                    // get json object
                    let mut record_val = match value.take() {
                        json::Value::Object(mut v) => {
                            // build span metrics item
                            let sm = crate::job::metrics::TraceMetricsItem {
                                organization: org_id.to_string(),
                                traces_stream_name: traces_stream_name.clone(),
                                service_name: service_name.clone(),
                                span_name: v
                                    .remove("o2_span_metrics_name")
                                    .map_or(span.name.clone(), |name| {
                                        name.as_str().unwrap().to_string()
                                    }),
                                span_status: span_status_for_spanmetric,
                                span_kind: span.kind.to_string(),
                                duration: ((end_time - start_time) / 1_000_000) as f64, /* milliseconds */
                                span_id: v["span_id"].to_string(),
                            };
                            span_metrics.push(sm);
                            v
                        }
                        _ => {
                            log::error!(
                                "[TRACES:OTLP] stream did not receive a valid json object, trace_id: {}",
                                trace_id
                            );
                            return Ok(HttpResponse::InternalServerError().json(
                                MetaHttpResponse::error(
                                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                                    "stream did not receive a valid json objectt".into(),
                                ),
                            ));
                        }
                    };

                    // add timestamp
                    record_val.insert(
                        cfg.common.column_timestamp.clone(),
                        json::Value::Number(timestamp.into()),
                    );
                    let (ts_data, _) = json_data_by_stream
                        .entry(traces_stream_name.to_string())
                        .or_insert((Vec::new(), None));
                    ts_data.push((timestamp, record_val));
                }
            }
        }
    }

    // batch process records through pipeline
    if let Some(exec_pl) = &executable_pipeline {
        let (
            records,
            timestamps,
            services,
            span_names,
            span_status_for_spanmetrics,
            span_kinds,
            span_durations,
        ) = stream_pipeline_inputs.into_parts();
        let records_count = records.len();
        match exec_pl.process_batch(org_id, records).await {
            Err(e) => {
                log::error!(
                    "[TRACES:OTLP] pipeline({}/{}) batch execution error: {}.",
                    org_id,
                    traces_stream_name,
                    e
                );
                partial_success.rejected_spans += records_count as i64;
                partial_success.error_message = format!("Pipeline batch execution error: {}", e);
            }
            Ok(pl_results) => {
                for (stream_params, stream_pl_results) in pl_results {
                    if stream_params.stream_type != StreamType::Traces {
                        continue;
                    }

                    for (idx, mut res) in stream_pl_results {
                        // get json object
                        let mut record_val = match res.take() {
                            json::Value::Object(mut v) => {
                                // build span metrics item
                                let sm = crate::job::metrics::TraceMetricsItem {
                                    organization: org_id.to_string(),
                                    traces_stream_name: stream_params.stream_name.to_string(),
                                    service_name: services[idx].to_owned(),
                                    span_name: v
                                        .remove("o2_span_metrics_name")
                                        .map_or(span_names[idx].to_owned(), |name| {
                                            name.as_str().unwrap().to_string()
                                        }),
                                    span_status: span_status_for_spanmetrics[idx].to_owned(),
                                    span_kind: span_kinds[idx].to_owned(),
                                    duration: span_durations[idx], // milliseconds
                                    span_id: v["span_id"].to_string(),
                                };
                                span_metrics.push(sm);
                                v
                            }
                            _ => {
                                log::error!(
                                    "[TRACES:OTLP] stream did not receive a valid json object"
                                );
                                return Ok(HttpResponse::InternalServerError().json(
                                    MetaHttpResponse::error(
                                        http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                                        "stream did not receive a valid json object".into(),
                                    ),
                                ));
                            }
                        };

                        // add timestamp
                        let timestamp = timestamps[idx];
                        record_val.insert(
                            cfg.common.column_timestamp.clone(),
                            json::Value::Number(timestamp.into()),
                        );
                        let (ts_data, _) = json_data_by_stream
                            .entry(traces_stream_name.to_string())
                            .or_insert((Vec::new(), None));
                        ts_data.push((timestamp, record_val));
                    }
                }
            }
        }
    }

    // if no data, fast return
    if json_data_by_stream.is_empty() {
        return format_response(partial_success, req_type);
    }

    if let Err(e) = write_traces_by_stream(org_id, (started_at, &start), json_data_by_stream).await
    {
        log::error!("Error while writing traces: {}", e);
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                format!("error while writing trace data: {e}",),
            )),
        );
    }

    let time = start.elapsed().as_secs_f64();
    let ep = match req_type {
        OtlpRequestType::Grpc => "/grpc/otlp/traces",
        _ => "/api/otlp/v1/traces",
    };

    if cfg.common.traces_span_metrics_enabled {
        // record span metrics
        for m in span_metrics {
            // send to metrics job
            if let Err(e) = crate::job::metrics::TRACE_METRICS_CHAN.0.try_send(m) {
                log::error!("traces metrics item send to job fail: {e}")
            }
        }
    }

    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            ep,
            "200",
            org_id,
            &traces_stream_name,
            StreamType::Traces.to_string().as_str(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            ep,
            "200",
            org_id,
            &traces_stream_name,
            StreamType::Traces.to_string().as_str(),
        ])
        .inc();

    format_response(partial_success, req_type)
}

fn get_span_status(status: Option<Status>) -> String {
    match status {
        Some(v) => match v.code() {
            StatusCode::Ok => "OK".to_string(),
            StatusCode::Error => "ERROR".to_string(),
            StatusCode::Unset => "UNSET".to_string(),
        },
        // unset is the default status for span - https://opentelemetry.io/docs/languages/go/instrumentation/#set-span-status
        None => "UNSET".to_string(),
    }
}

fn format_response(
    mut partial_success: ExportTracePartialSuccess,
    req_type: OtlpRequestType,
) -> Result<HttpResponse, Error> {
    let partial = partial_success.rejected_spans > 0;

    let res = if partial {
        partial_success.error_message =
            "Some spans were rejected due to exceeding the allowed retention period".to_string();
        ExportTraceServiceResponse {
            partial_success: Some(partial_success),
        }
    } else {
        ExportTraceServiceResponse::default()
    };

    match req_type {
        OtlpRequestType::HttpJson => Ok(if partial {
            HttpResponse::PartialContent().json(res)
        } else {
            HttpResponse::Ok().json(res)
        }),
        _ => {
            let mut out = BytesMut::with_capacity(res.encoded_len());
            res.encode(&mut out).expect("Out of memory");
            Ok(HttpResponse::Ok()
                .status(http::StatusCode::OK)
                .content_type("application/x-protobuf")
                .body(out))
        }
    }
}

async fn write_traces_by_stream(
    org_id: &str,
    time_stats: (i64, &Instant),
    json_data_by_stream: HashMap<String, O2IngestJsonData>,
) -> Result<(), Error> {
    for (traces_stream_name, (json_data, fn_num)) in json_data_by_stream {
        let mut req_stats = match write_traces(org_id, &traces_stream_name, json_data).await {
            Ok(v) => v,
            Err(e) => {
                return Err(e);
            }
        };
        let time = time_stats.1.elapsed().as_secs_f64();
        req_stats.response_time = time;
        // metric + data usage
        report_request_usage_stats(
            req_stats,
            org_id,
            &traces_stream_name,
            StreamType::Traces,
            UsageType::Traces,
            fn_num.map_or(0, |cnt| cnt as u16),
            time_stats.0,
        )
        .await;
    }
    Ok(())
}

async fn write_traces(
    org_id: &str,
    stream_name: &str,
    json_data: Vec<(i64, json::Map<String, json::Value>)>,
) -> Result<RequestStats, Error> {
    let cfg = get_config();
    // get schema and stream settings
    let mut traces_schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let stream_schema = stream_schema_exists(
        org_id,
        stream_name,
        StreamType::Traces,
        &mut traces_schema_map,
    )
    .await;

    let stream_settings = infra::schema::get_settings(org_id, stream_name, StreamType::Traces)
        .await
        .unwrap_or_default();

    let mut partition_keys: Vec<StreamPartition> = vec![];
    let mut partition_time_level =
        PartitionTimeLevel::from(cfg.limit.traces_file_retention.as_str());
    if stream_schema.has_partition_keys {
        let partition_det = crate::service::ingestion::get_stream_partition_keys(
            org_id,
            &StreamType::Traces,
            stream_name,
        )
        .await;
        partition_keys = partition_det.partition_keys;
        partition_time_level =
            unwrap_partition_time_level(partition_det.partition_time_level, StreamType::Traces);
    }
    if partition_keys.is_empty() {
        partition_keys.push(StreamPartition::new("service_name"));
    }

    // Start get stream alerts
    let mut stream_alerts_map: HashMap<String, Vec<Alert>> = HashMap::new();
    crate::service::ingestion::get_stream_alerts(
        &[StreamParams {
            org_id: org_id.to_owned().into(),
            stream_name: stream_name.to_owned().into(),
            stream_type: StreamType::Traces,
        }],
        &mut stream_alerts_map,
    )
    .await;
    let cur_stream_alerts = stream_alerts_map.get(&format!(
        "{}/{}/{}",
        org_id,
        StreamType::Traces,
        stream_name
    ));
    let mut triggers: TriggerAlertData =
        Vec::with_capacity(cur_stream_alerts.map_or(0, |v| v.len()));
    let mut evaluated_alerts = HashSet::new();
    // End get stream alert

    // Start check for schema
    let min_timestamp = json_data.iter().map(|(ts, _)| ts).min().unwrap();
    let _ = check_for_schema(
        org_id,
        stream_name,
        StreamType::Traces,
        &mut traces_schema_map,
        json_data.iter().map(|(_, v)| v).collect(),
        *min_timestamp,
    )
    .await;
    let record_schema = traces_schema_map
        .get(stream_name)
        .unwrap()
        .schema()
        .as_ref()
        .clone()
        .with_metadata(HashMap::new());
    let record_schema = Arc::new(record_schema);
    let schema_key = record_schema.hash_key();

    let mut data_buf: HashMap<String, SchemaRecords> = HashMap::new();
    let mut distinct_values = Vec::with_capacity(16);
    let mut trace_index_values = Vec::with_capacity(json_data.len());

    // Start write data
    for (timestamp, record_val) in json_data {
        // get service_name
        let service_name = json::get_string_value(record_val.get("service_name").unwrap());
        // get distinct_value item
        let mut map = Map::new();
        for field in DISTINCT_FIELDS.iter().chain(
            stream_settings
                .distinct_value_fields
                .iter()
                .map(|f| &f.name),
        ) {
            if let Some(val) = record_val.get(field) {
                map.insert(field.clone(), val.clone());
            }
        }
        if !map.is_empty() {
            distinct_values.push(MetadataItem::DistinctValues(DvItem {
                stream_type: StreamType::Traces,
                stream_name: stream_name.to_string(),
                value: map,
            }));
        }

        // build trace metadata
        let trace_id = record_val
            .get("trace_id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        trace_index_values.push(MetadataItem::TraceListIndexer(TraceListItem {
            _timestamp: timestamp,
            stream_name: stream_name.to_string(),
            service_name: service_name.to_string(),
            trace_id,
        }));

        // Start check for alert trigger
        if let Some(alerts) = cur_stream_alerts {
            if triggers.len() < alerts.len() {
                let alert_end_time = chrono::Utc::now().timestamp_micros();
                for alert in alerts {
                    let key = format!(
                        "{}/{}/{}/{}",
                        org_id,
                        StreamType::Traces,
                        stream_name,
                        alert.name
                    );
                    // check if alert already evaluated
                    if evaluated_alerts.contains(&key) {
                        continue;
                    }
                    if let Ok((Some(v), _)) = alert
                        .evaluate(Some(&record_val), (None, alert_end_time))
                        .await
                    {
                        triggers.push((alert.clone(), v));
                        evaluated_alerts.insert(key);
                    }
                }
            }
        }
        // End check for alert trigger

        // get hour key
        let hour_key = super::ingestion::get_write_partition_key(
            timestamp,
            &partition_keys,
            partition_time_level,
            &record_val,
            Some(&schema_key),
        );

        let hour_buf = data_buf.entry(hour_key).or_insert_with(|| SchemaRecords {
            schema_key: schema_key.clone(),
            schema: record_schema.clone(),
            records: vec![],
            records_size: 0,
        });
        let record_val = json::Value::Object(record_val);
        let record_size = json::estimate_json_bytes(&record_val);
        hour_buf.records.push(Arc::new(record_val));
        hour_buf.records_size += record_size;
    }

    // write data to wal
    let writer =
        ingester::get_writer(0, org_id, &StreamType::Traces.to_string(), stream_name).await;
    let req_stats = write_file(
        &writer,
        stream_name,
        data_buf,
        !cfg.common.wal_fsync_disabled,
    )
    .await;

    // send distinct_values
    if !distinct_values.is_empty() && !stream_name.starts_with(DISTINCT_STREAM_PREFIX) {
        if let Err(e) = write(org_id, MetadataType::DistinctValues, distinct_values).await {
            log::error!("Error while writing distinct values: {}", e);
        }
    }

    // send trace metadata
    if !trace_index_values.is_empty() {
        if let Err(e) = write(org_id, MetadataType::TraceListIndexer, trace_index_values).await {
            log::error!("Error while writing trace_index values: {}", e);
        }
    }

    // only one trigger per request
    evaluate_trigger(triggers).await;

    Ok(req_stats)
}

#[cfg(test)]
mod tests {
    use config::utils::json::json;

    use crate::service::ingestion::grpc::get_val_for_attr;

    #[test]
    fn test_get_val_for_attr() {
        let in_val = 10.00;
        let input = json!({ "key": in_val });
        let resp = get_val_for_attr(input);
        assert_eq!(resp.as_str().unwrap(), in_val.to_string());
    }
}
