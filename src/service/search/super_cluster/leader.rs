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

use arrow::array::RecordBatch;
use async_recursion::async_recursion;
use config::{
    get_config,
    meta::{cluster::NodeInfo, search::ScanStats},
};
use datafusion::{
    common::{tree_node::TreeNode, DataFusionError},
    physical_plan::{displayable, visit_execution_plan},
};
use hashbrown::HashMap;
use infra::errors::{Error, Result};
use o2_enterprise::enterprise::super_cluster::search::get_cluster_nodes;
use proto::cluster_rpc::{self};
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::service::search::{
    cluster::flight::{generate_context, register_table},
    datafusion::distributed_plan::{remote_scan::RemoteScanExec, rewrite::RemoteScanRewriter},
    request::Request,
    sql::Sql,
    utils::ScanStatsVisitor,
    DATAFUSION_RUNTIME,
};

#[async_recursion]
#[tracing::instrument(
    name = "service:search:flight:super_cluster_leader",
    skip_all,
    fields(org_id = req.org_id)
)]
pub async fn search(
    trace_id: &str,
    sql: Arc<Sql>,
    mut req: Request,
    req_regions: Vec<String>,
    req_clusters: Vec<String>,
) -> Result<(Vec<RecordBatch>, ScanStats, usize, bool, usize, String)> {
    let _start = std::time::Instant::now();
    let cfg = get_config();
    log::info!("[trace_id {trace_id}] super cluster leader: start {}", sql);

    let timeout = if req.timeout > 0 {
        req.timeout as u64
    } else {
        cfg.limit.query_timeout
    };
    req.timeout = timeout as _;

    if sql
        .schemas
        .iter()
        .any(|(_, schema)| schema.schema().fields().is_empty())
    {
        return Ok((vec![], ScanStats::new(), 0, false, 0, "".to_string()));
    }

    let (use_inverted_index, _) = super::super::is_use_inverted_index(&sql);
    req.set_use_inverted_index(use_inverted_index);

    // 2. get nodes
    let nodes = get_cluster_nodes(trace_id, req_regions, req_clusters).await?;

    let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
    if super::super::SEARCH_SERVER
        .insert_sender(trace_id, abort_sender, true)
        .await
        .is_err()
    {
        log::info!(
            "[trace_id {trace_id}] super cluster leader: search canceled before execution plan"
        );
        return Err(Error::ErrorCode(
            infra::errors::ErrorCodes::SearchCancelQuery(format!(
                "[trace_id {trace_id}] super cluster leader: search canceled before execution plan"
            )),
        ));
    }

    let datafusion_span = info_span!(
        "service:search:flight:super_cluster::datafusion",
        org_id = sql.org_id,
        stream_name = sql.stream_names.first().unwrap(),
        stream_type = sql.stream_type.to_string(),
    );

    let trace_id_move = trace_id.to_string();
    let query_task = DATAFUSION_RUNTIME.spawn(async move {
        run_datafusion(trace_id_move, req, sql, nodes)
            .instrument(datafusion_span)
            .await
    });
    tokio::pin!(query_task);

    let task = tokio::select! {
        ret = &mut query_task => {
            match ret {
                Ok(ret) => Ok(ret),
                Err(err) => {
                    log::error!("[trace_id {trace_id}] super cluster leader: datafusion execute error: {}", err);
                    Err(DataFusionError::Execution(err.to_string()))
                }
            }
        },
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(timeout)) => {
            query_task.abort();
            log::error!("[trace_id {trace_id}] super cluster leader: search timeout");
            Err(DataFusionError::ResourcesExhausted("super cluster leader: search timeout".to_string()))
        },
        _ = abort_receiver => {
            query_task.abort();
            log::info!("[trace_id {trace_id}] super cluster leader: search canceled");
            Err(DataFusionError::ResourcesExhausted("super cluster leader: search canceled".to_string()))
        }
    };

    let data = match task {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(Error::Message(err.to_string())),
    };
    let (data, mut scan_stats, partial_err) = match data {
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    };

    log::info!("[trace_id {trace_id}] super cluster leader: search finished");

    scan_stats.format_to_mb();
    Ok((data, scan_stats, 0, !partial_err.is_empty(), 0, partial_err))
}

async fn run_datafusion(
    trace_id: String,
    req: Request,
    sql: Arc<Sql>,
    nodes: Vec<Arc<dyn NodeInfo>>,
) -> Result<(Vec<RecordBatch>, ScanStats, String)> {
    let cfg = get_config();
    // construct physical plan
    let mut ctx = match generate_context(&req, &sql, cfg.limit.cpu_num).await {
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    };

    // register table
    register_table(&ctx, &sql).await?;
    datafusion_functions_json::register_all(&mut ctx)?;

    // create physical plan
    let plan = match ctx.state().create_logical_plan(&sql.sql).await {
        Ok(v) => v,
        Err(e) => {
            return Err(e.into());
        }
    };
    let mut physical_plan = match ctx.state().create_physical_plan(&plan).await {
        Ok(v) => v,
        Err(e) => {
            return Err(e.into());
        }
    };

    if cfg.common.print_key_sql {
        let plan = displayable(physical_plan.as_ref())
            .indent(false)
            .to_string();
        println!("+---------------------------+----------+");
        println!("leader physical plan before rewrite");
        println!("+---------------------------+----------+");
        println!("{}", plan);
    }

    // 6. rewrite physical plan
    let match_all_keys = sql.match_items.clone().unwrap_or_default();
    let partition_keys = sql
        .equal_items
        .iter()
        .map(|(stream_name, fields)| {
            (
                stream_name.clone(),
                fields
                    .iter()
                    .map(|(k, v)| cluster_rpc::KvItem::new(k, v))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    let context = tracing::Span::current().context();
    let mut rewrite = RemoteScanRewriter::new(
        req,
        nodes,
        HashMap::new(),
        Vec::new(),
        partition_keys,
        match_all_keys,
        sql.index_condition.clone(),
        true,
        context,
    );
    physical_plan = match physical_plan.rewrite(&mut rewrite) {
        Ok(v) => v.data,
        Err(e) => {
            return Err(e.into());
        }
    };

    // add remote scan exec to top if physical plan is not changed
    if !rewrite.is_changed {
        let table_name = sql.stream_names.first().unwrap();
        physical_plan = Arc::new(RemoteScanExec::new(
            physical_plan,
            rewrite.remote_scan_nodes.get_remote_node(table_name),
        )?);
    }

    if cfg.common.print_key_sql {
        let plan = displayable(physical_plan.as_ref())
            .indent(false)
            .to_string();
        println!("+---------------------------+----------+");
        println!("leader physical plan after rewrite");
        println!("+---------------------------+----------+");
        println!("{}", plan);
    }

    let ret = datafusion::physical_plan::collect(physical_plan.clone(), ctx.task_ctx()).await;
    let mut visit = ScanStatsVisitor::new();
    let _ = visit_execution_plan(physical_plan.as_ref(), &mut visit);
    if let Err(e) = ret {
        log::error!("[trace_id {trace_id}] super cluster leader: datafusion collect error: {e}");
        Err(e.into())
    } else {
        log::info!("[trace_id {trace_id}] super cluster leader: datafusion collect done");
        ret.map(|data| (data, visit.scan_stats, visit.partial_err))
            .map_err(|e| e.into())
    }
}
