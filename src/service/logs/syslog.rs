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

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};

use actix_web::{http, HttpResponse};
use anyhow::Result;
use chrono::{Duration, Utc};
use config::{
    get_config,
    meta::{
        stream::{StreamParams, StreamType},
        usage::UsageType,
    },
    metrics,
    utils::{flatten, json},
    ID_COL_NAME, ORIGINAL_DATA_COL_NAME,
};
use syslog_loose::{Message, ProcId, Protocol};

use super::ingest::handle_timestamp;
use crate::{
    common::{
        infra::config::SYSLOG_ROUTES,
        meta::{
            http::HttpResponse as MetaHttpResponse,
            ingestion::{IngestionResponse, IngestionStatus, StreamStatus},
            syslog::SyslogRoute,
        },
    },
    service::{
        format_stream_name, ingestion::check_ingestion_allowed, pipeline::execution::PipelinedExt,
    },
};

pub async fn ingest(msg: &str, addr: SocketAddr) -> Result<HttpResponse> {
    let start = std::time::Instant::now();
    let started_at: i64 = Utc::now().timestamp_micros();
    let ip = addr.ip();
    let matching_route = get_org_for_ip(ip).await;

    let route = match matching_route {
        Some(matching_route) => matching_route,
        None => {
            log::warn!("Syslogs from the IP {} are not allowed", ip);
            return Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    "Syslogs from the IP are not allowed".to_string(),
                )),
            );
        }
    };

    let in_stream_name = &route.stream_name;
    let org_id = &route.org_id;

    // check stream
    let stream_name = format_stream_name(in_stream_name);
    if let Err(e) = check_ingestion_allowed(org_id, Some(&stream_name)) {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                e.to_string(),
            )),
        );
    };

    let cfg = get_config();
    let min_ts = (Utc::now() - Duration::try_hours(cfg.limit.ingest_allowed_upto).unwrap())
        .timestamp_micros();

    let mut stream_params = vec![StreamParams::new(org_id, &stream_name, StreamType::Logs)];

    let mut runtime = crate::service::ingestion::init_functions_runtime();
    // Start retrieve associated pipeline and construct pipeline components
    let pipeline_params = crate::service::ingestion::get_stream_pipeline_params(
        org_id,
        &stream_name,
        &StreamType::Logs,
    )
    .await;
    // End pipeline construction

    if let Some((pl, node_map, graph, _)) = &pipeline_params {
        let pl_destinations = pl.get_all_destination_streams(node_map, graph);
        stream_params.extend(pl_destinations);
    }

    // Start get user defined schema
    let mut user_defined_schema_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut streams_need_original_set: HashSet<String> = HashSet::new();
    crate::service::ingestion::get_uds_and_original_data_streams(
        &stream_params,
        &mut user_defined_schema_map,
        &mut streams_need_original_set,
    )
    .await;
    // End get user defined schema

    let mut stream_status = StreamStatus::new(&stream_name);
    let mut json_data_by_stream = HashMap::new();

    // parse msg to json::Value
    let parsed_msg = syslog_loose::parse_message(msg);
    let mut value = message_to_value(parsed_msg);

    // store a copy of original data before it's being transformed and/or flattened, unless
    // 1. original data is not an object -> won't be flattened.
    // 2. no routing and current StreamName not in streams_need_original_set
    let original_data = if value.is_object() {
        if pipeline_params.is_none() && !streams_need_original_set.contains(&stream_name) {
            None
        } else {
            // otherwise, make a copy in case the routed stream needs original data
            Some(value.to_string())
        }
    } else {
        None // `item` won't be flattened, no need to store original
    };

    if let Some((pipeline, pl_node_map, pl_graph, vrl_map)) = pipeline_params.as_ref() {
        match pipeline.execute(value, pl_node_map, pl_graph, vrl_map, &mut runtime) {
            Err(e) => {
                log::error!(
                    "[Pipeline] {}/{}/{}: Execution error: {}.",
                    pipeline.org,
                    pipeline.name,
                    pipeline.id,
                    e
                );
                stream_status.status.failed += 1; // pipeline failed or dropped
                return Ok(HttpResponse::Ok().json(IngestionResponse::new(
                    http::StatusCode::OK.into(),
                    vec![stream_status],
                ))); // just return
            }
            Ok(pl_results) => {
                for (stream_params, mut value) in pl_results {
                    if stream_params.stream_type != StreamType::Logs {
                        continue;
                    }

                    // JSON Flattening
                    value = flatten::flatten_with_level(value, cfg.limit.ingest_flatten_level)?;

                    if value.is_null() || !value.is_object() {
                        stream_status.status.failed += 1;
                        continue;
                    }

                    // get json object
                    let mut local_val = match value.take() {
                        json::Value::Object(v) => v,
                        _ => unreachable!(),
                    };

                    if let Some(fields) =
                        user_defined_schema_map.get(stream_params.stream_name.as_str())
                    {
                        local_val = crate::service::logs::refactor_map(local_val, fields);
                    }

                    // add `_original` and '_record_id` if required by StreamSettings
                    if streams_need_original_set.contains(stream_params.stream_name.as_str())
                        && original_data.is_some()
                    {
                        local_val.insert(
                            ORIGINAL_DATA_COL_NAME.to_string(),
                            original_data.clone().unwrap().into(),
                        );
                        let record_id = crate::service::ingestion::generate_record_id(
                            org_id,
                            &stream_name,
                            &StreamType::Logs,
                        );
                        local_val.insert(
                            ID_COL_NAME.to_string(),
                            json::Value::String(record_id.to_string()),
                        );
                    }

                    // handle timestamp
                    let timestamp = match handle_timestamp(&mut local_val, min_ts) {
                        Ok(ts) => ts,
                        Err(e) => {
                            stream_status.status.failed += 1;
                            stream_status.status.error = e.to_string();
                            continue;
                        }
                    };

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
        value = flatten::flatten_with_level(value, cfg.limit.ingest_flatten_level).unwrap();

        // get json object
        let mut local_val = match value.take() {
            json::Value::Object(v) => v,
            _ => unreachable!(),
        };

        if let Some(fields) = user_defined_schema_map.get(&stream_name) {
            local_val = crate::service::logs::refactor_map(local_val, fields);
        }

        // add `_original` and '_record_id` if required by StreamSettings
        if streams_need_original_set.contains(&stream_name) && original_data.is_some() {
            local_val.insert(
                ORIGINAL_DATA_COL_NAME.to_string(),
                original_data.unwrap().into(),
            );
            let record_id = crate::service::ingestion::generate_record_id(
                org_id,
                &stream_name,
                &StreamType::Logs,
            );
            local_val.insert(
                ID_COL_NAME.to_string(),
                json::Value::String(record_id.to_string()),
            );
        }

        // handle timestamp
        let timestamp = match handle_timestamp(&mut local_val, min_ts) {
            Ok(ts) => ts,
            Err(e) => {
                stream_status.status.failed += 1;
                stream_status.status.error = e.to_string();
                return Ok(HttpResponse::Ok().json(IngestionResponse::new(
                    http::StatusCode::OK.into(),
                    vec![stream_status],
                ))); // just return
            }
        };

        let (ts_data, fn_num) = json_data_by_stream
            .entry(stream_name.clone())
            .or_insert((Vec::new(), None));
        ts_data.push((timestamp, local_val));
        *fn_num = Some(0); // no pl -> no func
    }

    // if no data, fast return
    if json_data_by_stream.is_empty() {
        return Ok(HttpResponse::Ok().json(IngestionResponse::new(
            http::StatusCode::OK.into(),
            vec![stream_status],
        ))); // just return
    }

    let (metric_rpt_status_code, response_body) = {
        let mut status = IngestionStatus::Record(stream_status.status);
        let write_result = super::write_logs_by_stream(
            org_id,
            "",
            (started_at, &start),
            UsageType::Syslog,
            &mut status,
            json_data_by_stream,
        )
        .await;
        stream_status.status = match status {
            IngestionStatus::Record(status) => status,
            IngestionStatus::Bulk(_) => unreachable!(),
        };
        match write_result {
            Ok(_) => ("200", stream_status),
            Err(e) => {
                log::error!("Error while writing logs: {}", e);
                ("500", stream_status)
            }
        }
    };

    let time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            "/api/org/ingest/logs/_syslog",
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            "/api/org/ingest/logs/_syslog",
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .inc();

    // drop variables
    drop(pipeline_params);
    drop(user_defined_schema_map);

    Ok(HttpResponse::Ok().json(IngestionResponse::new(
        http::StatusCode::OK.into(),
        vec![response_body],
    )))
}

async fn get_org_for_ip(ip: std::net::IpAddr) -> Option<SyslogRoute> {
    let mut matching_route = None;
    for (_, route) in SYSLOG_ROUTES.clone() {
        for subnet in &route.subnets {
            if subnet.contains(ip) {
                matching_route = Some(route);
                break;
            }
        }
    }
    matching_route
}

/// Create a `Value::Map` from the fields of the given syslog message.
fn message_to_value(message: Message<&str>) -> json::Value {
    let mut result = json::Map::new();

    result.insert("message".to_string(), message.msg.to_string().into());

    if let Some(host) = message.hostname {
        result.insert("hostname".to_string(), host.to_string().into());
    }

    if let Some(severity) = message.severity {
        result.insert("severity".to_string(), severity.as_str().to_owned().into());
    }

    if let Some(facility) = message.facility {
        result.insert("facility".to_string(), facility.as_str().to_owned().into());
    }

    if let Protocol::RFC5424(version) = message.protocol {
        result.insert("version".to_string(), version.into());
    }

    if let Some(app_name) = message.appname {
        result.insert("appname".to_string(), app_name.to_owned().into());
    }

    if let Some(msg_id) = message.msgid {
        result.insert("msgid".to_string(), msg_id.to_owned().into());
    }

    if let Some(timestamp) = message.timestamp {
        result.insert(
            "_timestamp".to_string(),
            timestamp.timestamp_micros().into(),
        );
    }

    if let Some(procid) = message.procid {
        let value: json::Value = match procid {
            ProcId::PID(pid) => pid.into(),
            ProcId::Name(name) => name.to_string().into(),
        };
        result.insert("procid".to_string(), value);
    }

    for element in message.structured_data {
        let mut sdata = json::Map::new();
        for (name, value) in element.params() {
            sdata.insert(name.to_string(), value.into());
        }
        result.insert(element.id.to_string(), sdata.into());
    }

    result.into()
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    #[tokio::test]
    async fn test_ingest() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let raw = r#"<190>2019-02-13T21:53:30.605850+00:00 74794bfb6795 liblogging-stdlog: [origin software="rsyslogd" swVersion="8.24.0" x-pid="9043" x-info="http://www.rsyslog.com"] This is a test message"#;
        ingest(raw, addr).await.unwrap();
    }
}
