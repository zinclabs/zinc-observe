use ahash::AHashMap;
use chrono::{Datelike, Timelike, Utc};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::common::json;
use crate::common::meta::usage::{STATS_STREAM, USAGE_STREAM};
use crate::common::{
    infra::{config::CONFIG, metrics},
    meta::{
        usage::{AggregatedData, GroupKey, RequestStats, UsageData, UsageEvent, UsageType},
        StreamType,
    },
};
use crate::handler::grpc::cluster_rpc::UsageDataList;

pub mod ingestion_service;
pub mod stats;

pub static USAGE_DATA: Lazy<Arc<RwLock<Vec<UsageData>>>> =
    Lazy::new(|| Arc::new(RwLock::new(vec![])));

pub async fn report_request_usage_stats(
    stats: RequestStats,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    usage_type: UsageType,
    num_functions: u16,
) {
    metrics::INGEST_RECORDS
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .inc_by(stats.records);
    metrics::INGEST_BYTES
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .inc_by(stats.size as u64);
    let event: UsageEvent = usage_type.into();

    if !CONFIG.common.usage_enabled {
        return;
    }
    if CONFIG.common.usage_org.eq(org_id)
        && (stream_name.eq(STATS_STREAM) || stream_name.eq(USAGE_STREAM))
    {
        return;
    }

    let request_body = usage_type.to_string();
    let now = Utc::now();

    let mut usage = vec![];

    if num_functions > 0 {
        usage.push(UsageData {
            event: UsageEvent::Functions,
            day: now.day(),
            hour: now.hour(),
            month: now.month(),
            year: now.year(),
            org_id: org_id.to_owned(),
            request_body: request_body.to_owned(),
            size: stats.size,
            unit: "MB".to_owned(),
            user_email: "".to_owned(),
            response_time: stats.response_time,
            num_records: stats.records * num_functions as u64,
            stream_type,
            stream_name: stream_name.to_owned(),
            min_ts: None,
            max_ts: None,
            compressed_size: None,
        });
    };

    if !CONFIG.common.report_compressed_size || event != UsageEvent::Ingestion {
        usage.push(UsageData {
            event,
            day: now.day(),
            hour: now.hour(),
            month: now.month(),
            year: now.year(),
            org_id: org_id.to_owned(),
            request_body: request_body.to_owned(),
            size: stats.size,
            unit: "MB".to_owned(),
            user_email: "".to_owned(),
            response_time: stats.response_time,
            num_records: stats.records,
            stream_type,
            stream_name: stream_name.to_owned(),
            min_ts: None,
            max_ts: None,
            compressed_size: None,
        });
    };
    if !usage.is_empty() {
        publish_usage(usage).await;
    }
}

pub async fn report_compression_stats(
    stats: RequestStats,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
) {
    if !CONFIG.common.usage_enabled {
        return;
    }
    if CONFIG.common.usage_org.eq(org_id)
        && (stream_name.eq(STATS_STREAM) || stream_name.eq(USAGE_STREAM))
    {
        return;
    }
    let now = Utc::now();
    let usage = vec![UsageData {
        event: UsageEvent::Ingestion,
        day: now.day(),
        hour: now.hour(),
        month: now.month(),
        year: now.year(),
        org_id: org_id.to_owned(),
        request_body: "".to_owned(),
        size: stats.size,
        unit: "MB".to_owned(),
        user_email: "".to_owned(),
        response_time: 0.0,
        num_records: stats.records,
        stream_type,
        stream_name: stream_name.to_owned(),
        min_ts: stats.min_ts,
        max_ts: stats.max_ts,
        compressed_size: stats.compressed_size,
    }];

    if !usage.is_empty() {
        publish_usage(usage).await;
    }
}

pub async fn publish_usage(mut usage: Vec<UsageData>) {
    let mut usages = USAGE_DATA.write().await;
    usages.append(&mut usage);

    if usages.len() >= CONFIG.common.usage_batch_size {
        let mut curr_usage = std::mem::take(&mut *usages);

        let mut groups: AHashMap<GroupKey, AggregatedData> = AHashMap::new();

        for usage_data in &curr_usage {
            let key = GroupKey {
                stream_name: usage_data.stream_name.clone(),
                org_id: usage_data.org_id.clone(),
                stream_type: usage_data.stream_type,
                day: usage_data.day,
                hour: usage_data.hour,
                event: usage_data.event,
            };

            let is_new = groups.contains_key(&key);

            let entry = groups.entry(key).or_insert_with(|| AggregatedData {
                count: 1,
                usage_data: usage_data.clone(),
            });
            if !is_new {
                continue;
            } else {
                entry.usage_data.num_records += usage_data.num_records;
                entry.usage_data.size += usage_data.size;
                entry.usage_data.response_time += usage_data.response_time;
                entry.count += 1;
            }
        }

        let mut report_data = vec![];
        for (_, data) in groups {
            let mut usage_data = data.usage_data;
            usage_data.response_time /= data.count as f64;
            report_data.push(json::to_value(usage_data).unwrap());
        }

        let req = crate::handler::grpc::cluster_rpc::UsageRequest {
            usage_list: Some(UsageDataList::from(report_data)),
            stream_name: USAGE_STREAM.to_owned(),
        };

        match ingestion_service::ingest(&CONFIG.common.usage_org, req).await {
            Ok(_) => {}
            Err(err) => {
                log::error!("Error in ingesting usage data {:?}", err);
                // on error in ingesting usage data, push back the data
                usages.append(&mut curr_usage);
            }
        }
    }
}
