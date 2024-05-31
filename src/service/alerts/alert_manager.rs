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

use std::str::FromStr;

use chrono::{Duration, FixedOffset, Utc};
use config::{
    get_config,
    meta::{
        stream::StreamType,
        usage::{TriggerData, TriggerDataStatus, TriggerDataType},
    },
};
use cron::Schedule;

use crate::{
    common::meta::{
        alerts::AlertFrequencyType, dashboards::reports::ReportFrequencyType,
        synthetics::SyntheticsFrequencyType,
    },
    service::{db, usage::publish_triggers_usage},
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

    log::debug!("Pulled {} jobs from scheduler", triggers.len());

    for trigger in triggers {
        tokio::task::spawn(async move {
            if let Err(e) = handle_triggers(trigger).await {
                log::error!("[ALERT_MANAGER] Error handling trigger: {}", e);
            }
        });
    }
    Ok(())
}

pub async fn handle_triggers(trigger: db::scheduler::Trigger) -> Result<(), anyhow::Error> {
    match trigger.module {
        db::scheduler::TriggerModule::Report => handle_report_triggers(trigger).await,
        db::scheduler::TriggerModule::Alert => handle_alert_triggers(trigger).await,
        db::scheduler::TriggerModule::Synthetics => handle_synthetics_triggers(trigger).await,
    }
}

async fn handle_alert_triggers(trigger: db::scheduler::Trigger) -> Result<(), anyhow::Error> {
    log::debug!(
        "Inside handle_alert_triggers: processing trigger: {}",
        &trigger.module_key
    );
    let columns = trigger.module_key.split('/').collect::<Vec<&str>>();
    assert_eq!(columns.len(), 3);
    let org_id = &trigger.org;
    let stream_type: StreamType = columns[0].into();
    let stream_name = columns[1];
    let alert_name = columns[2];
    let is_realtime = trigger.is_realtime;
    let is_silenced = trigger.is_silenced;

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

    let alert = match super::get(org_id, stream_type, stream_name, alert_name).await? {
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

    let mut new_trigger = db::scheduler::Trigger {
        next_run_at: Utc::now().timestamp_micros(),
        is_realtime: false,
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

    // evaluate alert
    let ret = alert.evaluate(None).await?;
    if ret.is_some() && alert.trigger_condition.silence > 0 {
        new_trigger.next_run_at += Duration::try_minutes(alert.trigger_condition.silence)
            .unwrap()
            .num_microseconds()
            .unwrap();
        new_trigger.is_silenced = true;
    } else if alert.trigger_condition.frequency_type == AlertFrequencyType::Cron {
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

    let mut trigger_data_stream = TriggerData {
        org: trigger.org,
        module: TriggerDataType::Alert,
        key: trigger.module_key.clone(),
        next_run_at: new_trigger.next_run_at,
        is_realtime: trigger.is_realtime,
        is_silenced: trigger.is_silenced,
        status: TriggerDataStatus::Completed,
        start_time: trigger.start_time.unwrap_or_default(),
        end_time: trigger.end_time.unwrap_or_default(),
        retries: trigger.retries,
        error: None,
    };

    // send notification
    if let Some(data) = ret {
        match alert.send_notification(&data).await {
            Ok(_) => {
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
                trigger_data_stream.error =
                    Some(format!("error sending notification for alert: {e}"));
            }
        }
    } else {
        log::debug!(
            "Alert conditions not satisfied, org: {}, module_key: {}",
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

    let mut trigger_data_stream = TriggerData {
        org: trigger.org.clone(),
        module: TriggerDataType::Report,
        key: trigger.module_key.clone(),
        next_run_at: new_trigger.next_run_at,
        is_realtime: trigger.is_realtime,
        is_silenced: trigger.is_silenced,
        status: TriggerDataStatus::Completed,
        start_time: trigger.start_time.unwrap_or_default(),
        end_time: trigger.end_time.unwrap_or_default(),
        retries: trigger.retries,
        error: None,
    };

    let now = Utc::now().timestamp_micros();
    match report.send_subscribers().await {
        Ok(_) => {
            log::debug!("Report send_subscribers done, report: {}", report_name);
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

    report.last_triggered_at = Some(now);
    // Check if the report has been disabled in the mean time
    let old_report = db::dashboards::reports::get(org_id, report_name).await?;
    if !old_report.enabled {
        report.enabled = old_report.enabled;
    }
    let result = db::dashboards::reports::set_without_updating_trigger(org_id, &report).await;
    if result.is_err() {
        log::error!(
            "Failed to update report: {report_name} after trigger: {}",
            result.err().unwrap()
        );
    }
    publish_triggers_usage(trigger_data_stream).await;

    Ok(())
}

pub async fn handle_synthetics_triggers(
    trigger: db::scheduler::Trigger,
) -> Result<(), anyhow::Error> {
    log::info!(
        "Handle synthetics trigger called for key: {}",
        &trigger.module_key
    );

    let org_id = &trigger.org;
    // For synthetics, trigger.module_key is the synthetics name
    let synthetics_name = &trigger.module_key;

    let mut synthetics = db::synthetics::get(org_id, synthetics_name).await?;

    let mut new_trigger = db::scheduler::Trigger {
        next_run_at: Utc::now().timestamp_micros(),
        is_realtime: false,
        is_silenced: false,
        status: db::scheduler::TriggerStatus::Waiting,
        retries: 0,
        ..trigger.clone()
    };

    if !synthetics.enabled {
        log::debug!(
            "Report not enabled: org: {}, report: {}",
            org_id,
            synthetics_name
        );
        // update trigger, check on next week
        new_trigger.next_run_at += Duration::try_days(7).unwrap().num_microseconds().unwrap();
        db::scheduler::update_trigger(new_trigger).await?;
        return Ok(());
    }
    let mut run_once = false;

    // Update trigger, set `next_run_at` to the
    // frequency interval of this synthetics
    match synthetics.schedule.frequency_type {
        SyntheticsFrequencyType::Seconds => {
            new_trigger.next_run_at += Duration::try_seconds(synthetics.schedule.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        SyntheticsFrequencyType::Minutes => {
            new_trigger.next_run_at += Duration::try_minutes(synthetics.schedule.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        SyntheticsFrequencyType::Hours => {
            new_trigger.next_run_at += Duration::try_hours(synthetics.schedule.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        SyntheticsFrequencyType::Days => {
            new_trigger.next_run_at += Duration::try_days(synthetics.schedule.interval)
                .unwrap()
                .num_microseconds()
                .unwrap();
        }
        SyntheticsFrequencyType::Once => {
            // Check on next week
            new_trigger.next_run_at += Duration::try_days(7).unwrap().num_microseconds().unwrap();
            // Disable the report
            synthetics.enabled = false;
            run_once = true;
        }
        SyntheticsFrequencyType::Cron => {
            let schedule = Schedule::from_str(&synthetics.schedule.cron)?;
            // tz_offset is in minutes
            let tz_offset =
                FixedOffset::east_opt(synthetics.schedule.timezone_offset * 60).unwrap();
            new_trigger.next_run_at = schedule
                .upcoming(tz_offset)
                .next()
                .unwrap()
                .timestamp_micros();
        }
    }

    let mut trigger_data_stream = TriggerData {
        org: trigger.org.clone(),
        module: TriggerDataType::Synthetics,
        key: trigger.module_key.clone(),
        next_run_at: new_trigger.next_run_at,
        is_realtime: trigger.is_realtime,
        is_silenced: trigger.is_silenced,
        status: TriggerDataStatus::Completed,
        start_time: trigger.start_time.unwrap_or_default(),
        end_time: trigger.end_time.unwrap_or_default(),
        retries: trigger.retries,
        error: None,
    };

    let now = Utc::now().timestamp_micros();
    match synthetics.test_target().await {
        Ok(_) => {
            log::debug!("Synthetics test done for synthetics: {}", synthetics_name);
            // Synthetics test successful, update the trigger
            if run_once {
                new_trigger.status = db::scheduler::TriggerStatus::Completed;
            }
            db::scheduler::update_trigger(new_trigger).await?;
            log::debug!("Update trigger for synthetics: {}", synthetics_name);
            trigger_data_stream.end_time = Utc::now().timestamp_micros();
        }
        Err(e) => {
            log::error!("Failed while testing synthetics target {synthetics_name}: {e}");
            trigger_data_stream.end_time = Utc::now().timestamp_micros();
            trigger_data_stream.status = TriggerDataStatus::Failed;
            trigger_data_stream.error = Some(format!("error processing synthetics: {e}"));
        }
    }
    synthetics.last_triggered_at = Some(now);
    synthetics.last_trigger_status = Some(trigger_data_stream.status.clone());

    if let Err(e) = db::synthetics::set_without_updating_trigger(org_id, &synthetics).await {
        log::error!(
            "Failed to update synthetics: {synthetics_name} after trigger: {}",
            e
        );
    }

    publish_triggers_usage(trigger_data_stream).await;
    Ok(())
}
