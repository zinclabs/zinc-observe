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
use bytes::BytesMut;
use chrono::{Duration, Utc};
use config::{
    cluster,
    meta::{stream::StreamType, usage::UsageType},
    metrics,
    utils::{flatten, json},
};
use infra::schema::SchemaCache;
use opentelemetry::trace::{SpanId, TraceId};
use opentelemetry_proto::tonic::collector::logs::v1::{
    ExportLogsPartialSuccess, ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use prost::Message;

use crate::{
    common::meta::{
        alerts::Alert,
        http::HttpResponse as MetaHttpResponse,
        ingestion::{IngestionStatus, StreamStatus},
        stream::StreamParams,
    },
    handler::http::request::CONTENT_TYPE_JSON,
    service::{
        db, get_formatted_stream_name,
        ingestion::get_val_for_attr,
        schema::{get_upto_discard_error, stream_schema_exists},
        usage::report_request_usage_stats,
    },
};

const SERVICE_NAME: &str = "service.name";
const SERVICE: &str = "service";

pub async fn logs_proto_handler(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
    user_email: &str,
) -> Result<HttpResponse, std::io::Error> {
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
) -> Result<HttpResponse, std::io::Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

    // Start check stream
    let mut stream_schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let stream_name = match in_stream_name {
        Some(name) => {
            get_formatted_stream_name(
                &mut StreamParams::new(org_id, name, StreamType::Logs),
                &mut stream_schema_map,
            )
            .await
        }
        None => {
            let _schema_exists =
                stream_schema_exists(org_id, "default", StreamType::Logs, &mut stream_schema_map)
                    .await;
            "default".to_owned()
        }
    };
    let stream_name = &stream_name;

    if !cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) {
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
    // End check stream

    // check memtable
    if let Err(e) = ingester::check_memtable_size() {
        return Ok(
            HttpResponse::ServiceUnavailable().json(MetaHttpResponse::error(
                http::StatusCode::SERVICE_UNAVAILABLE.into(),
                e.to_string(),
            )),
        );
    }

    let cfg = config::get_config();
    let min_ts = (Utc::now() - Duration::try_hours(cfg.limit.ingest_allowed_upto).unwrap())
        .timestamp_micros();

    // Start Register Transforms for stream
    let mut runtime = crate::service::ingestion::init_functions_runtime();
    let (local_trans, stream_vrl_map) = crate::service::ingestion::register_stream_functions(
        org_id,
        &StreamType::Logs,
        stream_name,
    );
    // End Register Transforms for stream

    let stream_param = StreamParams {
        org_id: org_id.to_owned().into(),
        stream_name: stream_name.to_owned().into(),
        stream_type: StreamType::Logs,
    };

    // Start get stream alerts
    let mut stream_alerts_map: HashMap<String, Vec<Alert>> = HashMap::new();
    crate::service::ingestion::get_stream_alerts(&[stream_param.clone()], &mut stream_alerts_map)
        .await;
    // End get stream alert

    // Start get user defined schema
    let mut user_defined_schema_map: HashMap<String, HashSet<String>> = HashMap::new();
    crate::service::ingestion::get_user_defined_schema(
        &[stream_param],
        &mut user_defined_schema_map,
    )
    .await;
    // End get user defined schema

    // Start get partition details
    let partition_det = crate::service::ingestion::get_stream_partition_keys(
        org_id,
        &StreamType::Logs,
        stream_name,
    )
    .await;
    let partition_keys = partition_det.partition_keys;
    let partition_time_level = partition_det.partition_time_level;
    // End get partition details

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

    let mut stream_status = StreamStatus::new(stream_name);
    let mut json_data = Vec::new(); // TODO(taiming): get size and use capacity

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

                // JSON Flattening
                value = flatten::flatten_with_level(value, cfg.limit.ingest_flatten_level).unwrap();

                if !local_trans.is_empty() {
                    value = crate::service::ingestion::apply_stream_functions(
                        &local_trans,
                        value,
                        &stream_vrl_map,
                        org_id,
                        stream_name,
                        &mut runtime,
                    )
                    .unwrap();
                }

                // get json object
                let mut local_val = match value.take() {
                    json::Value::Object(v) => v,
                    _ => unreachable!(),
                };

                if let Some(fields) = user_defined_schema_map.get(stream_name) {
                    local_val = crate::service::logs::refactor_map(local_val, fields);
                }

                json_data.push((timestamp, local_val));
            }
        }
    }

    let mut res = ExportLogsServiceResponse {
        partial_success: None,
    };

    // QUESTION(taiming): return directly when no data?
    // if no data, fast return
    if json_data.is_empty() {
        let mut out = BytesMut::with_capacity(res.encoded_len());
        res.encode(&mut out).expect("Out of memory");
        return Ok(HttpResponse::Ok()
            .status(http::StatusCode::OK)
            .content_type(CONTENT_TYPE_JSON)
            .body(out)); // just return
    }

    let mut status = IngestionStatus::Record(stream_status.status);
    let mut req_stats = match super::write_logs(
        &super::StreamMeta {
            org_id: org_id.to_string(),
            stream_name: stream_name.to_string(),
            partition_keys: &partition_keys,
            partition_time_level: &partition_time_level,
            stream_alerts_map: &stream_alerts_map,
        },
        &mut stream_schema_map,
        &mut status,
        json_data,
    )
    .await
    {
        Ok(rs) => rs,
        Err(e) => {
            log::error!("Error while writing logs: {}", e);
            stream_status.status = match status {
                IngestionStatus::Record(status) => status,
                IngestionStatus::Bulk(_) => unreachable!(),
            };
            // QUESTION(taiming): return directly when error?
            res.partial_success = Some(ExportLogsPartialSuccess {
                rejected_log_records: stream_status.status.failed as i64,
                error_message: stream_status.status.error,
            });
            let mut out = BytesMut::with_capacity(res.encoded_len());
            res.encode(&mut out).expect("Out of memory");
            return Ok(HttpResponse::Ok()
                .status(http::StatusCode::OK)
                .content_type(CONTENT_TYPE_JSON)
                .body(out));
        }
    };

    let time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/api/org/v1/logs",
            "200",
            org_id,
            stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/api/org/v1/logs",
            "200",
            org_id,
            stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .inc();

    req_stats.response_time = start.elapsed().as_secs_f64();
    req_stats.user_email = Some(user_email.to_string());
    // metric + data usage
    report_request_usage_stats(
        req_stats,
        org_id,
        stream_name,
        StreamType::Logs,
        UsageType::Json,
        0,
        started_at,
    )
    .await;

    stream_status.status = match status {
        IngestionStatus::Record(status) => status,
        IngestionStatus::Bulk(_) => unreachable!(),
    };
    let res = ExportLogsServiceResponse {
        partial_success: Some(ExportLogsPartialSuccess {
            rejected_log_records: stream_status.status.failed as i64,
            error_message: stream_status.status.error,
        }),
    };
    let mut out = BytesMut::with_capacity(res.encoded_len());
    res.encode(&mut out).expect("Out of memory");

    return Ok(HttpResponse::Ok()
        .status(http::StatusCode::OK)
        .content_type(CONTENT_TYPE_JSON)
        .body(out));
}
