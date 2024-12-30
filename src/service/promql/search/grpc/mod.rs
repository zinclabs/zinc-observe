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
    time::{Duration, UNIX_EPOCH},
};

use async_trait::async_trait;
use datafusion::{arrow::datatypes::Schema, error::DataFusionError, prelude::SessionContext};
use infra::{cache::tmpfs, errors::Result};
use promql_parser::parser;
use proto::cluster_rpc;

use crate::service::{
    promql::{value, Query, TableProvider, DEFAULT_LOOKBACK},
    search,
};

mod storage;
mod wal;

struct StorageProvider {
    trace_id: String,
    need_wal: bool,
}

#[async_trait]
impl TableProvider for StorageProvider {
    async fn create_context(
        &self,
        org_id: &str,
        stream_name: &str,
        time_range: (i64, i64),
        filters: &mut [(String, Vec<String>)],
    ) -> datafusion::error::Result<
        Vec<(SessionContext, Arc<Schema>, config::meta::search::ScanStats)>,
    > {
        let mut resp = Vec::new();
        // register storage table
        let trace_id = self.trace_id.to_owned() + "-storage-" + stream_name;
        let ctx =
            storage::create_context(&trace_id, org_id, stream_name, time_range, filters).await?;
        resp.push(ctx);
        // register Wal table
        if self.need_wal {
            let trace_id = self.trace_id.to_owned() + "-wal-" + stream_name;
            let wal_ctx_list =
                wal::create_context(&trace_id, org_id, stream_name, time_range, filters).await?;
            for ctx in wal_ctx_list {
                resp.push(ctx);
            }
        }
        Ok(resp)
    }
}

#[tracing::instrument(name = "promql:search:grpc:search", skip_all, fields(org_id = req.org_id))]
pub async fn search(
    req: &cluster_rpc::MetricsQueryRequest,
) -> Result<cluster_rpc::MetricsQueryResponse> {
    let start = std::time::Instant::now();
    let trace_id = req.job.as_ref().unwrap().trace_id.to_string();

    let org_id = &req.org_id;
    let query = req.query.as_ref().unwrap();
    let prom_expr = parser::parse(&query.query).map_err(|e| {
        log::error!("promQL parse query error: {e}");
        DataFusionError::Execution(e)
    })?;

    let eval_stmt = parser::EvalStmt {
        expr: prom_expr,
        start: UNIX_EPOCH
            .checked_add(Duration::from_micros(query.start as _))
            .unwrap(),
        end: UNIX_EPOCH
            .checked_add(Duration::from_micros(query.end as _))
            .unwrap(),
        interval: Duration::from_micros(query.step as _),
        lookback_delta: DEFAULT_LOOKBACK,
    };

    let timeout = if req.timeout > 0 {
        req.timeout as u64
    } else {
        config::get_config().limit.query_timeout
    };

    let mut engine = Query::new(
        org_id,
        StorageProvider {
            trace_id: trace_id.to_string(),
            need_wal: req.need_wal,
        },
        timeout,
    );

    let (value, result_type, mut scan_stats) = engine.exec(eval_stmt).await?;
    let result_type = match result_type {
        Some(v) => v,
        None => value.get_type().to_string(),
    };

    // clear session
    search::datafusion::storage::file_list::clear(&trace_id);
    // clear tmpfs
    tmpfs::delete(&trace_id, true).unwrap();

    scan_stats.format_to_mb();
    let mut resp = cluster_rpc::MetricsQueryResponse {
        job: req.job.clone(),
        took: start.elapsed().as_millis() as i32,
        result_type,
        scan_stats: Some(cluster_rpc::ScanStats::from(&scan_stats)),
        ..Default::default()
    };

    match value {
        value::Value::None => {
            return Ok(resp);
        }
        value::Value::Instant(v) => {
            resp.result.push(cluster_rpc::Series {
                metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                sample: Some((&v.sample).into()),
                ..Default::default()
            });
        }
        value::Value::Range(v) => {
            resp.result.push(cluster_rpc::Series {
                metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                samples: v.samples.iter().map(|x| x.into()).collect(),
                ..Default::default()
            });
        }
        value::Value::Vector(v) => {
            v.iter().for_each(|v| {
                resp.result.push(cluster_rpc::Series {
                    metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                    sample: Some((&v.sample).into()),
                    ..Default::default()
                });
            });
        }
        value::Value::Matrix(v) => {
            v.iter().for_each(|v| {
                let samples = v
                    .samples
                    .iter()
                    .filter_map(|x| if x.is_nan() { None } else { Some(x.into()) })
                    .collect::<Vec<_>>();
                if !samples.is_empty() {
                    resp.result.push(cluster_rpc::Series {
                        metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                        samples,
                        ..Default::default()
                    });
                }
            });
        }
        value::Value::Sample(v) => {
            resp.result.push(cluster_rpc::Series {
                sample: Some((&v).into()),
                ..Default::default()
            });
        }
        value::Value::Float(v) => {
            resp.result.push(cluster_rpc::Series {
                scalar: Some(v),
                ..Default::default()
            });
        }
        value::Value::String(v) => {
            resp.result.push(cluster_rpc::Series {
                stringliteral: Some(v),
                ..Default::default()
            });
        }
    }

    Ok(resp)
}
