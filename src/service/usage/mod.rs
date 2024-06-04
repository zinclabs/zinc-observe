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

use std::sync::Arc;

use chrono::{Datelike, Timelike, Utc};
use config::{
    get_config,
    meta::{
        stream::StreamType,
        usage::{
            AggregatedData, GroupKey, RequestStats, TriggerData, UsageData, UsageEvent, UsageType,
            TRIGGERS_USAGE_STREAM, USAGE_STREAM,
        },
    },
    metrics,
    utils::json,
    SIZE_IN_MB,
};
use hashbrown::HashMap;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::auditor;
use once_cell::sync::Lazy;
use proto::cluster_rpc;
use reqwest::Client;
use tokio::{sync::RwLock, time};

pub mod ingestion_service;
pub mod stats;

pub static USAGE_DATA: Lazy<Arc<RwLock<Vec<UsageData>>>> =
    Lazy::new(|| Arc::new(RwLock::new(vec![])));
pub static TRIGGERS_USAGE_DATA: Lazy<Arc<RwLock<Vec<TriggerData>>>> =
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
        .inc_by(stats.records as u64);
    metrics::INGEST_BYTES
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .inc_by((stats.size * SIZE_IN_MB) as u64);
    let event: UsageEvent = usage_type.into();

    if !get_config().common.usage_enabled {
        return;
    }

    let request_body = stats.request_body.unwrap_or(usage_type.to_string());
    let user_email = stats.user_email.unwrap_or("".to_owned());
    let now = Utc::now();

    let mut usage = vec![];

    if num_functions > 0 {
        usage.push(UsageData {
            event: UsageEvent::Functions,
            day: now.day(),
            hour: now.hour(),
            month: now.month(),
            year: now.year(),
            event_time_hour: format!(
                "{:04}{:02}{:02}{:02}",
                now.year(),
                now.month(),
                now.day(),
                now.hour()
            ),
            org_id: org_id.to_owned(),
            request_body: request_body.to_owned(),
            size: stats.size,
            unit: "MB".to_owned(),
            user_email: user_email.to_owned(),
            response_time: stats.response_time,
            num_records: stats.records * num_functions as i64,
            stream_type,
            stream_name: stream_name.to_owned(),
            min_ts: None,
            max_ts: None,
            cached_ratio: None,
            compressed_size: None,
            search_type: stats.search_type,
        });
    };

    usage.push(UsageData {
        event,
        day: now.day(),
        hour: now.hour(),
        month: now.month(),
        year: now.year(),
        event_time_hour: format!(
            "{:04}{:02}{:02}{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour()
        ),
        org_id: org_id.to_owned(),
        request_body: request_body.to_owned(),
        size: stats.size,
        unit: "MB".to_owned(),
        user_email,
        response_time: stats.response_time,
        num_records: stats.records,
        stream_type,
        stream_name: stream_name.to_owned(),
        min_ts: stats.min_ts,
        max_ts: stats.max_ts,
        cached_ratio: stats.cached_ratio,
        compressed_size: None,
        search_type: stats.search_type,
    });
    if !usage.is_empty() {
        publish_usage(usage).await;
    }
}

pub async fn publish_usage(mut usage: Vec<UsageData>) {
    let mut usages = USAGE_DATA.write().await;
    usages.append(&mut usage);

    if usages.len() < get_config().common.usage_batch_size {
        return;
    }

    let curr_usages = std::mem::take(&mut *usages);
    // release the write lock
    drop(usages);

    ingest_usages(curr_usages).await
}

pub async fn publish_triggers_usage(trigger: TriggerData) {
    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }

    let mut usages = TRIGGERS_USAGE_DATA.write().await;
    usages.push(trigger);

    if usages.len() < cfg.common.usage_batch_size {
        return;
    }

    let curr_usages = std::mem::take(&mut *usages);
    // release the write lock
    drop(usages);

    ingest_trigger_usages(curr_usages).await
}

pub async fn flush() {
    // flush audit data
    #[cfg(feature = "enterprise")]
    flush_audit().await;

    // flush usage report
    flush_usage().await;
    // flush triggers usage report
    flush_triggers_usage().await;
}

async fn flush_usage() {
    if !get_config().common.usage_enabled {
        return;
    }

    let mut usages = USAGE_DATA.write().await;
    if usages.len() == 0 {
        return;
    }

    let curr_usages = std::mem::take(&mut *usages);
    // release the write lock
    drop(usages);

    ingest_usages(curr_usages).await
}

async fn flush_triggers_usage() {
    if !get_config().common.usage_enabled {
        return;
    }

    let mut usages = TRIGGERS_USAGE_DATA.write().await;
    if usages.len() == 0 {
        return;
    }

    let curr_usages = std::mem::take(&mut *usages);
    // release the write lock
    drop(usages);

    ingest_trigger_usages(curr_usages).await
}

async fn ingest_usages(curr_usages: Vec<UsageData>) {
    if curr_usages.is_empty() {
        log::info!(" Returning as no usages reported ");
        return;
    }
    let mut groups: HashMap<GroupKey, AggregatedData> = HashMap::new();
    let mut search_events = vec![];
    for usage_data in &curr_usages {
        // Skip aggregation for usage_data with event "Search"
        if usage_data.event == UsageEvent::Search {
            search_events.push(usage_data.clone());
            continue;
        }

        let key = GroupKey {
            stream_name: usage_data.stream_name.clone(),
            org_id: usage_data.org_id.clone(),
            stream_type: usage_data.stream_type,
            day: usage_data.day,
            hour: usage_data.hour,
            event: usage_data.event,
            email: usage_data.user_email.clone(),
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
        report_data.push(usage_data);
    }

    // Push all the search events
    report_data.append(&mut search_events);
    let cfg = get_config();
    if &cfg.common.usage_reporting_mode != "local"
        && !cfg.common.usage_reporting_url.is_empty()
        && !cfg.common.usage_reporting_creds.is_empty()
    {
        let url = url::Url::parse(&cfg.common.usage_reporting_url).unwrap();
        let creds = if cfg.common.usage_reporting_creds.starts_with("Basic") {
            cfg.common.usage_reporting_creds.to_string()
        } else {
            format!("Basic {}", &cfg.common.usage_reporting_creds)
        };
        if let Err(e) = Client::builder()
            .build()
            .unwrap()
            .post(url)
            .header("Content-Type", "application/json")
            .header(reqwest::header::AUTHORIZATION, creds)
            .json(&report_data)
            .send()
            .await
        {
            log::error!("Error in ingesting usage data to external URL {:?}", e);
            if &cfg.common.usage_reporting_mode != "both" {
                // on error in ingesting usage data, push back the data
                let mut usages = USAGE_DATA.write().await;
                let mut curr_usages = curr_usages.clone();
                usages.append(&mut curr_usages);
                drop(usages);
            }
        }
    }

    if &cfg.common.usage_reporting_mode != "remote" {
        let report_data = report_data
            .iter_mut()
            .map(|usage| json::to_value(usage).unwrap())
            .collect::<Vec<_>>();
        // report usage data
        let req = cluster_rpc::UsageRequest {
            stream_name: USAGE_STREAM.to_owned(),
            data: Some(cluster_rpc::UsageData::from(report_data)),
        };
        if let Err(e) = ingestion_service::ingest(&cfg.common.usage_org, req).await {
            log::error!("Error in ingesting usage data {:?}", e);
            // on error in ingesting usage data, push back the data
            let mut usages = USAGE_DATA.write().await;
            let mut curr_usages = curr_usages.clone();
            usages.append(&mut curr_usages);
            drop(usages);
        }
    }
}

async fn ingest_trigger_usages(curr_usages: Vec<TriggerData>) {
    if curr_usages.is_empty() {
        log::info!(" Returning as no triggers reported ");
        return;
    }

    let mut json_triggers = vec![];
    for trigger_data in &curr_usages {
        json_triggers.push(json::to_value(trigger_data).unwrap());
    }

    // report trigger usage data
    let req = cluster_rpc::UsageRequest {
        stream_name: TRIGGERS_USAGE_STREAM.to_owned(),
        data: Some(cluster_rpc::UsageData::from(json_triggers)),
    };
    if let Err(e) = ingestion_service::ingest(&get_config().common.usage_org, req).await {
        log::error!("Error in ingesting triggers usage data {:?}", e);
        // on error in ingesting usage data, push back the data
        let mut usages = TRIGGERS_USAGE_DATA.write().await;
        let mut curr_usages = curr_usages.clone();
        usages.append(&mut curr_usages);
        drop(usages);
    }
}

async fn publish_existing_usage() {
    let mut usages = USAGE_DATA.write().await;
    log::debug!("publishing usage reports,len: {}", usages.len());
    if usages.is_empty() {
        return;
    }

    let curr_usages = std::mem::take(&mut *usages);
    // release the write lock
    drop(usages);

    ingest_usages(curr_usages).await
}

async fn publish_existing_triggers_usage() {
    let mut usages = TRIGGERS_USAGE_DATA.write().await;
    log::debug!("publishing triggers usage reports,len: {}", usages.len());

    if usages.is_empty() {
        return;
    }

    let curr_usages = std::mem::take(&mut *usages);
    // release the write lock
    drop(usages);

    ingest_trigger_usages(curr_usages).await
}

pub async fn run() {
    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }
    let mut usage_interval = time::interval(time::Duration::from_secs(
        cfg.common.usage_publish_interval.try_into().unwrap(),
    ));
    usage_interval.tick().await; // trigger the first run
    loop {
        log::debug!("Usage ingestion loop running");
        usage_interval.tick().await;
        publish_existing_usage().await;
        publish_existing_triggers_usage().await;
    }
}

#[cfg(feature = "enterprise")]
pub async fn audit(msg: auditor::AuditMessage) {
    auditor::audit(msg, publish_audit).await;
}

#[cfg(feature = "enterprise")]
pub async fn flush_audit() {
    auditor::flush_audit(publish_audit).await;
}

#[cfg(feature = "enterprise")]
async fn publish_audit(
    req: cluster_rpc::UsageRequest,
) -> Result<cluster_rpc::UsageResponse, anyhow::Error> {
    ingestion_service::ingest(&get_config().common.usage_org, req).await
}
