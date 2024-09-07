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
    io::{BufRead, Read},
};

use actix_web::http;
use anyhow::Result;
use chrono::{Duration, Utc};
use config::{
    get_config,
    meta::{
        stream::{Routing, StreamParams, StreamType},
        usage::UsageType,
    },
    metrics,
    utils::{flatten, json, time::parse_timestamp_micro_from_value},
};
use flate2::read::GzDecoder;
use vrl::compiler::runtime::Runtime;

use crate::{
    common::meta::{
        functions::{StreamTransform, VRLResultResolver},
        ingestion::{
            AWSRecordType, GCPIngestionResponse, IngestionData, IngestionDataIter, IngestionError,
            IngestionRequest, IngestionResponse, IngestionStatus, KinesisFHIngestionResponse,
            StreamStatus,
        },
    },
    service::{
        format_stream_name, get_formatted_stream_name, ingestion::check_ingestion_allowed,
        schema::get_upto_discard_error,
    },
};

pub async fn ingest(
    org_id: &str,
    in_stream_name: &str,
    in_req: IngestionRequest<'_>,
    user_email: &str,
    extend_json: Option<&HashMap<String, serde_json::Value>>,
) -> Result<IngestionResponse> {
    let cfg = config::get_config();
    let start = std::time::Instant::now();
    let started_at: i64 = Utc::now().timestamp_micros();
    let mut need_usage_report = true;

    // check stream
    let stream_name = if cfg.common.skip_formatting_stream_name {
        get_formatted_stream_name(StreamParams::new(org_id, in_stream_name, StreamType::Logs))
            .await?
    } else {
        format_stream_name(in_stream_name)
    };
    check_ingestion_allowed(org_id, Some(&stream_name))?;

    let min_ts = (Utc::now() - Duration::try_hours(cfg.limit.ingest_allowed_upto).unwrap())
        .timestamp_micros();

    let mut runtime = crate::service::ingestion::init_functions_runtime();
    let mut stream_vrl_map: HashMap<String, VRLResultResolver> = HashMap::new();
    let mut stream_before_functions_map: HashMap<String, Vec<StreamTransform>> = HashMap::new();
    let mut stream_after_functions_map: HashMap<String, Vec<StreamTransform>> = HashMap::new();

    let mut stream_params = vec![StreamParams::new(org_id, &stream_name, StreamType::Logs)];

    // Start get routing keys
    let mut stream_routing_map: HashMap<String, Vec<Routing>> = HashMap::new();
    crate::service::ingestion::get_stream_routing(
        StreamParams::new(org_id, &stream_name, StreamType::Logs),
        &mut stream_routing_map,
    )
    .await;

    if let Some(routes) = stream_routing_map.get(&stream_name) {
        for route in routes {
            stream_params.push(StreamParams::new(
                org_id,
                &route.destination,
                StreamType::Logs,
            ));
        }
    }
    // End get routing keys

    // Start get user defined schema
    let mut user_defined_schema_map: HashMap<String, HashSet<String>> = HashMap::new();
    crate::service::ingestion::get_user_defined_schema(
        &stream_params,
        &mut user_defined_schema_map,
    )
    .await;
    // End get user defined schema

    // Start Register functions for stream
    crate::service::ingestion::get_stream_functions(
        &stream_params,
        &mut stream_before_functions_map,
        &mut stream_after_functions_map,
        &mut stream_vrl_map,
    )
    .await;
    // End Register functions for index

    let json_req: Vec<json::Value>; // to hold json request because of borrow checker
    let (endpoint, usage_type, data) = match in_req {
        IngestionRequest::JSON(req) => {
            json_req = json::from_slice(req).unwrap_or({
                let val: json::Value = json::from_slice(req)?;
                vec![val]
            });
            (
                "/api/org/ingest/logs/_json",
                UsageType::Json,
                IngestionData::JSON(&json_req),
            )
        }
        IngestionRequest::Multi(req) => (
            "/api/org/ingest/logs/_multi",
            UsageType::Multi,
            IngestionData::Multi(req),
        ),
        IngestionRequest::GCP(req) => (
            "/api/org/ingest/logs/_gcs",
            UsageType::GCPSubscription,
            IngestionData::GCP(req),
        ),
        IngestionRequest::KinesisFH(req) => (
            "/api/org/ingest/logs/_kinesis",
            UsageType::KinesisFirehose,
            IngestionData::KinesisFH(req),
        ),
        IngestionRequest::RUM(req) => (
            "/api/org/ingest/logs/_rum",
            UsageType::RUM,
            IngestionData::Multi(req),
        ),
        IngestionRequest::Usage(req) => {
            // no need to report usage for usage data
            need_usage_report = false;
            json_req = json::from_slice(req).unwrap_or({
                let val: json::Value = json::from_slice(req)?;
                vec![val]
            });
            (
                "/api/org/ingest/logs/_usage",
                UsageType::Json,
                IngestionData::JSON(&json_req),
            )
        }
    };

    let mut stream_status = StreamStatus::new(&stream_name);
    let mut json_data_by_stream = HashMap::new();
    for ret in data.iter() {
        let mut item = match ret {
            Ok(item) => item,
            Err(e) => {
                log::error!("IngestionError: {:?}", e);
                return Err(anyhow::anyhow!("Failed processing: {:?}", e));
            }
        };
        let mut routed_stream_name = stream_name.clone();

        if let Some(extend) = extend_json.as_ref() {
            for (key, val) in extend.iter() {
                item[key] = val.clone();
            }
        }

        let main_stream_key = format!("{org_id}/{}/{stream_name}", StreamType::Logs);
        // Start row based transform before flattening the value
        if let Some(transforms) = stream_before_functions_map.get(&main_stream_key) {
            if !transforms.is_empty() {
                item = match apply_functions(
                    item,
                    transforms,
                    &stream_vrl_map,
                    org_id,
                    &routed_stream_name,
                    &mut runtime,
                ) {
                    Ok(res) => res,
                    Err(e) => {
                        stream_status.status.failed += 1;
                        stream_status.status.error = e.to_string();
                        continue;
                    }
                }
            }
        }
        // end row based transformation

        // JSON Flattening
        let item = flatten::flatten_with_level(item, cfg.limit.ingest_flatten_level)?;

        // Start re-routing if exists
        if let Some(routings) = stream_routing_map.get(&routed_stream_name) {
            if !routings.is_empty() {
                for route in routings {
                    let mut is_routed = true;
                    let val = &route.routing;
                    for q_condition in val.iter() {
                        if !q_condition.evaluate(item.as_object().unwrap()).await {
                            is_routed = false;
                            break;
                        }
                    }
                    if !val.is_empty() && is_routed {
                        routed_stream_name = route.destination.clone();
                        break;
                    }
                }
            }
        }
        // End re-routing

        let key = format!("{org_id}/{}/{routed_stream_name}", StreamType::Logs);
        // Start row based transform
        let mut res = if let Some(transforms) = stream_after_functions_map.get(&key) {
            match apply_functions(
                item,
                transforms,
                &stream_vrl_map,
                org_id,
                &routed_stream_name,
                &mut runtime,
            ) {
                Ok(res) => res,
                Err(e) => {
                    stream_status.status.failed += 1;
                    stream_status.status.error = e.to_string();
                    continue;
                }
            }
        } else {
            item
        };
        // end row based transform

        // get json object
        let mut local_val = match res.take() {
            json::Value::Object(val) => val,
            _ => unreachable!(),
        };

        if let Some(fields) = user_defined_schema_map.get(&routed_stream_name) {
            local_val = crate::service::logs::refactor_map(local_val, fields);
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

        let function_no = stream_before_functions_map
            .get(&main_stream_key)
            .map(|v| v.len())
            .unwrap_or_default()
            + stream_after_functions_map
                .get(&key)
                .map(|v| v.len())
                .unwrap_or_default();
        let (ts_data, fn_num) = json_data_by_stream
            .entry(routed_stream_name.clone())
            .or_insert((Vec::new(), None));
        ts_data.push((timestamp, local_val));
        *fn_num = need_usage_report.then_some(function_no);
    }

    // if no data, fast return
    if json_data_by_stream.is_empty() {
        return Ok(IngestionResponse::new(
            http::StatusCode::OK.into(),
            vec![stream_status],
        ));
    }

    let (metric_rpt_status_code, response_body) = {
        let mut status = IngestionStatus::Record(stream_status.status);
        let write_result = super::write_logs_by_stream(
            org_id,
            user_email,
            (started_at, &start),
            usage_type,
            &mut status,
            json_data_by_stream,
        )
        .await;
        stream_status.status = match status {
            IngestionStatus::Record(status) => status,
            IngestionStatus::Bulk(_) => unreachable!(),
        };
        match write_result {
            Ok(()) => ("200", stream_status),
            Err(e) => {
                log::error!("Error while writing logs: {}", e);
                ("500", stream_status)
            }
        }
    };

    // update ingestion metrics
    let took_time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            endpoint,
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .observe(took_time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            endpoint,
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.to_string().as_str(),
        ])
        .inc();

    // drop variables
    drop(runtime);
    drop(stream_vrl_map);
    drop(stream_routing_map);
    drop(user_defined_schema_map);

    Ok(IngestionResponse::new(
        http::StatusCode::OK.into(),
        vec![response_body],
    ))
}

pub fn apply_functions<'a>(
    item: json::Value,
    local_trans: &[StreamTransform],
    stream_vrl_map: &'a HashMap<String, VRLResultResolver>,
    org_id: &'a str,
    stream_name: &'a str,
    runtime: &mut Runtime,
) -> Result<json::Value> {
    let mut value = item;
    if !local_trans.is_empty() {
        value = crate::service::ingestion::apply_stream_functions(
            local_trans,
            value,
            stream_vrl_map,
            org_id,
            stream_name,
            runtime,
        )?;
    }

    if value.is_null() || !value.is_object() {
        Err(anyhow::Error::msg("apply functions failure"))
    } else {
        Ok(value)
    }
}

pub fn handle_timestamp(
    local_val: &mut json::Map<String, json::Value>,
    min_ts: i64,
) -> Result<i64, anyhow::Error> {
    let cfg = get_config();
    // handle timestamp
    let timestamp = match local_val.get(&cfg.common.column_timestamp) {
        Some(v) => match parse_timestamp_micro_from_value(v) {
            Ok(t) => t,
            Err(_) => return Err(anyhow::Error::msg("Can't parse timestamp")),
        },
        None => Utc::now().timestamp_micros(),
    };
    // check ingestion time
    if timestamp < min_ts {
        return Err(get_upto_discard_error());
    }
    local_val.insert(
        cfg.common.column_timestamp.clone(),
        json::Value::Number(timestamp.into()),
    );
    Ok(timestamp)
}

impl<'a> Iterator for IngestionDataIter<'a> {
    type Item = Result<json::Value, IngestionError>;

    fn next(&mut self) -> Option<Result<json::Value, IngestionError>> {
        match self {
            IngestionDataIter::JSONIter(iter) => iter.next().cloned().map(Ok),
            IngestionDataIter::MultiIter(iter) => loop {
                match iter.next() {
                    Some(Ok(line)) if line.trim().is_empty() => {
                        // If the line is empty, just continue to the next iteration.
                        continue;
                    }
                    Some(Ok(line)) => {
                        // If the line is not empty, attempt to parse it as JSON.
                        return Some(json::from_str(&line).map_err(IngestionError::from));
                    }
                    Some(Err(e)) => {
                        // If there's an error reading the line, return it.
                        return Some(Err(IngestionError::from(e)));
                    }
                    None => {
                        // If there are no more lines, return None.
                        return None;
                    }
                }
            },
            IngestionDataIter::GCP(iter, err) => match err {
                Some(e) => Some(Err(IngestionError::GCPError(e.clone()))),
                None => iter.next().map(Ok),
            },
            IngestionDataIter::KinesisFH(iter, err) => match err {
                Some(e) => Some(Err(IngestionError::AWSError(e.clone()))),
                None => iter.next().map(Ok),
            },
        }
    }
}

impl<'a> IngestionData<'a> {
    pub fn iter(&'a self) -> IngestionDataIter<'a> {
        match self {
            IngestionData::JSON(vec) => IngestionDataIter::JSONIter(vec.iter()),
            IngestionData::Multi(data) => {
                IngestionDataIter::MultiIter(std::io::BufReader::new(*data).lines())
            }
            IngestionData::GCP(request) => {
                let data = &request.message.data;
                let request_id = &request.message.message_id;
                let req_timestamp = &request.message.publish_time;
                match decode_and_decompress(data) {
                    Ok(decompressed_data) => {
                        let value: json::Value = json::from_str(&decompressed_data).unwrap();
                        IngestionDataIter::GCP(vec![value].into_iter(), None)
                    }
                    Err(e) => IngestionDataIter::GCP(
                        vec![].into_iter(),
                        Some(GCPIngestionResponse {
                            request_id: request_id.to_string(),
                            error_message: Some(e.to_string()),
                            timestamp: req_timestamp.to_string(),
                        }),
                    ),
                }
            }
            IngestionData::KinesisFH(request) => {
                let mut events = Vec::new();
                let request_id = &request.request_id;
                let req_timestamp = request.timestamp.unwrap_or(Utc::now().timestamp_micros());

                for record in &request.records {
                    match decode_and_decompress(&record.data) {
                        Err(err) => {
                            return IngestionDataIter::KinesisFH(
                                events.into_iter(),
                                Some(KinesisFHIngestionResponse {
                                    request_id: request_id.to_string(),
                                    error_message: Some(err.to_string()),
                                    timestamp: req_timestamp,
                                }),
                            );
                        }
                        Ok(decompressed_data) => {
                            match deserialize_aws_record_from_str(&decompressed_data, request_id) {
                                Ok(parsed_events) => events.extend(parsed_events),
                                Err(err) => {
                                    return IngestionDataIter::KinesisFH(
                                        events.into_iter(),
                                        Some(KinesisFHIngestionResponse {
                                            request_id: request_id.to_string(),
                                            error_message: Some(err.to_string()),
                                            timestamp: req_timestamp,
                                        }),
                                    );
                                }
                            }
                        }
                    }
                }
                return IngestionDataIter::KinesisFH(events.into_iter(), None);
            }
        }
    }
}

pub fn decode_and_decompress(encoded_data: &str) -> Result<String, Box<dyn std::error::Error>> {
    let decoded_data = config::utils::base64::decode_raw(encoded_data)?;
    let mut gz = GzDecoder::new(&decoded_data[..]);
    let mut decompressed_data = String::new();
    match gz.read_to_string(&mut decompressed_data) {
        Ok(_) => Ok(decompressed_data),
        Err(_) => Ok(String::from_utf8(decoded_data)?),
    }
}

fn deserialize_aws_record_from_str(data: &str, request_id: &str) -> Result<Vec<json::Value>> {
    let mut events = vec![];
    let mut value;
    for line in data.lines() {
        match json::from_str(line) {
            Ok(AWSRecordType::KinesisFHLogs(kfh_log_data)) => {
                for event in kfh_log_data.log_events.iter() {
                    value = json::to_value(event)?;
                    let local_val = value
                        .as_object_mut()
                        .ok_or(anyhow::anyhow!("Error to convert Value to object"))?;

                    local_val.insert("requestId".to_owned(), request_id.into());
                    local_val.insert(
                        "messageType".to_owned(),
                        kfh_log_data.message_type.clone().into(),
                    );
                    local_val.insert("owner".to_owned(), kfh_log_data.owner.clone().into());
                    local_val.insert("logGroup".to_owned(), kfh_log_data.log_group.clone().into());
                    local_val.insert(
                        "logStream".to_owned(),
                        kfh_log_data.log_stream.clone().into(),
                    );
                    local_val.insert(
                        "subscriptionFilters".to_owned(),
                        kfh_log_data.subscription_filters.clone().into(),
                    );

                    let local_msg = event.message.as_str().unwrap();

                    if local_msg.starts_with('{') && local_msg.ends_with('}') {
                        let result: Result<json::Value, json::Error> = json::from_str(local_msg);

                        match result {
                            Err(_e) => {
                                local_val.insert("message".to_owned(), event.message.clone());
                            }
                            Ok(message_val) => {
                                local_val.insert("message".to_owned(), message_val.clone());
                            }
                        }
                    } else {
                        local_val.insert("message".to_owned(), local_msg.into());
                    }

                    local_val.insert(
                        get_config().common.column_timestamp.clone(),
                        event.timestamp.into(),
                    );

                    value = local_val.clone().into();
                    events.push(value);
                }
            }
            Ok(AWSRecordType::KinesisFHMetrics(kfh_metric_data)) => {
                // Parse "dimensions" and "values" fields from KinesisFHMetricData
                let values = json::to_value(kfh_metric_data.value.clone())?;
                let dimensions = kfh_metric_data.dimensions.clone();
                let timestamp = kfh_metric_data.timestamp;

                let mut parsed_metric_value = json::to_value(kfh_metric_data)?;
                let local_parsed_metric_value = parsed_metric_value.as_object_mut().ok_or(
                    anyhow::anyhow!("CloudWatch metrics failed to parse Metric Object"),
                )?;

                for (value_name, value_val) in values.as_object().ok_or(anyhow::anyhow!(
                    "CloudWatch metrics failed to Metric Value Object"
                ))? {
                    local_parsed_metric_value.insert(value_name.to_owned(), value_val.to_owned());
                }
                local_parsed_metric_value.remove("value");

                let metric_dimensions = dimensions
                    .as_object()
                    .ok_or(anyhow::anyhow!(
                        "CloudWatch metrics dimensions parsing failed"
                    ))?
                    .iter()
                    .map(|(k, v)| format!("{}=[{}]", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");

                local_parsed_metric_value
                    .insert("metric_dimensions".to_owned(), metric_dimensions.into());
                local_parsed_metric_value.remove("dimensions");

                local_parsed_metric_value.insert(
                    get_config().common.column_timestamp.clone(),
                    timestamp.into(),
                );
                local_parsed_metric_value.remove("timestamp");

                value = local_parsed_metric_value.clone().into();
                events.push(value);
            }
            _ => {
                value = json::from_str(line)?;
                events.push(value);
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::{decode_and_decompress, deserialize_aws_record_from_str};

    #[test]
    fn test_decode_and_decompress_success() {
        let encoded_data = "H4sIAAAAAAAAADWO0QqCMBiFX2XsOkKJZHkXot5YQgpdhMTSPzfSTbaZhPjuzbTLj3M45xtxC1rTGvJPB9jHQXrOL2lyP4VZdoxDvMFyEKDmpJF9NVBTskTW2gaNrGMl+85mC2VGAW0X1P1Dl4p3hksR8caA0ti/Fb9e+AZhZhwxr5a64VbD0NaOuR5xPLJzycEh+81fbxa4JmjVQ6uejwIG5YuLGjGgjWFIPlFll7ig8zOKuAImNWzxVExfL8ipzewAAAA=";
        let expected = "{\"messageType\":\"CONTROL_MESSAGE\",\"owner\":\"CloudwatchLogs\",\"logGroup\":\"\",\"logStream\":\"\",\"subscriptionFilters\":[],\"logEvents\":[{\"id\":\"\",\"timestamp\":1680683189085,\"message\":\"CWL CONTROL MESSAGE: Checking health of destination Firehose.\"}]}";
        let result =
            decode_and_decompress(encoded_data).expect("Failed to decode and decompress data");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_success() {
        let encoded_data = "eyJtZXNzYWdlIjoiMiAwNTg2OTQ4NTY0NzYgZW5pLTAzYzBmNWJhNzlhNjZlZjE3IDEwLjMuMTY2LjcxIDEwLjMuMTQxLjIwOSA0NDMgMzg2MzQgNiAxMDMgNDI5MjYgMTY4MDgzODU1NiAxNjgwODM4NTc4IEFDQ0VQVCBPSyJ9Cg==";
        let expected = "{\"message\":\"2 058694856476 eni-03c0f5ba79a66ef17 10.3.166.71 10.3.141.209 443 38634 6 103 42926 1680838556 1680838578 ACCEPT OK\"}\n";
        let result = decode_and_decompress(encoded_data).expect("Failed to decode data");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_and_decompress_invalid_base64() {
        let encoded_data = "H4sIAAAAAAAC/ytJLS4BAAxGw7gNAAA&"; // Invalid base64 string
        let result = decode_and_decompress(encoded_data);
        assert!(
            result.is_err(),
            "Expected an error due to invalid base64 input"
        );
    }

    #[test]
    fn test_deserialize_from_str_metrics() {
        let encoded_data = "eyJtZXRyaWNfc3RyZWFtX25hbWUiOiJDdXN0b21QYXJ0aWFsLUJDbjVjQSIsImFjY291bnRfaWQiOiI3MzkxNDcyMjI5ODkiLCJyZWdpb24iOiJ1cy1lYXN0LTIiLCJuYW1lc3BhY2UiOiJBV1MvVXNhZ2UiLCJtZXRyaWNfbmFtZSI6IkNhbGxDb3VudCIsImRpbWVuc2lvbnMiOnsiQ2xhc3MiOiJOb25lIiwiUmVzb3VyY2UiOiJHZXRNZXRyaWNEYXRhIiwiU2VydmljZSI6IkNsb3VkV2F0Y2giLCJUeXBlIjoiQVBJIn0sInRpbWVzdGFtcCI6MTcxMzkwMjcwMDAwMCwidmFsdWUiOnsibWF4IjoxLjAsIm1pbiI6MS4wLCJzdW0iOjMuMCwiY291bnQiOjMuMH0sInVuaXQiOiJOb25lIn0KeyJtZXRyaWNfc3RyZWFtX25hbWUiOiJDdXN0b21QYXJ0aWFsLUJDbjVjQSIsImFjY291bnRfaWQiOiI3MzkxNDcyMjI5ODkiLCJyZWdpb24iOiJ1cy1lYXN0LTIiLCJuYW1lc3BhY2UiOiJBV1MvRmlyZWhvc2UiLCJtZXRyaWNfbmFtZSI6IktNU0tleUludmFsaWRTdGF0ZSIsImRpbWVuc2lvbnMiOnsiRGVsaXZlcnlTdHJlYW1OYW1lIjoiUFVULUhUUC1SZFFXOCJ9LCJ0aW1lc3RhbXAiOjE3MTM5MDI2NDAwMDAsInZhbHVlIjp7Im1heCI6MC4wLCJtaW4iOjAuMCwic3VtIjowLjAsImNvdW50Ijo2MC4wfSwidW5pdCI6IkNvdW50In0KeyJtZXRyaWNfc3RyZWFtX25hbWUiOiJDdXN0b21QYXJ0aWFsLUJDbjVjQSIsImFjY291bnRfaWQiOiI3MzkxNDcyMjI5ODkiLCJyZWdpb24iOiJ1cy1lYXN0LTIiLCJuYW1lc3BhY2UiOiJBV1MvRmlyZWhvc2UiLCJtZXRyaWNfbmFtZSI6IktNU0tleU5vdEZvdW5kIiwiZGltZW5zaW9ucyI6eyJEZWxpdmVyeVN0cmVhbU5hbWUiOiJQVVQtSFRQLVJkUVc4In0sInRpbWVzdGFtcCI6MTcxMzkwMjY0MDAwMCwidmFsdWUiOnsibWF4IjowLjAsIm1pbiI6MC4wLCJzdW0iOjAuMCwiY291bnQiOjYwLjB9LCJ1bml0IjoiQ291bnQifQo=";
        let decoded = decode_and_decompress(encoded_data);
        assert!(decoded.is_ok());
        let decoded = decoded.unwrap();
        let request_id = "test_id".to_string();
        let result = deserialize_aws_record_from_str(&decoded, &request_id);
        assert!(result.is_ok());
        let value = result.unwrap();
        for val in value {
            assert_eq!(val.get("account_id").unwrap(), "739147222989");
        }
    }

    #[test]
    fn test_deserialize_from_str_logs() {
        let encoded_data = "eyJtZXNzYWdlVHlwZSI6IkRBVEFfTUVTU0FHRSIsIm93bmVyIjoiMTIzNDU2Nzg5MDEyIiwibG9nR3JvdXAiOiJsb2dfZ3JvdXBfbmFtZSIsImxvZ1N0cmVhbSI6ImxvZ19zdHJlYW1fbmFtZSIsInN1YnNjcmlwdGlvbkZpbHRlcnMiOlsic3Vic2NyaXB0aW9uX2ZpbHRlcl9uYW1lIl0sImxvZ0V2ZW50cyI6W3siaWQiOiIwMTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDEyMzQ1IiwidGltZXN0YW1wIjoxNzEzOTgzNDQ2LCJtZXNzYWdlIjoibG9nbWVzc2FnZTEifSx7ImlkIjoiMDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NSIsInRpbWVzdGFtcCI6IDE3MTM5ODM0NDYsIm1lc3NhZ2UiOiJsb2dtZXNzYWdlMiJ9XX0=";
        let decoded = decode_and_decompress(encoded_data);
        assert!(decoded.is_ok());
        let decoded = decoded.unwrap();
        let request_id = "test_id".to_string();
        let result = deserialize_aws_record_from_str(&decoded, &request_id);
        assert!(result.is_ok());
        let result = result.unwrap();
        for val in result {
            assert_eq!(val.get("owner").unwrap(), "123456789012");
        }
    }
}
