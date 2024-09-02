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

use arrow::array::RecordBatch;
use async_recursion::async_recursion;
use config::{get_config, meta::search::ScanStats};
use datafusion::{common::tree_node::TreeNode, physical_plan::displayable};
use hashbrown::HashMap;
use infra::errors::{Error, Result};
use o2_enterprise::enterprise::super_cluster::search::get_cluster_nodes;
use proto::cluster_rpc::{self};
use tracing::{info_span, Instrument};

use crate::service::search::{
    cluster::flight::{generate_context, register_table},
    datafusion::distributed_plan::{remote_scan::RemoteScanExec, rewrite::RemoteScanRewriter},
    new_sql::NewSql,
    request::Request,
};

#[async_recursion]
#[tracing::instrument(
    name = "service:search:flight:super_cluster_leader",
    skip_all,
    fields(org_id = req.org_id)
)]
pub async fn search(
    trace_id: &str,
    sql: Arc<NewSql>,
    mut req: Request,
    req_regions: Vec<String>,
    req_clusters: Vec<String>,
) -> Result<(Vec<RecordBatch>, ScanStats, usize, bool, usize)> {
    let _start = std::time::Instant::now();
    let cfg = get_config();
    log::info!("[trace_id {trace_id}] flight->leader: start {}", sql);

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
        return Ok((vec![], ScanStats::new(), 0, false, 0));
    }

    // 2. get nodes
    let nodes = get_cluster_nodes(trace_id, req_regions, req_clusters).await?;

    // 4. construct physical plan
    let ctx = match generate_context(&req, &sql, cfg.limit.cpu_num).await {
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    };

    // 5. register table
    register_table(&ctx, &sql).await?;

    // 5. create physical plan
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
            .set_show_schema(false)
            .indent(true)
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
    let mut rewrite = RemoteScanRewriter::new(
        req,
        nodes,
        HashMap::new(),
        partition_keys,
        match_all_keys,
        true,
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
            rewrite.file_lists.get(table_name).unwrap().clone(),
            rewrite
                .equal_keys
                .get(table_name)
                .cloned()
                .unwrap_or_default(),
            rewrite.match_all_keys.clone(),
            false,
            rewrite.req,
            rewrite.nodes,
        ));
    }

    if cfg.common.print_key_sql {
        let plan = displayable(physical_plan.as_ref())
            .set_show_schema(false)
            .indent(true)
            .to_string();
        println!("+---------------------------+----------+");
        println!("leader physical plan after rewrite");
        println!("+---------------------------+----------+");
        println!("{}", plan);
    }

    let datafusion_span = info_span!(
        "service:search:flight:super_cluster::datafusion",
        org_id = sql.org_id,
        stream_name = sql.stream_names.first().unwrap(),
        stream_type = sql.stream_type.to_string(),
    );

    let trace_id2 = trace_id.to_owned();
    let task = tokio::task::spawn(
        async move {
            tokio::select! {
                ret = datafusion::physical_plan::collect(physical_plan, ctx.task_ctx()) => {
                    match ret {
                        Ok(ret) => Ok(ret),
                        Err(err) => {
                            log::error!("[trace_id {trace_id2}] flight->leader: datafusion execute error: {}", err); 
                            Err(err)
                        }
                    }
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(timeout)) => {
                    log::error!("[trace_id {trace_id2}] flight->leader: search timeout");
                    Err(datafusion::error::DataFusionError::ResourcesExhausted(format!("[trace_id {trace_id2}] flight->leader: search timeout")))
                },
            }
        }
        .instrument(datafusion_span),
    );

    let data = match task.await {
        Ok(Ok(data)) => Ok(data),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(Error::Message(err.to_string())),
    };
    let data = match data {
        Ok(v) => v,
        Err(e) => {
            return Err(e);
        }
    };

    log::info!("[trace_id {trace_id}] flight->leader: search finished");

    let scan_stats = ScanStats::new();
    Ok((data, scan_stats, 0, false, 0))
}
