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

use std::sync::Arc;

use config::{
    get_config,
    meta::{
        self_reporting::{
            error::ErrorData,
            usage::{TriggerData, ERROR_STREAM, TRIGGERS_USAGE_STREAM},
            ReportingData, ReportingMessage, ReportingQueue, ReportingRunner,
        },
        stream::{StreamParams, StreamType},
    },
    utils::json,
};
use once_cell::sync::Lazy;
use tokio::{
    sync::{mpsc, Mutex},
    time,
};

pub(super) static USAGE_QUEUE: Lazy<Arc<ReportingQueue>> =
    Lazy::new(|| Arc::new(initialize_usage_queue()));

pub(super) static ERROR_QUEUE: Lazy<Arc<ReportingQueue>> =
    Lazy::new(|| Arc::new(initialize_error_queue()));

fn initialize_usage_queue() -> ReportingQueue {
    let cfg = get_config();
    let timeout = time::Duration::from_secs(
        cfg.common
            .usage_publish_interval
            .try_into()
            .expect("Env ZO_USAGE_PUBLISH_INTERVAL invalid format. Should be set as integer"),
    );
    let batch_size = cfg.common.usage_batch_size;

    let (msg_sender, msg_receiver) = mpsc::channel::<ReportingMessage>(
        batch_size * std::cmp::max(2, cfg.limit.usage_reporting_thread_num),
    );
    let msg_receiver = Arc::new(Mutex::new(msg_receiver));

    for thread_id in 0..cfg.limit.usage_reporting_thread_num {
        let msg_receiver = msg_receiver.clone();
        tokio::task::spawn(async move {
            self_reporting_ingest_job(thread_id, msg_receiver, batch_size, timeout).await
        });
    }

    ReportingQueue::new(msg_sender)
}

fn initialize_error_queue() -> ReportingQueue {
    let cfg = get_config();
    let timeout = time::Duration::from_secs(
        cfg.common
            .usage_publish_interval
            .try_into()
            .expect("Env ZO_USAGE_PUBLISH_INTERVAL invalid format. Should be set as integer"),
    );
    let batch_size = cfg.common.usage_batch_size;

    let (msg_sender, msg_receiver) = mpsc::channel::<ReportingMessage>(
        batch_size * std::cmp::max(2, cfg.limit.usage_reporting_thread_num),
    );
    let msg_receiver = Arc::new(Mutex::new(msg_receiver));

    for thread_id in 0..cfg.limit.usage_reporting_thread_num {
        let msg_receiver = msg_receiver.clone();
        tokio::task::spawn(async move {
            self_reporting_ingest_job(thread_id, msg_receiver, batch_size, timeout).await
        });
    }

    ReportingQueue::new(msg_sender)
}

async fn self_reporting_ingest_job(
    thread_id: usize,
    msg_receiver: Arc<Mutex<mpsc::Receiver<ReportingMessage>>>,
    batch_size: usize,
    timeout: time::Duration,
) {
    log::debug!("[SELF-REPORTING] thread_{thread_id} starts waiting for reporting messages");
    let mut reporting_runner = ReportingRunner::new(batch_size, timeout);
    let mut interval = time::interval(timeout);

    loop {
        let mut msg_receiver = msg_receiver.lock().await;
        tokio::select! {
            msg = msg_receiver.recv() => {
                match msg {
                    Some(ReportingMessage::Start(start_sender)) => {
                        log::debug!("[SELF-REPORTING] thread_{thread_id} received starting ping");
                        start_sender.send(()).ok();
                    }
                    Some(ReportingMessage::Shutdown(res_sender)) => {
                        log::debug!("[SELF-REPORTING] thread_{thread_id} received shutdown ping");
                        // process any remaining data before shutting down
                        if !reporting_runner.pending.is_empty() {
                            let buffered = reporting_runner.take_batch();
                            ingest_buffered_data(thread_id, buffered).await;
                        }
                        res_sender.send(()).ok();
                        break;
                    }
                    Some(ReportingMessage::Data(reporting_data)) => {
                        reporting_runner.push(reporting_data);
                        if reporting_runner.should_process() {
                            let buffered = reporting_runner.take_batch();
                            ingest_buffered_data(thread_id, buffered).await;
                        }
                    }
                    None => break, // channel closed
                }
            }
            _ = interval.tick() => {
                if reporting_runner.should_process() {
                    let buffered = reporting_runner.take_batch();
                    ingest_buffered_data(thread_id, buffered).await;
                }
            }
        }
    }
}

async fn ingest_buffered_data(thread_id: usize, buffered: Vec<ReportingData>) {
    log::debug!(
        "[SELF-REPORTING] thread_{thread_id} ingests {} buffered data",
        buffered.len()
    );

    let (usages, triggers, errors) = buffered.into_iter().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut usages, mut triggers, mut errors), item| {
            match item {
                ReportingData::Usage(usage) => usages.push(*usage),
                ReportingData::Trigger(trigger) => triggers.push(json::to_value(*trigger).unwrap()),
                ReportingData::Error(error) => errors.push(json::to_value(*error).unwrap()),
            }
            (usages, triggers, errors)
        },
    );

    let cfg = get_config();

    if !usages.is_empty() {
        super::ingestion::ingest_usages(usages).await;
    }

    if !triggers.is_empty() {
        let trigger_stream = StreamParams::new(
            &cfg.common.usage_org,
            TRIGGERS_USAGE_STREAM,
            StreamType::Logs,
        );
        // on error in ingesting usage data, push back the data
        if super::ingestion::ingest_reporting_data(triggers.clone(), trigger_stream)
            .await
            .is_err()
            && &cfg.common.usage_reporting_mode != "both"
        {
            // on error in ingesting usage data, push back the data
            for trigger_json in triggers {
                let trigger: TriggerData = json::from_value(trigger_json).unwrap();
                if let Err(e) = USAGE_QUEUE
                    .enqueue(ReportingData::Trigger(Box::new(trigger)))
                    .await
                {
                    log::error!(
                        "[SELF-REPORTING] Error in pushing back un-ingested UsageData to UsageQueue: {e}"
                    );
                }
            }
        }
    }

    if !errors.is_empty() {
        let error_stream = StreamParams::new(&cfg.common.usage_org, ERROR_STREAM, StreamType::Logs);
        if super::ingestion::ingest_reporting_data(errors.clone(), error_stream)
            .await
            .is_err()
            && &cfg.common.usage_reporting_mode != "both"
        {
            // on error in ingesting usage data, push back the data
            for error_json in errors {
                let error: ErrorData = json::from_value(error_json).unwrap();
                if let Err(e) = ERROR_QUEUE
                    .enqueue(ReportingData::Error(Box::new(error)))
                    .await
                {
                    log::error!(
                        "[SELF-REPORTING] Error in pushing back un-ingested ErrorData to ErrorQueue: {e}"
                    );
                }
            }
        }
    }
}
