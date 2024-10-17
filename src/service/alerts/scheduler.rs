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

use std::str::FromStr;

use chrono::{Duration, FixedOffset, Utc};
use config::{
    get_config,
    meta::{
        stream::StreamType,
        usage::{TriggerData, TriggerDataStatus, TriggerDataType},
    },
    utils::json,
};
use cron::Schedule;
use futures::future::try_join_all;
use proto::cluster_rpc;

use crate::{
    common::meta::{alerts::FrequencyType, dashboards::reports::ReportFrequencyType},
    service::{
        alerts::alert::{get_alert_start_end_time, get_row_column_map},
        db::{self, scheduler::DerivedTriggerData},
        ingestion::ingestion_service,
        usage::publish_triggers_usage,
    },
};

pub async fn run() -> Result<(), anyhow::Error> {
    log::debug!("Pulling jobs from scheduler");
    let cfg = get_config();
    // Scheduler pulls only those triggers that match the conditions-
    // - trigger.next_run_at <= now
    // - !(trigger.is_realtime && !trigger.is_silenced)
    // - trigger.status == "Waiting"
    let triggers = db::scheduler::pull(
        cfg.limit.alert_schedule_concurrency,
        cfg.limit.alert_schedule_timeout,
        cfg.limit.report_schedule_timeout,
    )
    .await?;

    log::info!("Pulled {} jobs from scheduler", triggers.len());

    let mut tasks = Vec::new();
    for trigger in triggers {
        let task = tokio::task::spawn(async move {
            if let Err(e) = handle_triggers(trigger).await {
                log::error!("[SCHEDULER] Error handling trigger: {}", e);
            }
        });
        tasks.push(task);
    }
    if let Err(e) = try_join_all(tasks).await {
        log::error!("[SCHEDULER] Error handling triggers: {}", e);
    }
    Ok(())
}

pub async fn handle_triggers(trigger: db::scheduler::Trigger) -> Result<(), anyhow::Error> {
    match trigger.module {
        db::scheduler::TriggerModule::Report => handle_report_triggers(trigger).await,
        db::scheduler::TriggerModule::Alert => handle_alert_triggers(trigger).await,
        db::scheduler::TriggerModule::DerivedStream => {
            handle_derived_stream_triggers(trigger).await
        }
    }
}

async fn handle_alert_triggers(trigger: db::scheduler::Trigger) -> Result<(), anyhow::Error> {
    log::debug!(
        "Inside handle_alert_triggers: processing trigger: {}",
        &trigger.module_key
    );
    let columns = trigger.module_key.split('/').collect::<Vec<&str>>();
    assert_eq!(columns.len(), 3);
    let org_id = trigger.org.clone();
    let stream_type: StreamType = columns[0].into();
    let stream_name = columns[1];
    let alert_name = columns[2];
    let is_realtime = trigger.is_realtime;
    let is_silenced = trigger.is_silenced;
    let triggered_at = trigger.start_time.unwrap_or_default();

    if is_realtime && is_silenced {
        log::debug!(
            "Realtime alert need wakeup, {}/{}",
            org_id,
            &trigger.module_key
        );
        // wakeup the trigger
        let new_trigger = db::scheduler::Trigger {
            next_run_at: Utc::now().timestamp_micros(),
            is_realtime: true,
            is_silenced: false,
            status: db::scheduler::TriggerStatus::Waiting,
            ..trigger.clone()
        };
        db::scheduler::update_trigger(new_trigger).await?;
        return Ok(());
    }

    let alert = match super::alert::get(&org_id, stream_type, stream_name, alert_name).await? {
        Some(alert) => alert,
        None => {
            return Err(anyhow::anyhow!(
                "alert not found: {}/{}/{}/{}",
                org_id,
                stream_name,
                stream_type,
                alert_name
            ));
        }
    };
    let now = Utc::now().timestamp_micros();
    // The delay in processing the trigger from the time it was supposed to run
    let processing_delay = if trigger.next_run_at == 0 {
        0
    } else {
        now - trigger.next_run_at
    };
    // This is the end time of the last trigger timerange  + 1.
    // This will be used in alert evaluation as the start time.
    // If this is None, alert will use the period to evaluate alert
    let start_time = if trigger.data.is_empty() {
        // approximate the start time involving the alert manager delay
        Some(
            now - Duration::try_minutes(alert.trigger_condition.period)
                .unwrap()
                .num_microseconds()
                .unwrap()
                - processing_delay,
        )
    } else {
        let last_data: Result<DerivedTriggerData, json::Error> = json::from_str(&trigger.data);
        if let Ok(last_data) = last_data {
            Some(last_data.period_end_time + 1)
        } else {
            None
        }
    };

    let mut new_trigger = db::scheduler::Trigger {
        next_run_at: now,
        is_realtime: alert.is_real_time,
        is_silenced: false,
        status: db::scheduler::TriggerStatus::Waiting,
        retries: 0,
        ..trigger.clone()
    };

    if !alert.enabled {
        // update trigger, check on next week
        new_trigger.next_run_at += Duration::try_days(7).unwrap().num_microseconds().unwrap();
        new_trigger.is_silenced = true;
        db::scheduler::update_trigger(new_trigger).await?;
        return Ok(());
    }

    let mut should_store_last_end_time =
        alert.trigger_condition.frequency == (alert.trigger_condition.period * 60);
    let mut trigger_data_stream: TriggerData = TriggerData {
        _timestamp: triggered_at,
        org: trigger.org,
        module: TriggerDataType::Alert,
        key: trigger.module_key.clone(),
        next_run_at: new_trigger.next_run_at,
        is_realtime: trigger.is_realtime,
        is_silenced: trigger.is_silenced,
        status: TriggerDataStatus::Completed,
        start_time: now
            - Duration::try_minutes(alert.trigger_condition.period)
                .unwrap()
                .num_microseconds()
                .unwrap(),
        end_time: now,
        retries: trigger.retries,
        error: None,
        success_response: None,
        is_partial: None,
    };

    // evaluate alert
    let result = alert.evaluate(None, start_time).await;
    if result.is_err() {
        let err = result.err().unwrap();
        trigger_data_stream.status = TriggerDataStatus::Failed;
        let err_string = err.to_string();
        if err_string.starts_with("Partial") {
            trigger_data_stream.is_partial = Some(true);
        }
        trigger_data_stream.error = Some(err_string);
        // update its status and retries
        db::scheduler::update_status(
            &new_trigger.org,
            new_trigger.module,
            &new_trigger.module_key,
            db::scheduler::TriggerStatus::Waiting,
            trigger.retries + 1,
        )
        .await?;
        if trigger.retries + 1 >= get_config().limit.scheduler_max_retries {
            // It has been tried the maximum time, just disable the alert
            // and show the error.
            if let Some(mut alert) =
                super::alert::get(&org_id, stream_type, stream_name, alert_name).await?
            {
                alert.enabled = false;
                if let Err(e) = db::alerts::alert::set_without_updating_trigger(
                    &org_id,
                    stream_type,
                    stream_name,
                    &alert,
                )
                .await
                {
                    log::error!("Failed to update alert: {alert_name} after trigger: {e}",);
                }
            }
        }
        publish_triggers_usage(trigger_data_stream).await;
        return Err(err);
    }

    let (ret, end_time) = result.unwrap();
    if ret.is_some() {
        log::info!(
            "Alert conditions satisfied, org: {}, module_key: {}",
            &new_trigger.org,
            &new_trigger.module_key
        );
    }
    if ret.is_some() && alert.trigger_condition.silence > 0 {
        if alert.trigger_condition.frequency_type == FrequencyType::Cron {
            let schedule = Schedule::from_str(&alert.trigger_condition.cron)?;
            let silence =
                Utc::now() + Duration::try_minutes(alert.trigger_condition.silence).unwrap();
            let silence = silence.with_timezone(
                FixedOffset::east_opt(alert.tz_offset * 60)
                    .as_ref()
                    .unwrap(),
            );
            // Check for the cron timestamp after the silence period
            new_trigger.next_run_at = schedule.after(&silence).next().unwrap().timestamp_micros();
        } else {
            // When the silence period is less than the frequency, the alert runs after the silence
            // period completely ignoring the frequency. So, if frequency is 60 mins and
            // silence is 10 mins, the condition is satisfied, in that case, the alert
            // will run after 10 mins of silence period. To avoid this scenario, we
            // should use the max of (frequency, silence) as the next_run_at.
            // Silence period is in minutes, and the frequency is in seconds.
            let next_run_in_seconds = std::cmp::max(
                alert.trigger_condition.silence * 60,
                alert.trigger_condition.frequency,
            );
            new_trigger.next_run_at += Duration::try_seconds(next_run_in_seconds)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        new_trigger.is_silenced = true;
        // For silence period, no need to store last end time
        should_store_last_end_time = false;
    } else if alert.trigger_condition.frequency_type == FrequencyType::Cron {
        let schedule = Schedule::from_str(&alert.trigger_condition.cron)?;
        // tz_offset is in minutes
        let tz_offset = FixedOffset::east_opt(alert.tz_offset * 60).unwrap();
        new_trigger.next_run_at = schedule
            .upcoming(tz_offset)
            .next()
            .unwrap()
            .timestamp_micros();
    } else {
        new_trigger.next_run_at += Duration::try_seconds(alert.trigger_condition.frequency)
            .unwrap()
            .num_microseconds()
            .unwrap();
    }
    trigger_data_stream.next_run_at = new_trigger.next_run_at;

    let last_satisfied_at = if ret.is_some() {
        Some(triggered_at)
    } else {
        None
    };
    // send notification
    if let Some(data) = ret {
        let vars = get_row_column_map(&data);
        // Multi-time range alerts can have multiple time ranges, hence only
        // use the main start_time (now - period) and end_time (now) for the alert evaluation.
        let use_given_time = alert.query_condition.multi_time_range.is_some()
            && !alert
                .query_condition
                .multi_time_range
                .as_ref()
                .unwrap()
                .is_empty();
        let (alert_start_time, alert_end_time) = get_alert_start_end_time(
            &vars,
            alert.trigger_condition.period,
            end_time,
            start_time,
            use_given_time,
        );
        trigger_data_stream.start_time = alert_start_time;
        trigger_data_stream.end_time = alert_end_time;
        match alert.send_notification(&data, end_time, start_time).await {
            Ok((success_msg, err_msg)) => {
                let success_msg = success_msg.trim().to_owned();
                let err_msg = err_msg.trim().to_owned();
                if !err_msg.is_empty() {
                    log::error!(
                        "Some notifications for alert {}/{} could not be sent: {err_msg}",
                        &new_trigger.org,
                        &new_trigger.module_key
                    );
                    trigger_data_stream.error = Some(err_msg);
                } else {
                    log::info!(
                        "Alert notification sent, org: {}, module_key: {}",
                        &new_trigger.org,
                        &new_trigger.module_key
                    );
                }
                trigger_data_stream.success_response = Some(success_msg);
                // Notification was sent successfully, store the last used end_time in the triggers
                new_trigger.data = if should_store_last_end_time {
                    json::to_string(&DerivedTriggerData {
                        period_end_time: alert_end_time,
                    })
                    .unwrap()
                } else {
                    "".to_string()
                };
                // Notification is already sent to some destinations,
                // hence in case of partial errors, no need to retry
                db::scheduler::update_trigger(new_trigger).await?;
            }
            Err(e) => {
                log::error!(
                    "Error sending alert notification: org: {}, module_key: {}",
                    &new_trigger.org,
                    &new_trigger.module_key
                );
                if trigger.retries + 1 >= get_config().limit.scheduler_max_retries {
                    // It has been tried the maximum time, just update the
                    // next_run_at to the next expected trigger time
                    log::debug!(
                        "This alert trigger: {}/{} has reached maximum retries",
                        &new_trigger.org,
                        &new_trigger.module_key
                    );
                    // Alert could not be sent for multiple times, in the next run
                    // if the same start time used for alert evaluation, the extended
                    // timerange may contain huge amount of data, which may cause issues.
                    // E.g. the alert was supposed to run at 11:00am with period of 30min,
                    // but it could not be sent for multiple times, in the next run at
                    // 11:31am (say), the alert will be checked from 10:30am (as start time
                    // still not changed) to 11:31am. This may create issues if the data is huge.
                    // To avoid that, we need to empty the data. So, in the next run, the period
                    // will be used to evaluate the alert.
                    new_trigger.data = "".to_string();
                    db::scheduler::update_trigger(new_trigger).await?;
                } else {
                    // Otherwise update its status only
                    db::scheduler::update_status(
                        &new_trigger.org,
                        new_trigger.module,
                        &new_trigger.module_key,
                        db::scheduler::TriggerStatus::Waiting,
                        trigger.retries + 1,
                    )
                    .await?;
                    trigger_data_stream.next_run_at = now;
                }
                trigger_data_stream.status = TriggerDataStatus::Failed;
                trigger_data_stream.error =
                    Some(format!("error sending notification for alert: {e}"));
            }
        }
    } else {
        log::info!(
            "Alert conditions not satisfied, org: {}, module_key: {}",
            &new_trigger.org,
            &new_trigger.module_key
        );
        // Condition did not match, store the last used end_time in the triggers
        // In the next run, the alert will be checked from the last end_time
        new_trigger.data = if should_store_last_end_time {
            json::to_string(&DerivedTriggerData {
                period_end_time: end_time,
            })
            .unwrap()
        } else {
            "".to_string()
        };
        db::scheduler::update_trigger(new_trigger).await?;
        trigger_data_stream.start_time = match start_time {
            Some(start_time) => start_time,
            None => {
                end_time
                    - Duration::try_minutes(alert.trigger_condition.period)
                        .unwrap()
                        .num_microseconds()
                        .unwrap()
            }
        };
        trigger_data_stream.end_time = end_time;
        trigger_data_stream.status = TriggerDataStatus::ConditionNotSatisfied;
    }

    // Check if the alert has been disabled in the mean time
    let mut old_alert =
        match super::alert::get(&org_id, stream_type, stream_name, alert_name).await? {
            Some(alert) => alert,
            None => {
                return Err(anyhow::anyhow!(
                    "alert not found: {}/{}/{}/{}",
                    org_id,
                    stream_name,
                    stream_type,
                    alert_name
                ));
            }
        };
    old_alert.last_triggered_at = Some(triggered_at);
    if let Some(last_satisfied_at) = last_satisfied_at {
        old_alert.last_satisfied_at = Some(last_satisfied_at);
    }
    if let Err(e) = db::alerts::alert::set_without_updating_trigger(
        &org_id,
        stream_type,
        stream_name,
        &old_alert,
    )
    .await
    {
        log::error!("Failed to update alert: {alert_name} after trigger: {e}");
    }
    // publish the triggers as stream
    publish_triggers_usage(trigger_data_stream).await;

    Ok(())
}

async fn handle_report_triggers(trigger: db::scheduler::Trigger) -> Result<(), anyhow::Error> {
    log::debug!(
        "Inside handle_report_trigger,org: {}, module_key: {}",
        &trigger.org,
        &trigger.module_key
    );
    let org_id = &trigger.org;
    // For report, trigger.module_key is the report name
    let report_name = &trigger.module_key;

    let mut report = db::dashboards::reports::get(org_id, report_name).await?;
    let mut new_trigger = db::scheduler::Trigger {
        next_run_at: Utc::now().timestamp_micros(),
        is_realtime: false,
        is_silenced: false,
        status: db::scheduler::TriggerStatus::Waiting,
        retries: 0,
        ..trigger.clone()
    };

    if !report.enabled {
        log::debug!(
            "Report not enabled: org: {}, report: {}",
            org_id,
            report_name
        );
        // update trigger, check on next week
        new_trigger.next_run_at += Duration::try_days(7).unwrap().num_microseconds().unwrap();
        db::scheduler::update_trigger(new_trigger).await?;
        return Ok(());
    }
    let mut run_once = false;

    // Update trigger, set `next_run_at` to the
    // frequency interval of this report
    match report.frequency.frequency_type {
        ReportFrequencyType::Hours => {
            new_trigger.next_run_at += Duration::try_hours(report.frequency.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        ReportFrequencyType::Days => {
            new_trigger.next_run_at += Duration::try_days(report.frequency.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        ReportFrequencyType::Weeks => {
            new_trigger.next_run_at += Duration::try_weeks(report.frequency.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        ReportFrequencyType::Months => {
            // Assumes each month to be of 30 days.
            new_trigger.next_run_at += Duration::try_days(report.frequency.interval * 30)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        ReportFrequencyType::Once => {
            // Check on next week
            new_trigger.next_run_at += Duration::try_days(7).unwrap().num_microseconds().unwrap();
            // Disable the report
            report.enabled = false;
            run_once = true;
        }
        ReportFrequencyType::Cron => {
            let schedule = Schedule::from_str(&report.frequency.cron)?;
            // tz_offset is in minutes
            let tz_offset = FixedOffset::east_opt(report.tz_offset * 60).unwrap();
            new_trigger.next_run_at = schedule
                .upcoming(tz_offset)
                .next()
                .unwrap()
                .timestamp_micros();
        }
    }

    let triggered_at = trigger.start_time.unwrap_or_default();
    let mut trigger_data_stream = TriggerData {
        _timestamp: triggered_at,
        org: trigger.org.clone(),
        module: if report.destinations.is_empty() {
            TriggerDataType::CachedReport
        } else {
            TriggerDataType::Report
        },
        key: trigger.module_key.clone(),
        next_run_at: new_trigger.next_run_at,
        is_realtime: trigger.is_realtime,
        is_silenced: trigger.is_silenced,
        status: TriggerDataStatus::Completed,
        start_time: trigger.start_time.unwrap_or_default(),
        end_time: trigger.end_time.unwrap_or_default(),
        retries: trigger.retries,
        error: None,
        success_response: None,
        is_partial: None,
    };

    match report.send_subscribers().await {
        Ok(_) => {
            log::info!("Report {} sent to destination", report_name);
            // Report generation successful, update the trigger
            if run_once {
                new_trigger.status = db::scheduler::TriggerStatus::Completed;
            }
            db::scheduler::update_trigger(new_trigger).await?;
            log::debug!("Update trigger for report: {}", report_name);
            trigger_data_stream.end_time = Utc::now().timestamp_micros();
        }
        Err(e) => {
            log::error!("Error sending report to subscribers: {e}");
            if trigger.retries + 1 >= get_config().limit.scheduler_max_retries && !run_once {
                // It has been tried the maximum time, just update the
                // next_run_at to the next expected trigger time
                log::debug!(
                    "This report trigger: {org_id}/{report_name} has reached maximum possible retries"
                );
                db::scheduler::update_trigger(new_trigger).await?;
            } else {
                if run_once {
                    report.enabled = true;
                }
                // Otherwise update its status only
                db::scheduler::update_status(
                    &new_trigger.org,
                    new_trigger.module,
                    &new_trigger.module_key,
                    db::scheduler::TriggerStatus::Waiting,
                    trigger.retries + 1,
                )
                .await?;
            }
            trigger_data_stream.end_time = Utc::now().timestamp_micros();
            trigger_data_stream.status = TriggerDataStatus::Failed;
            trigger_data_stream.error = Some(format!("error processing report: {e}"));
        }
    }

    // Check if the report has been disabled in the mean time
    let mut old_report = db::dashboards::reports::get(org_id, report_name).await?;
    if old_report.enabled {
        old_report.enabled = report.enabled;
    }
    old_report.last_triggered_at = Some(triggered_at);
    let result = db::dashboards::reports::set_without_updating_trigger(org_id, &old_report).await;
    if result.is_err() {
        log::error!(
            "Failed to update report: {report_name} after trigger: {}",
            result.err().unwrap()
        );
    }
    publish_triggers_usage(trigger_data_stream).await;

    Ok(())
}

async fn handle_derived_stream_triggers(
    trigger: db::scheduler::Trigger,
) -> Result<(), anyhow::Error> {
    log::debug!(
        "Inside handle_derived_stream_triggers processing trigger: {}",
        trigger.module_key
    );

    // module_key format: stream_type/stream_name/pipeline_name/derived_stream_name
    let columns = trigger.module_key.split('/').collect::<Vec<_>>();
    assert_eq!(columns.len(), 4);
    let org_id = &trigger.org;
    let stream_type: StreamType = columns[0].into();
    let stream_name = columns[1];
    let pipeline_name = columns[2];
    let name = columns[3];

    let is_real_time = trigger.is_realtime;
    let is_silenced = trigger.is_silenced;
    if is_real_time && is_silenced {
        log::debug!(
            "Realtime derived_stream needs to wake up, {}/{}",
            org_id,
            trigger.module_key
        );
        let new_trigger = db::scheduler::Trigger {
            next_run_at: Utc::now().timestamp_micros(),
            is_silenced: false,
            status: db::scheduler::TriggerStatus::Waiting,
            ..trigger.clone()
        };
        db::scheduler::update_trigger(new_trigger).await?;
        return Ok(());
    }

    let Ok(pipeline) = db::pipelines::get(org_id, stream_type, stream_name, pipeline_name).await
    else {
        return Err(anyhow::anyhow!(
            "Pipeline associated with trigger not found: {}/{}/{}/{}",
            org_id,
            stream_name,
            stream_type,
            pipeline_name
        ));
    };

    let Some(derived_stream) = pipeline
        .derived_streams
        .and_then(|ds| ds.into_iter().find(|ds| ds.name == name))
    else {
        return Err(anyhow::anyhow!(
            "DerivedStream associated with the trigger not found in pipeline: {}/{}/{}/{}",
            org_id,
            stream_name,
            stream_type,
            name,
        ));
    };
    let start_time = if trigger.data.is_empty() {
        None
    } else {
        let last_data: Result<DerivedTriggerData, json::Error> = json::from_str(&trigger.data);
        if let Ok(last_data) = last_data {
            Some(last_data.period_end_time + 1)
        } else {
            None
        }
    };
    let mut new_trigger = db::scheduler::Trigger {
        next_run_at: Utc::now().timestamp_micros(),
        is_silenced: false,
        status: db::scheduler::TriggerStatus::Waiting,
        retries: 0,
        ..trigger.clone()
    };

    // evaluate trigger and configure trigger next run time
    let (ret, end_time) = derived_stream.evaluate(None, start_time).await?;
    if ret.is_some() {
        log::info!(
            "DerivedStream conditions satisfied, org: {}, module_key: {}",
            new_trigger.org,
            new_trigger.module_key
        );
    }
    // Store the last used derived stream period end time
    new_trigger.data = json::to_string(&DerivedTriggerData {
        period_end_time: end_time,
    })
    .unwrap();
    if ret.is_some() && derived_stream.trigger_condition.silence > 0 {
        if derived_stream.trigger_condition.frequency_type == FrequencyType::Cron {
            let schedule = Schedule::from_str(&derived_stream.trigger_condition.cron)?;
            let silence = Utc::now()
                + Duration::try_minutes(derived_stream.trigger_condition.silence).unwrap();
            let silence = silence.with_timezone(
                FixedOffset::east_opt(derived_stream.tz_offset * 60)
                    .as_ref()
                    .unwrap(),
            );
            // Check for the cron timestamp after the silence period
            new_trigger.next_run_at = schedule.after(&silence).next().unwrap().timestamp_micros();
        } else {
            new_trigger.next_run_at +=
                Duration::try_minutes(derived_stream.trigger_condition.silence)
                    .unwrap()
                    .num_microseconds()
                    .unwrap();
        }
        new_trigger.is_silenced = true;
    } else if derived_stream.trigger_condition.frequency_type == FrequencyType::Cron {
        let schedule = Schedule::from_str(&derived_stream.trigger_condition.cron)?;
        // tz_offset is in minutes
        let tz_offset = FixedOffset::east_opt(derived_stream.tz_offset * 60).unwrap();
        new_trigger.next_run_at = schedule
            .upcoming(tz_offset)
            .next()
            .unwrap()
            .timestamp_micros();
    } else {
        new_trigger.next_run_at +=
            Duration::try_minutes(derived_stream.trigger_condition.frequency)
                .unwrap()
                .num_microseconds()
                .unwrap();
    }

    let mut trigger_data_stream = TriggerData {
        _timestamp: trigger.start_time.unwrap_or_default(),
        org: trigger.org,
        module: TriggerDataType::DerivedStream,
        key: trigger.module_key.clone(),
        next_run_at: new_trigger.next_run_at,
        is_realtime: trigger.is_realtime,
        is_silenced: trigger.is_silenced,
        status: TriggerDataStatus::Completed,
        start_time: if let Some(start_time) = start_time {
            start_time
        } else {
            end_time
                - Duration::try_minutes(derived_stream.trigger_condition.period)
                    .unwrap()
                    .num_microseconds()
                    .unwrap()
        },
        end_time: trigger.end_time.unwrap_or_default(),
        retries: trigger.retries,
        error: None,
        success_response: None,
        is_partial: None,
    };

    // ingest evaluation result into destination
    if let Some(data) = ret {
        let local_val = data
            .into_iter()
            .map(json::Value::Object)
            .collect::<Vec<_>>();
        if local_val.is_empty() {
            log::info!(
                "DerivedStream condition does not match any data for the period, org: {}, module_key: {}",
                &new_trigger.org,
                &new_trigger.module_key
            );
            db::scheduler::update_trigger(new_trigger).await?;
            trigger_data_stream.status = TriggerDataStatus::ConditionNotSatisfied;
        } else {
            // Ingest result into destination stream
            let (org_id, stream_name, stream_type): (String, String, i32) = {
                (
                    derived_stream.destination.org_id.into(),
                    derived_stream.destination.stream_name.into(),
                    cluster_rpc::StreamType::from(derived_stream.destination.stream_type).into(),
                )
            };
            let req = cluster_rpc::IngestionRequest {
                org_id: org_id.clone(),
                stream_name: stream_name.clone(),
                stream_type,
                data: Some(cluster_rpc::IngestionData::from(local_val)),
                ingestion_type: Some(cluster_rpc::IngestionType::Json.into()), /* TODO(taiming): finalize IngestionType for derived_stream */
            };
            match ingestion_service::ingest(&org_id, req).await {
                Ok(resp) if resp.status_code == 200 => {
                    log::info!(
                        "DerivedStream result ingested to destination {org_id}/{stream_name}/{stream_type}",
                    );
                    db::scheduler::update_trigger(new_trigger).await?;
                }
                error => {
                    let err = error.map_or_else(|e| e.to_string(), |resp| resp.message);
                    log::error!(
                        "Error in ingesting DerivedStream result to destination {:?}, org: {}, module_key: {}",
                        err,
                        new_trigger.org,
                        new_trigger.module_key
                    );
                    if trigger.retries + 1 >= get_config().limit.scheduler_max_retries {
                        // It has been tried the maximum time, just update the
                        // next_run_at to the next expected trigger time
                        log::debug!(
                            "This DerivedStream trigger: {}/{} has reached maximum retries",
                            &new_trigger.org,
                            &new_trigger.module_key
                        );
                        db::scheduler::update_trigger(new_trigger).await?;
                    } else {
                        // Otherwise update its status only
                        db::scheduler::update_status(
                            &new_trigger.org,
                            new_trigger.module,
                            &new_trigger.module_key,
                            db::scheduler::TriggerStatus::Waiting,
                            trigger.retries + 1,
                        )
                        .await?;
                    }
                    trigger_data_stream.status = TriggerDataStatus::Failed;
                    trigger_data_stream.error = Some(format!(
                        "error saving enrichment table for DerivedStream: {err}"
                    ));
                }
            }
        }
    } else {
        log::info!(
            "DerivedStream conditions not satisfied, org: {}, module_key: {}",
            &new_trigger.org,
            &new_trigger.module_key
        );
        db::scheduler::update_trigger(new_trigger).await?;
        trigger_data_stream.status = TriggerDataStatus::ConditionNotSatisfied;
    }

    // publish the triggers as stream
    trigger_data_stream.end_time = Utc::now().timestamp_micros();
    publish_triggers_usage(trigger_data_stream).await;

    Ok(())
}
