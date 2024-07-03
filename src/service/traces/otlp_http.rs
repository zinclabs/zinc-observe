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

use std::{collections::HashMap, io::Error};

use actix_web::{http, web, HttpResponse};
use chrono::{Duration, Utc};
use config::{
    cluster, get_config,
    meta::{stream::StreamType, usage::UsageType},
    metrics,
    utils::{flatten, json},
};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;

use super::{BLOCK_FIELDS, PARENT_SPAN_ID, PARENT_TRACE_ID, REF_TYPE, SERVICE, SERVICE_NAME};
use crate::{
    common::meta::{
        http::HttpResponse as MetaHttpResponse,
        traces::{
            Event, ExportTracePartialSuccess, ExportTraceServiceResponse, Span, SpanLink,
            SpanLinkContext, SpanRefType,
        },
    },
    service::{
        db, format_stream_name, ingestion::grpc::get_val_for_attr,
        usage::report_request_usage_stats,
    },
};

pub async fn traces_proto(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
) -> Result<HttpResponse, Error> {
    let request = match ExportTraceServiceRequest::decode(body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Invalid proto: {}", e),
            )));
        }
    };
    super::handle_trace_request(org_id, request, false, in_stream_name).await
}

pub async fn traces_json(
    org_id: &str,
    body: web::Bytes,
    in_stream_name: Option<&str>,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();

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

    // check memtable
    if let Err(e) = ingester::check_memtable_size() {
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
    let min_ts = (Utc::now() - Duration::try_hours(cfg.limit.ingest_allowed_upto).unwrap())
        .timestamp_micros();

    // Start Register Transforms for stream
    let mut runtime = crate::service::ingestion::init_functions_runtime();
    let (local_trans, stream_vrl_map) = crate::service::ingestion::register_stream_functions(
        org_id,
        &StreamType::Traces,
        &traces_stream_name,
    );
    // End Register Transforms for stream

    let body: json::Value = match json::from_slice(body.as_ref()) {
        Ok(v) => v,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("Invalid json: {}", e),
            )));
        }
    };
    let spans = match body.get("resourceSpans") {
        Some(json::Value::Array(v)) => v,
        _ => {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Invalid json: the structure must be {{\"resourceSpans\":[]}}".to_string(),
            )));
        }
    };

    let mut service_name: String = traces_stream_name.to_string();
    let mut json_data = Vec::with_capacity(64);
    let mut partial_success = ExportTracePartialSuccess::default();
    log::debug!("the spans here: {:#?}", spans);
    for res_span in spans.iter() {
        let mut service_att_map: HashMap<String, json::Value> = HashMap::new();
        if res_span.get("resource").is_some() {
            let resource = res_span.get("resource").unwrap().as_object().unwrap();
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
                            service_name = item.1.as_str().unwrap().to_string();
                            service_att_map.insert(SERVICE_NAME.to_string(), item.1.clone());
                        }
                    } else {
                        service_att_map.insert(
                            format!(
                                "{}.{}",
                                SERVICE,
                                local_attr.get("key").unwrap().as_str().unwrap()
                            ),
                            get_val_for_attr(local_attr.get("value").unwrap().clone()),
                        );
                    }
                }
            }
        }
        let scope_resources = res_span.get("scopeSpans");
        let inst_resources = if let Some(v) = scope_resources {
            v.as_array().unwrap()
        } else {
            res_span
                .get("instrumentationLibrarySpans")
                .unwrap()
                .as_array()
                .unwrap()
        };
        for inst_span in inst_resources {
            if inst_span.get("spans").is_some() {
                let spans = inst_span.get("spans").unwrap().as_array().unwrap();
                for span in spans {
                    let span_id: String = span.get("spanId").unwrap().as_str().unwrap().to_string();
                    let trace_id: String =
                        span.get("traceId").unwrap().as_str().unwrap().to_string();

                    let mut span_ref = HashMap::new();
                    if span.get("parentSpanId").is_some() {
                        span_ref.insert(
                            PARENT_SPAN_ID.to_string(),
                            span.get("parentSpanId")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_string(),
                        );
                        span_ref.insert(PARENT_TRACE_ID.to_string(), trace_id.clone());
                        span_ref
                            .insert(REF_TYPE.to_string(), format!("{:?}", SpanRefType::ChildOf));
                    }

                    let start_time = json::get_uint_value(span.get("startTimeUnixNano").unwrap());
                    let end_time = json::get_uint_value(span.get("endTimeUnixNano").unwrap());
                    let mut span_att_map: HashMap<String, json::Value> = HashMap::new();
                    let attributes = span.get("attributes").unwrap().as_array().unwrap();
                    for span_att in attributes {
                        let mut key = span_att.get("key").unwrap().as_str().unwrap().to_string();
                        if BLOCK_FIELDS.contains(&key.as_str()) {
                            key = format!("attr_{}", key);
                        }
                        span_att_map.insert(
                            key,
                            get_val_for_attr(span_att.get("value").unwrap().clone()),
                        );
                    }

                    let mut events = vec![];
                    let mut links = vec![];
                    let mut event_att_map: HashMap<String, json::Value> = HashMap::new();
                    let mut link_att_map: HashMap<String, json::Value> = HashMap::new();

                    let empty_vec = Vec::new();
                    let span_events = match span.get("events") {
                        Some(v) => v.as_array().unwrap(),
                        None => &empty_vec,
                    };
                    for event in span_events {
                        let attributes = event.get("attributes").unwrap().as_array().unwrap();
                        for event_att in attributes {
                            event_att_map.insert(
                                event_att.get("key").unwrap().as_str().unwrap().to_string(),
                                get_val_for_attr(event_att.get("value").unwrap().clone()),
                            );
                        }
                        events.push(Event {
                            name: event.get("name").unwrap().as_str().unwrap().to_string(),
                            _timestamp: json::get_uint_value(event.get("timeUnixNano").unwrap()),
                            attributes: event_att_map.clone(),
                        })
                    }
                    let span_links = match span.get("links") {
                        Some(v) => v.as_array().unwrap(),
                        None => &empty_vec,
                    };
                    for link in span_links {
                        let attributes = link.get("attributes").unwrap().as_array().unwrap();
                        for link_att in attributes {
                            link_att_map.insert(
                                link_att.get("key").unwrap().as_str().unwrap().to_string(),
                                get_val_for_attr(link_att.get("value").unwrap().clone()),
                            );
                        }
                        let (mut trace_id, mut span_id, mut trace_flags, mut trace_state) =
                            (None, None, None, None);
                        if let Some(link_trace) = link.get("traceId") {
                            trace_id = Some(link_trace.as_str().unwrap().to_owned());
                        }

                        if let Some(link_span) = link.get("spanId") {
                            span_id = Some(link_span.as_str().unwrap().to_owned());
                        }

                        if let Some(link_trace_flags) = link.get("traceFlags") {
                            trace_flags = Some(link_trace_flags.as_u64().unwrap() as u32);
                        }

                        if trace_flags.is_none() {
                            if let Some(link_trace_flags) = link.get("flags") {
                                trace_flags = Some(link_trace_flags.as_u64().unwrap() as u32);
                            }
                        }

                        if let Some(link_trace_state) = link.get("traceState") {
                            trace_state = Some(link_trace_state.as_str().unwrap().to_owned());
                        }

                        if trace_id.is_none()
                            || span_id.is_none()
                            || trace_flags.is_none()
                            || trace_state.is_none()
                        {
                            if let Some(span_context) = link.get("context") {
                                let span_context: SpanLinkContext =
                                    json::from_value(span_context.to_owned()).unwrap();
                                if trace_id.is_none() {
                                    trace_id = Some(span_context.trace_id);
                                }
                                if span_id.is_none() {
                                    span_id = Some(span_context.span_id);
                                }
                                if trace_flags.is_none() {
                                    trace_flags = span_context.trace_flags;
                                }
                                if trace_state.is_none() {
                                    trace_state = span_context.trace_state;
                                }
                            }
                        }
                        links.push(SpanLink {
                            context: SpanLinkContext {
                                span_id: span_id.unwrap(),
                                trace_id: trace_id.unwrap(),
                                trace_flags,
                                trace_state,
                            },
                            attributes: link_att_map.clone(),
                            dropped_attributes_count: 0, // TODO: add appropriate value
                        })
                    }

                    let timestamp = (start_time / 1000) as i64;
                    if timestamp < min_ts {
                        partial_success.rejected_spans += 1;
                        continue;
                    }

                    let local_val = Span {
                        trace_id: trace_id.clone(),
                        span_id,
                        span_kind: span.get("kind").unwrap().to_string(),
                        span_status: json::get_string_value(
                            span.get("status")
                                .unwrap_or(&json::Value::String("UNSET".to_string())),
                        ),
                        operation_name: span.get("name").unwrap().as_str().unwrap().to_string(),
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

                    let mut value: json::Value = json::to_value(local_val).unwrap();

                    // JSON Flattening
                    value = flatten::flatten(value).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                    })?;

                    // Start row based transform
                    if !local_trans.is_empty() {
                        value = crate::service::ingestion::apply_stream_functions(
                            &local_trans,
                            value,
                            &stream_vrl_map,
                            org_id,
                            &traces_stream_name,
                            &mut runtime,
                        )
                        .map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                        })?;
                    }
                    // End row based transform

                    // get json object
                    let mut record_val = match value.take() {
                        json::Value::Object(v) => v,
                        _ => unreachable!(),
                    };

                    // add timestamp
                    record_val.insert(
                        cfg.common.column_timestamp.clone(),
                        json::Value::Number(timestamp.into()),
                    );
                    json_data.push((timestamp, record_val));
                }
            }
        }
    }

    // if no data, fast return
    if json_data.is_empty() {
        return format_response(partial_success);
    }

    let mut req_stats = match super::write_traces(org_id, &traces_stream_name, json_data).await {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error while writing traces: {}", e);
            return format_response(partial_success);
        }
    };
    let time = start.elapsed().as_secs_f64();
    req_stats.response_time = time;

    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/api/org/v1/traces",
            "200",
            org_id,
            &traces_stream_name,
            StreamType::Traces.to_string().as_str(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/api/org/v1/traces",
            "200",
            org_id,
            &traces_stream_name,
            StreamType::Traces.to_string().as_str(),
        ])
        .inc();

    // metric + data usage
    report_request_usage_stats(
        req_stats,
        org_id,
        &traces_stream_name,
        StreamType::Traces,
        UsageType::Traces,
        0,
        started_at,
    )
    .await;

    format_response(partial_success)
}

#[cfg(test)]
mod tests {
    use json::json;

    use super::*;

    #[test]
    fn test_get_val_for_attr() {
        let in_val = 10.00;
        let input = json!({ "key": in_val });
        let resp = get_val_for_attr(input);
        assert_eq!(resp.as_str().unwrap(), in_val.to_string());
    }
}

fn format_response(mut partial_success: ExportTracePartialSuccess) -> Result<HttpResponse, Error> {
    Ok(if partial_success.rejected_spans > 0 {
        partial_success.error_message =
            "Some spans were rejected due to exceeding the allowed retention period".to_string();
        HttpResponse::PartialContent().json(ExportTraceServiceResponse {
            partial_success: Some(partial_success),
        })
    } else {
        HttpResponse::Ok().json(ExportTraceServiceResponse::default())
    })
}
