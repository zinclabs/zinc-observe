// Copyright 2022 Zinc Labs Inc. and Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::Utc;
use std::collections::HashMap;
use tokio::time;

use crate::common::notification::send_notification;
use crate::infra::config::{TRIGGERS, TRIGGERS_IN_PROCESS};
use crate::meta;
use crate::meta::alert::{Evaluate, Trigger, TriggerTimer};
use crate::meta::search::Request;
use crate::service::search as SearchService;
use crate::service::triggers;

#[cfg_attr(coverage_nightly, no_coverage)]
pub async fn run() -> Result<(), anyhow::Error> {
    for trigger in TRIGGERS.iter() {
        if !trigger.is_ingest_time {
            let trigger = trigger.clone();
            tokio::task::spawn(async move { handle_triggers(trigger).await });
        }
    }
    Ok(())
}

#[cfg_attr(coverage_nightly, no_coverage)]
pub async fn handle_triggers(trigger: Trigger) {
    match super::db::alerts::get(
        &trigger.org,
        &trigger.stream,
        trigger.stream_type,
        &trigger.alert_name,
    )
    .await
    {
        Err(_) => log::error!("[ALERT MANAGER] Error fetching alert"),
        Ok(result) => {
            if let Some(alert) = result {
                if TRIGGERS_IN_PROCESS
                    .clone()
                    .contains_key(&trigger.alert_name)
                {
                    let mut curr_time = TRIGGERS_IN_PROCESS.get_mut(&trigger.alert_name).unwrap();
                    let delay = trigger.timestamp - curr_time.updated_at;
                    if delay > 0 {
                        log::info!(
                            "Updating timeout for trigger to {}",
                            curr_time.expires_at + delay
                        );
                        curr_time.expires_at += delay;
                        curr_time.updated_at = trigger.timestamp;
                    }
                } else {
                    let expires_at =
                        Utc::now().timestamp_micros() + get_micros_from_min(alert.duration); // * 60 * 1000000;
                    log::info!("Setting timeout for trigger to {}", expires_at);
                    TRIGGERS_IN_PROCESS.insert(
                        trigger.alert_name.clone(),
                        TriggerTimer {
                            updated_at: trigger.timestamp,
                            expires_at,
                        },
                    );
                    handle_trigger(&trigger.alert_name, alert.frequency).await;
                }
            }
        }
    }
}

#[cfg_attr(coverage_nightly, no_coverage)]
async fn handle_trigger(alert_name: &str, frequency: i64) {
    let mut interval = time::interval(time::Duration::from_secs((frequency * 60) as _));

    loop {
        interval.tick().await;
        let loc_triggers = TRIGGERS.clone();
        let trigger = loc_triggers.get(&alert_name.to_owned()).unwrap();
        if TRIGGERS_IN_PROCESS.clone().contains_key(alert_name) {
            let alert_resp = super::db::alerts::get(
                &trigger.org,
                &trigger.stream,
                trigger.stream_type,
                &trigger.alert_name,
            )
            .await;

            match alert_resp.unwrap_or(None) {
                Some(alert) => {
                    let mut query = alert.query.clone().unwrap();
                    let curr_ts = Utc::now().timestamp_micros();
                    query.end_time = curr_ts;
                    query.start_time = curr_ts - get_micros_from_min(alert.duration);
                    let req: meta::search::Request = Request {
                        query,
                        aggs: HashMap::new(),
                        encoding: meta::search::RequestEncoding::Empty,
                    };
                    // do search
                    match SearchService::search_for_http(
                        &trigger.org,
                        alert.stream_type.unwrap(),
                        &req,
                    )
                    .await
                    {
                        Ok(res) => {
                            if !res.hits.is_empty() {
                                let record = res.hits.first().unwrap().as_object().unwrap();
                                if alert.condition.clone().evaluate(record.clone()) {
                                    let curr_ts = Utc::now().timestamp_micros();
                                    let mut local_trigger = trigger.clone();

                                    if trigger.clone().last_sent_at == 0
                                        || (trigger.clone().last_sent_at > 0
                                            && curr_ts - trigger.clone().last_sent_at
                                                > get_micros_from_min(alert.time_between_alerts))
                                    {
                                        let _ = send_notification(&alert, &trigger.clone()).await;
                                        local_trigger.last_sent_at = curr_ts;
                                    }
                                    //Update trigger for last sent

                                    local_trigger.count += 1;
                                    let _ = triggers::save_trigger(
                                        alert.name.clone(),
                                        local_trigger.clone(),
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(err) => {
                            log::error!("alert_manager search error: {:?}", err);
                        }
                    }
                }
                None => log::error!("Error fetching alert "),
            }
        }
    }
}

fn get_micros_from_min(min: i64) -> i64 {
    min * 60 * 1000000
}
