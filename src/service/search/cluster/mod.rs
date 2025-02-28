// Copyright 2025 OpenObserve Inc.
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

use ::datafusion::arrow::datatypes::Schema;
use config::{utils::json, TIMESTAMP_COL_NAME};
use hashbrown::HashMap;
#[cfg(feature = "enterprise")]
use {
    super::request::Request,
    config::metrics,
    infra::dist_lock,
    infra::errors::{Error, ErrorCodes, Result},
};

pub mod cache_multi;
pub mod cacher;
pub mod flight;
pub mod http;

#[cfg(feature = "enterprise")]
#[tracing::instrument(name = "work_group:checking", skip_all, fields(user_id = user_id))]
pub async fn work_group_checking(
    trace_id: &str,
    start: std::time::Instant,
    req: &Request,
    work_group: &Option<o2_enterprise::enterprise::search::WorkGroup>,
    locker: &Option<dist_lock::Locker>,
    user_id: Option<&str>,
) -> Result<()> {
    let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
    if super::SEARCH_SERVER
        .insert_sender(trace_id, abort_sender, false)
        .await
        .is_err()
    {
        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[&req.org_id])
            .dec();
        dist_lock::unlock_with_trace_id(trace_id, locker).await?;
        log::warn!("[trace_id {trace_id}] search->cluster: request canceled before enter queue");
        return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!(
            "[trace_id {trace_id}] search->cluster: request canceled before enter queue"
        ))));
    }
    tokio::select! {
        res = work_group_need_wait(trace_id, start, req, work_group, user_id) => {
            match res {
                Ok(_) => {
                    return Ok(());
                },
                Err(e) => {
                    metrics::QUERY_PENDING_NUMS
                        .with_label_values(&[&req.org_id])
                        .dec();
                    dist_lock::unlock_with_trace_id(trace_id, locker).await?;
                    return Err(e);
                }
            }
        }
        _ = abort_receiver => {
            metrics::QUERY_PENDING_NUMS
                .with_label_values(&[&req.org_id])
                .dec();
            dist_lock::unlock_with_trace_id(trace_id, locker).await?;
            log::warn!("[trace_id {trace_id}] search->cluster: waiting in queue was canceled");
            return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!("[trace_id {trace_id}] search->cluster: waiting in queue was canceled"))));
        }
    }
}

#[cfg(feature = "enterprise")]
#[tracing::instrument(name = "work_group:need_wait", skip_all, fields(user_id = user_id))]
pub async fn work_group_need_wait(
    trace_id: &str,
    start: std::time::Instant,
    req: &Request,
    work_group: &Option<o2_enterprise::enterprise::search::WorkGroup>,
    user_id: Option<&str>,
) -> Result<()> {
    let mut log_wait = false;
    loop {
        if start.elapsed().as_millis() as u64 >= req.timeout as u64 * 1000 {
            metrics::QUERY_TIMEOUT_NUMS
                .with_label_values(&[&req.org_id])
                .inc();
            return Err(Error::Message(format!(
                "[trace_id {trace_id}] search: request timeout in queue"
            )));
        }
        if let Some(wg) = work_group.as_ref() {
            match wg.need_wait(user_id).await {
                Ok((true, cur, max)) => {
                    if !log_wait {
                        log::info!(
                            "[trace_id {trace_id}] user: {:?} is waiting in work_group {:?}[{}/{}]",
                            user_id,
                            wg,
                            cur,
                            max
                        );
                        log_wait = true;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Ok((false, cur, max)) => {
                    if log_wait {
                        log::info!(
                            "[trace_id {trace_id}] user: {:?} get approved in work_group  {:?}[{}/{}]",
                            user_id,
                            wg,
                            cur,
                            max
                        );
                    }
                    return Ok(());
                }
                Err(e) => {
                    return Err(Error::Message(e.to_string()));
                }
            }
        }
    }
}

pub fn handle_table_response(
    schema: Arc<Schema>,
    sources: Vec<json::Value>,
) -> (Vec<String>, Vec<json::Value>) {
    let columns = schema
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect::<Vec<_>>();
    let mut table = Vec::with_capacity(sources.len());
    for row in &sources {
        let mut new_row = Vec::with_capacity(columns.len());
        let row = row.as_object().unwrap();
        for column in &columns {
            let value = match row.get(column) {
                Some(v) => v.to_owned(),
                None => json::Value::Null,
            };
            new_row.push(value);
        }
        table.push(json::Value::Array(new_row));
    }
    (columns, table)
}

pub fn handle_metrics_response(sources: Vec<json::Value>) -> Vec<json::Value> {
    // handle metrics response
    let mut results_metrics: HashMap<String, json::Value> = HashMap::with_capacity(16);
    let mut results_values: HashMap<String, Vec<[json::Value; 2]>> = HashMap::with_capacity(16);
    for source in sources {
        let fields = source.as_object().unwrap();
        let mut key = Vec::with_capacity(fields.len());
        fields.iter().for_each(|(k, v)| {
            if *k != TIMESTAMP_COL_NAME && k != "value" {
                key.push(format!("{k}_{v}"));
            }
        });
        let key = key.join("_");
        if !results_metrics.contains_key(&key) {
            let mut fields = fields.clone();
            fields.remove(TIMESTAMP_COL_NAME);
            fields.remove("value");
            results_metrics.insert(key.clone(), json::Value::Object(fields));
        }
        let entry = results_values.entry(key).or_default();
        let value = [
            fields.get(TIMESTAMP_COL_NAME).unwrap().to_owned(),
            json::Value::String(fields.get("value").unwrap().to_string()),
        ];
        entry.push(value);
    }

    let mut new_sources = Vec::with_capacity(results_metrics.len());
    for (key, metrics) in results_metrics {
        new_sources.push(json::json!({
            "metrics": metrics,
            "values": results_values.get(&key).unwrap(),
        }));
    }

    new_sources
}
