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

use std::collections::{HashMap, HashSet};

use actix_web::{http, web, HttpResponse};
use anyhow::Result;
use bytes::BytesMut;
use chrono::{Duration, Utc};
use config::{
    get_config,
    meta::{
        stream::{StreamParams, StreamType},
        usage::UsageType,
    },
    metrics,
    utils::{flatten, json},
};
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_proto::tonic::collector::logs::v1::{
    ExportLogsPartialSuccess, ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use prost::Message;

use crate::{
    common::meta::{
        http::HttpResponse as MetaHttpResponse,
        ingestion::{IngestionStatus, StreamStatus},
    },
    handler::http::request::CONTENT_TYPE_JSON,
    service::{
        format_stream_name,
        ingestion::{check_ingestion_allowed, get_val_for_attr},
        pipeline::execution::PipelinedExt,
        schema::get_upto_discard_error,
    },
};

const SERVICE_NAME: &str = "service.name";
const SERVICE: &str = "service";

pub async fn logs_proto_handler(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
    user_email: &str,
) -> Result<HttpResponse> {
    let request = ExportLogsServiceRequest::decode(body).expect("Invalid protobuf");
    match super::otlp_grpc::handle_grpc_request(org_id, request, false, in_stream_name, user_email)
        .await
    {
        Ok(res) => Ok(res),
        Err(e) => {
            log::error!("error while handling request: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            )
        }
    }
}

// example at: https://opentelemetry.io/docs/specs/otel/protocol/file-exporter/#examples
// otel collector handling json request for logs https://github.com/open-telemetry/opentelemetry-collector/blob/main/pdata/plog/json.go
pub async fn logs_json_handler(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
    user_email: &str,
) -> Result<HttpResponse> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

    // check stream
    let stream_name = match in_stream_name {
        Some(name) => format_stream_name(name),
        None => "default".to_owned(),
    };
    check_ingestion_allowed(org_id, Some(&stream_name))?;

    let cfg = get_config();
    let min_ts = (Utc::now() - Duration::try_hours(cfg.limit.ingest_allowed_upto).unwrap())
        .timestamp_micros();

    let mut stream_params = vec![StreamParams::new(org_id, &stream_name, StreamType::Logs)];

    // Start retrieve associated pipeline and construct pipeline components
    let pipeline_params = crate::service::ingestion::get_stream_pipeline_params(
        org_id,
        &stream_name,
        &StreamType::Logs,
    )
    .await;
    // End pipeline construction

    if let Some((pl, node_map, graph)) = &pipeline_params {
        let pl_destinations = pl.get_all_destination_streams(node_map, graph);
        stream_params.extend(pl_destinations);
    }

    // Start get user defined schema
    let mut user_defined_schema_map: HashMap<String, HashSet<String>> = HashMap::new();
    crate::service::ingestion::get_user_defined_schema(
        &stream_params,
        &mut user_defined_schema_map,
    )
    .await;
    // End get user defined schema

    let mut stream_status = StreamStatus::new(&stream_name);
    let mut json_data_by_stream = HashMap::new();

    let body: json::Value = match json::from_slice(body.as_ref()) {
        Ok(v) => v,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Invalid json: {}", e),
            )));
        }
    };

    let logs = match body.get("resourceLogs") {
        Some(v) => match v.as_array() {
            Some(v) => v,
            None => {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    "Invalid json: the structure must be {{\"resourceLogs\":[]}}".to_string(),
                )));
            }
        },
        None => match body.get("resource_logs") {
            Some(v) => match v.as_array() {
                Some(v) => v,
                None => {
                    return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                        http::StatusCode::BAD_REQUEST.into(),
                        "Invalid json: the structure must be {{\"resource_logs\":[]}}".to_string(),
                    )));
                }
            },
            None => {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    "Invalid json: the structure must be {{\"resourceLogs\":[]}} or {{\"resource_logs\":[]}}".to_string(),
                )));
            }
        },
    };

    let mut res = ExportLogsServiceResponse {
        partial_success: None,
    };

    for res_log in logs.iter() {
        let mut service_att_map: json::Map<String, json::Value> = json::Map::new();
        if res_log.get("resource").is_some() {
            let resource = res_log.get("resource").unwrap().as_object().unwrap();
            if resource.get("attributes").is_some() {
                let attributes = resource.get("attributes").unwrap().as_array().unwrap();
                for res_attr in attributes {
                    let local_attr = res_attr.as_object().unwrap();
                    if local_attr
                        .get("key")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .eq(SERVICE_NAME)
                    {
                        let loc_service_name =
                            local_attr.get("value").unwrap().as_object().unwrap();
                        for item in loc_service_name {
                            service_att_map.insert(SERVICE_NAME.to_string(), item.1.clone());
                        }
                    } else {
                        service_att_map.insert(
                            format!(
                                "{}_{}",
                                SERVICE,
                                local_attr.get("key").unwrap().as_str().unwrap()
                            ),
                            get_val_for_attr(local_attr.get("value").unwrap()),
                        );
                    }
                }
            }
        }
        let scope_resources = res_log.get("scopeLogs");
        let inst_resources = if let Some(v) = scope_resources {
            v.as_array().unwrap()
        } else {
            res_log.get("scope_logs").unwrap().as_array().unwrap()
        };
        for inst_log in inst_resources {
            let log_records = if inst_log.get("logRecords").is_some() {
                inst_log.get("logRecords").unwrap().as_array().unwrap()
            } else {
                inst_log.get("log_records").unwrap().as_array().unwrap()
            };

            for log in log_records {
                let start_time: i64 = if log.get("timeUnixNano").is_some() {
                    json::get_int_value(log.get("timeUnixNano").unwrap())
                } else {
                    json::get_int_value(log.get("time_unix_nano").unwrap())
                };

                let timestamp = if start_time > 0 {
                    start_time / 1000
                } else {
                    Utc::now().timestamp_micros()
                };

                let mut value: json::Value = json::to_value(log).unwrap();

                // get json object
                let local_val = value.as_object_mut().unwrap();

                if log.get("attributes").is_some() {
                    let attributes = log.get("attributes").unwrap().as_array().unwrap();
                    for res_attr in attributes {
                        let local_attr = res_attr.as_object().unwrap();
                        let mut key = local_attr.get("key").unwrap().as_str().unwrap().to_string();
                        flatten::format_key(&mut key);
                        local_val.insert(key, get_val_for_attr(local_attr.get("value").unwrap()));
                    }
                }
                // remove attributes after adding
                local_val.remove("attributes");

                // remove body before adding
                local_val.remove("body_stringvalue");

                // process trace id
                if log.get("trace_id").is_some() {
                    local_val.remove("trace_id");
                    let trace_id = log.get("trace_id").unwrap();
                    let trace_id_str = trace_id.as_str().unwrap();
                    let trace_id_bytes = hex::decode(trace_id_str).unwrap().try_into().unwrap();
                    let trace_id = TraceId::from_bytes(trace_id_bytes).to_string();
                    local_val.insert("trace_id".to_owned(), trace_id.into());
                } else if log.get("traceId").is_some() {
                    local_val.remove("traceId");
                    let trace_id = log.get("traceId").unwrap();
                    let trace_id_str = trace_id.as_str().unwrap();
                    let trace_id_bytes = hex::decode(trace_id_str).unwrap().try_into().unwrap();
                    let trace_id = TraceId::from_bytes(trace_id_bytes).to_string();
                    local_val.insert("trace_id".to_owned(), trace_id.into());
                };

                // process span id
                if log.get("span_id").is_some() {
                    local_val.remove("span_id");
                    let span_id = log.get("span_id").unwrap();
                    let span_id_str = span_id.as_str().unwrap();
                    let span_id_bytes = hex::decode(span_id_str).unwrap().try_into().unwrap();
                    let span_id = SpanId::from_bytes(span_id_bytes).to_string();
                    local_val.insert("span_id".to_owned(), span_id.into());
                } else if log.get("spanId").is_some() {
                    local_val.remove("spanId");
                    let span_id = log.get("spanId").unwrap();
                    let span_id_str = span_id.as_str().unwrap();
                    let span_id_bytes = hex::decode(span_id_str).unwrap().try_into().unwrap();
                    let span_id = SpanId::from_bytes(span_id_bytes).to_string();
                    local_val.insert("span_id".to_owned(), span_id.into());
                };

                if log.get("body").is_some() {
                    let body = log.get("body").unwrap().get("stringValue").unwrap();
                    local_val.insert("body".to_owned(), body.clone());
                }

                // check ingestion time
                if timestamp < min_ts {
                    stream_status.status.failed += 1; // to old data, just discard
                    stream_status.status.error = get_upto_discard_error().to_string();
                    continue;
                }
                local_val.insert(
                    cfg.common.column_timestamp.clone(),
                    json::Value::Number(timestamp.into()),
                );

                local_val.append(&mut service_att_map.clone());

                value = json::to_value(local_val)?;

                if let Some((pipeline, pl_node_map, pl_graph)) = pipeline_params.as_ref() {
                    match pipeline.execute(value, pl_node_map, pl_graph) {
                        Err(e) => {
                            log::error!(
                                "[Pipeline] {}/{}/{}: Execution error: {}. Skip and ingest original",
                                pipeline.org,
                                pipeline.name,
                                pipeline.id,
                                e
                            );
                            stream_status.status.failed += 1; // pipeline failed or dropped
                            stream_status.status.error = format!("Pipeline execution error: {}", e);
                            continue;
                        }
                        Ok(pl_results) => {
                            for (stream_params, (mut value, is_flattened)) in pl_results {
                                if stream_params.stream_type != StreamType::Logs {
                                    continue;
                                }

                                if !is_flattened {
                                    // JSON Flattening
                                    value = flatten::flatten_with_level(
                                        value,
                                        cfg.limit.ingest_flatten_level,
                                    )
                                    .unwrap();
                                }

                                // get json object
                                let mut local_val = match value.take() {
                                    json::Value::Object(v) => v,
                                    _ => unreachable!(),
                                };

                                if let Some(fields) =
                                    user_defined_schema_map.get(stream_params.stream_name.as_str())
                                {
                                    local_val =
                                        crate::service::logs::refactor_map(local_val, fields);
                                }

                                let function_no = pipeline.num_of_func();
                                let (ts_data, fn_num) = json_data_by_stream
                                    .entry(stream_params.stream_name.to_string())
                                    .or_insert((Vec::new(), None));
                                ts_data.push((timestamp, local_val));
                                *fn_num = Some(function_no);
                            }
                        }
                    }
                } else {
                    // JSON Flattening
                    value =
                        flatten::flatten_with_level(value, cfg.limit.ingest_flatten_level).unwrap();

                    // get json object
                    let mut local_val = match value.take() {
                        json::Value::Object(v) => v,
                        _ => unreachable!(),
                    };

                    if let Some(fields) = user_defined_schema_map.get(&stream_name) {
                        local_val = crate::service::logs::refactor_map(local_val, fields);
                    }

                    let (ts_data, fn_num) = json_data_by_stream
                        .entry(stream_name.clone())
                        .or_insert((Vec::new(), None));
                    ts_data.push((timestamp, local_val));
                    *fn_num = Some(0); // no pl -> no func
                }
            }
        }
    }

    // Update partial success
    if stream_status.status.failed > 0 {
        res.partial_success = Some(ExportLogsPartialSuccess {
            rejected_log_records: stream_status.status.failed as i64,
            error_message: stream_status.status.error.clone(),
        });
    }

    // if no data, fast return
    if json_data_by_stream.is_empty() {
        let mut out = BytesMut::with_capacity(res.encoded_len());
        res.encode(&mut out).expect("Out of memory");
        return Ok(HttpResponse::Ok()
            .status(http::StatusCode::OK)
            .content_type(CONTENT_TYPE_JSON)
            .body(out)); // just return
    }

    let mut status = IngestionStatus::Record(stream_status.status);
    let (metric_rpt_status_code, response_body) = match super::write_logs_by_stream(
        org_id,
        user_email,
        (started_at, &start),
        UsageType::Logs,
        &mut status,
        json_data_by_stream,
    )
    .await
    {
        Ok(()) => {
            let mut out = BytesMut::with_capacity(res.encoded_len());
            res.encode(&mut out).expect("Out of memory");
            ("200", out)
        }
        Err(e) => {
            log::error!("Error while writing logs: {}", e);
            stream_status.status = match status {
                IngestionStatus::Record(status) => status,
                IngestionStatus::Bulk(_) => unreachable!(),
            };
            res.partial_success = Some(ExportLogsPartialSuccess {
                rejected_log_records: stream_status.status.failed as i64,
                error_message: stream_status.status.error,
            });
            let mut out = BytesMut::with_capacity(res.encoded_len());
            res.encode(&mut out).expect("Out of memory");
            ("500", out)
        }
    };

    // metric + data usage
    let took_time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/api/oltp/v1/logs",
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .observe(took_time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/api/oltp/v1/logs",
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .inc();

    // drop variables
    drop(pipeline_params);
    drop(user_defined_schema_map);

    return Ok(HttpResponse::Ok()
        .status(http::StatusCode::OK)
        .content_type(CONTENT_TYPE_JSON)
        .body(response_body));
}
