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

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use config::meta::search::ScanStats;
use datafusion::error::{DataFusionError, Result};
use hashbrown::HashMap;
use promql_parser::parser::EvalStmt;
use tokio::sync::{Mutex, RwLock, Semaphore};

use crate::service::promql::{
    micros, micros_since_epoch, value::*, TableProvider, DEFAULT_LOOKBACK,
};

#[derive(Clone)]
pub struct Query {
    pub org_id: String,
    pub table_provider: Arc<Box<dyn TableProvider>>,
    /// The time boundaries for the evaluation. If start equals end an instant
    /// is evaluated.
    pub start: i64,
    pub end: i64,
    /// Time between two evaluated instants for the range [start:end].
    pub interval: i64,
    /// Default look back from sample search.
    pub lookback_delta: i64,
    /// key — metric name; value — time series data
    pub data_cache: Arc<RwLock<HashMap<String, Value>>>,
    pub scan_stats: Arc<RwLock<ScanStats>>,
    pub timeout: u64, // seconds, query timeout
    pub data_loading: Arc<Mutex<bool>>,
}

impl Query {
    pub fn new<P>(org_id: &str, provider: P, timeout: u64) -> Self
    where
        P: TableProvider,
    {
        let now = micros_since_epoch(SystemTime::now());
        let five_min = micros(DEFAULT_LOOKBACK);
        Self {
            org_id: org_id.to_string(),
            table_provider: Arc::new(Box::new(provider)),
            start: now,
            end: now,
            interval: five_min,
            lookback_delta: five_min,
            data_cache: Arc::new(RwLock::new(HashMap::default())),
            data_loading: Arc::new(Mutex::new(false)),
            scan_stats: Arc::new(RwLock::new(ScanStats::default())),
            timeout,
        }
    }

    #[tracing::instrument(name = "promql:engine:exec", skip_all)]
    pub async fn exec(&mut self, stmt: EvalStmt) -> Result<(Value, Option<String>, ScanStats)> {
        let cfg = config::get_config();
        self.start = micros_since_epoch(stmt.start);
        self.end = micros_since_epoch(stmt.end);
        if stmt.interval > Duration::ZERO {
            self.interval = micros(stmt.interval);
        }
        if stmt.lookback_delta > Duration::ZERO {
            self.lookback_delta = micros(stmt.lookback_delta);
        }

        let ctx = Arc::new(self.clone());
        let expr = Arc::new(stmt.expr);
        let mut result_type: Option<String> = None;

        // range query always be matrix result type.
        if self.start != self.end {
            result_type = Some("matrix".to_string());
        } else {
            // Instant query
            let mut engine = super::Engine::new(ctx, self.start);
            let (mut value, result_type_exec) = engine.exec(&expr).await?;
            if let Value::Float(val) = value {
                value = Value::Sample(Sample::new(self.end, val));
            }
            value.sort();
            if result_type_exec.is_some() {
                result_type = result_type_exec;
            }
            return Ok((value, result_type, *self.scan_stats.read().await));
        }

        // Range query
        // See https://promlabs.com/blog/2020/06/18/the-anatomy-of-a-promql-query/#range-queries
        let mut instant_vectors = Vec::new();
        let mut string_literals = Vec::new();
        let mut tasks = Vec::new();
        let semaphore = std::sync::Arc::new(Semaphore::new(cfg.limit.cpu_num));
        let nr_steps = ((self.end - self.start) / self.interval) + 1;
        for i in 0..nr_steps {
            let time = self.start + (self.interval * i);
            let expr = expr.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let mut engine = super::Engine::new(ctx.clone(), time);
            let task: tokio::task::JoinHandle<Result<(Value, Option<String>)>> =
                tokio::task::spawn(async move {
                    let ret = engine.exec(&expr).await;
                    drop(permit);
                    ret
                });
            tasks.push((time, task));
        }

        for (time, ret) in tasks {
            let (result, result_type_exec) = match ret.await {
                Ok(Ok((value, result_type))) => (value, result_type),
                Ok(Err(e)) => {
                    log::error!("Error executing query engine: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    log::error!("Error executing query task: {}", e);
                    return Err(DataFusionError::Execution(e.to_string()));
                }
            };
            if result_type.is_none() && result_type_exec.is_some() {
                result_type = result_type_exec;
            }
            match result {
                Value::Instant(v) => {
                    instant_vectors.push(RangeValue::new(v.labels.to_owned(), [v.sample]))
                }
                Value::Vector(vs) => instant_vectors.extend(
                    vs.into_iter()
                        .map(|v| RangeValue::new(v.labels.to_owned(), [v.sample])),
                ),
                Value::Range(v) => instant_vectors.push(v),
                Value::Matrix(vs) => instant_vectors.extend(vs),
                Value::Sample(s) => instant_vectors.push(RangeValue::new(Labels::default(), [s])),
                Value::Float(val) => instant_vectors
                    .push(RangeValue::new(Labels::default(), [Sample::new(time, val)])),
                Value::String(val) => string_literals.push(val),
                Value::None => continue,
            };
        }

        if !string_literals.is_empty() {
            let output_str = string_literals.join(", ");
            return Ok((
                Value::String(output_str),
                result_type,
                *self.scan_stats.read().await,
            ));
        }

        // empty result quick return
        if instant_vectors.is_empty() {
            return Ok((Value::None, result_type, *self.scan_stats.read().await));
        }

        // merge data
        let mut merged_data = HashMap::new();
        let mut merged_metrics = HashMap::new();
        for value in instant_vectors {
            merged_data
                .entry(signature(&value.labels))
                .or_insert_with(Vec::new)
                .extend(value.samples);
            merged_metrics.insert(signature(&value.labels), value.labels);
        }
        let merged_data = merged_data
            .into_iter()
            .map(|(sig, samples)| {
                RangeValue::new(merged_metrics.get(&sig).unwrap().to_owned(), samples)
            })
            .collect::<Vec<_>>();

        // sort data
        let mut value = Value::Matrix(merged_data);
        value.sort();
        Ok((value, result_type, *self.scan_stats.read().await))
    }
}
