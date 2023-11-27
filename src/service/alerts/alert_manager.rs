// Copyright 2023 Zinc Labs Inc.
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

use chrono::Utc;
use tokio::time;

use crate::common::infra::config::{TRIGGERS, TRIGGERS_IN_PROCESS};
use crate::common::meta::alerts::{Trigger, TriggerTimer};

pub async fn run() -> Result<(), anyhow::Error> {
    for trigger in TRIGGERS.iter() {
        if !trigger.is_ingest_time && !trigger.parent_alert_deleted {
            let trigger = trigger.clone();
            tokio::task::spawn(async move { handle_triggers(trigger).await });
        }
    }
    Ok(())
}

pub async fn handle_triggers(trigger: Trigger) {
    match super::db::alerts::get(
        &trigger.org,
        &trigger.stream,
        trigger.stream_type,
        &trigger.alert_name,
    )
    .await
    {
        Err(_) => {
            let trigger_key = format!("{}/{}", &trigger.org, &trigger.alert_name);
            let mut local_trigger = trigger;
            local_trigger.parent_alert_deleted = true;
            let _ = crate::service::db::alerts::triggers::set(&trigger_key, &local_trigger).await;
        }
        Ok(result) => {
            let key = format!("{}/{}", &trigger.org, &trigger.alert_name);
            if let Some(alert) = result {
                if TRIGGERS_IN_PROCESS.clone().contains_key(&key) {
                    let mut curr_time = TRIGGERS_IN_PROCESS.get_mut(&key).unwrap();
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
                        Utc::now().timestamp_micros() + get_micros_from_min(alert.period); // * 60 * 1000000;
                    log::info!("Setting timeout for trigger to {}", expires_at);
                    TRIGGERS_IN_PROCESS.insert(
                        key.to_owned(),
                        TriggerTimer {
                            updated_at: trigger.timestamp,
                            expires_at,
                        },
                    );
                    handle_trigger(&key, alert.frequency).await;
                }
            }
        }
    }
}

async fn handle_trigger(alert_key: &str, frequency: i64) {
    let mut interval = time::interval(time::Duration::from_secs((frequency * 60) as _));

    loop {
        interval.tick().await;
        let loc_triggers = TRIGGERS.clone();
        let trigger = match loc_triggers.get(&alert_key.to_owned()) {
            Some(trigger) => trigger,
            None => {
                log::info!("Trigger {} not found", alert_key);
                break;
            }
        };
        if !trigger.parent_alert_deleted && TRIGGERS_IN_PROCESS.clone().contains_key(alert_key) {
            let _ = super::db::alerts::get(
                &trigger.org,
                &trigger.stream,
                trigger.stream_type,
                &trigger.alert_name,
            )
            .await;

            // match alert_resp.unwrap_or(None) {
            //     Some(alert) => {
            //         let mut query = alert.query.clone().unwrap();
            //         let curr_ts = Utc::now().timestamp_micros();
            //         query.end_time = curr_ts;
            //         query.start_time = curr_ts - get_micros_from_min(alert.duration);
            //         let req: meta::search::Request = Request {
            //             query,
            //             aggs: HashMap::new(),
            //             encoding: meta::search::RequestEncoding::Empty,
            //             timeout: 0,
            //         };
            //         // do search
            //         match SearchService::search("", &trigger.org, alert.stream_type.unwrap(), &req)
            //             .await
            //         {
            //             Ok(res) => {
            //                 if !res.hits.is_empty() {
            //                     let record = res.hits.first().unwrap().as_object().unwrap();
            //                     if alert.condition.evaluate(record.clone()) {
            //                         let curr_ts = Utc::now().timestamp_micros();
            //                         let mut local_trigger = trigger.clone();

            //                         if trigger.last_sent_at == 0
            //                             || (trigger.last_sent_at > 0
            //                                 && curr_ts - trigger.last_sent_at
            //                                     > get_micros_from_min(alert.time_between_alerts))
            //                         {
            //                             let _ = send_notification(&alert, &trigger).await;
            //                             local_trigger.last_sent_at = curr_ts;
            //                         }
            //                         //Update trigger for last sent

            //                         local_trigger.count += 1;
            //                         let _ =
            //                             triggers::save_trigger(&alert.name, &local_trigger).await;
            //                     }
            //                 }
            //             }
            //             Err(err) => {
            //                 log::error!("alert_manager search error: {:?}", err);
            //             }
            //         }
            //     }
            //     None => log::error!("Error fetching alert "),
            // }
        }
    }
}

fn get_micros_from_min(min: i64) -> i64 {
    min * 60 * 1000000
}
