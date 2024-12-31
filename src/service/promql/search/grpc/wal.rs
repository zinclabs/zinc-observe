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

use std::{collections::HashSet, sync::Arc};

use arrow::record_batch::RecordBatch;
use config::{
    get_config,
    meta::{cluster::IntoArcVec, search::ScanStats, stream::StreamType},
    utils::record_batch_ext::RecordBatchExt,
};
use datafusion::{
    arrow::datatypes::Schema,
    datasource::MemTable,
    error::{DataFusionError, Result},
    physical_plan::visit_execution_plan,
    prelude::{col, lit, SessionContext},
};
use promql_parser::label::Matchers;
use proto::cluster_rpc::{self, IndexInfo, QueryIdentifier};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
    common::infra::cluster::get_cached_online_ingester_nodes,
    service::{
        promql::utils::{apply_label_selector, apply_matchers},
        search::{
            cluster::flight::print_plan,
            datafusion::{
                distributed_plan::{
                    node::{RemoteScanNode, SearchInfos},
                    remote_scan::RemoteScanExec,
                },
                exec::prepare_datafusion_context,
                table_provider::empty_table::NewEmptyTable,
            },
            grpc::wal::adapt_batch,
            utils::ScanStatsVisitor,
        },
    },
};

#[tracing::instrument(name = "promql:search:grpc:wal:create_context", skip(trace_id))]
pub(crate) async fn create_context(
    trace_id: &str,
    org_id: &str,
    stream_name: &str,
    time_range: (i64, i64),
    matchers: Matchers,
    label_selector: Option<HashSet<String>>,
) -> Result<Vec<(SessionContext, Arc<Schema>, ScanStats)>> {
    let mut resp = vec![];
    // fetch all schema versions, get latest schema
    let schema = Arc::new(
        infra::schema::get(org_id, stream_name, StreamType::Metrics)
            .await
            .map_err(|err| {
                log::error!("[trace_id {trace_id}] get schema error: {}", err);
                DataFusionError::Execution(err.to_string())
            })?,
    );

    // get file list
    let batches = get_file_list(
        trace_id,
        org_id,
        stream_name,
        Arc::clone(&schema),
        time_range,
        matchers,
        label_selector,
    )
    .await?;
    if batches.is_empty() {
        return Ok(vec![(
            SessionContext::new(),
            Arc::new(Schema::empty()),
            ScanStats::default(),
        )]);
    }

    let mut arrow_scan_stats = ScanStats::new();
    arrow_scan_stats.files = batches.len() as i64;
    for batch in batches.iter() {
        arrow_scan_stats.original_size += batch.size() as i64;
    }

    log::info!(
        "[trace_id {trace_id}] promql->wal->search: load wal files: batches {}, scan_size {}",
        arrow_scan_stats.files,
        arrow_scan_stats.original_size,
    );

    let ctx = prepare_datafusion_context(None, vec![], false, 0).await?;
    let mem_table = Arc::new(MemTable::try_new(schema.clone(), vec![batches])?);
    ctx.register_table(stream_name, mem_table)?;
    resp.push((ctx, schema, arrow_scan_stats));

    Ok(resp)
}

/// get file list from local cache, no need match_source, each file will be
/// searched
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(name = "promql:search:grpc:wal:get_file_list", skip(trace_id))]
async fn get_file_list(
    trace_id: &str,
    org_id: &str,
    stream_name: &str,
    schema: Arc<Schema>,
    time_range: (i64, i64),
    matchers: Matchers,
    label_selector: Option<HashSet<String>>,
) -> Result<Vec<RecordBatch>> {
    let cfg = get_config();
    let nodes = get_cached_online_ingester_nodes().await;
    if nodes.is_none() && nodes.as_deref().unwrap().is_empty() {
        return Ok(vec![]);
    }
    let nodes = nodes.unwrap();

    let ctx = prepare_datafusion_context(None, vec![], false, 0).await?;
    let table = Arc::new(
        NewEmptyTable::new(stream_name, Arc::clone(&schema))
            .with_partitions(ctx.state().config().target_partitions()),
    );
    ctx.register_table(stream_name, table)?;

    // create physical plan
    let (start, end) = time_range;
    let mut df = match ctx.table(stream_name).await {
        Ok(df) => df.filter(
            col(&cfg.common.column_timestamp)
                .gt(lit(start))
                .and(col(&cfg.common.column_timestamp).lt_eq(lit(end))),
        )?,
        Err(_) => {
            return Ok(vec![]);
        }
    };

    df = apply_matchers(df, &schema, &matchers)?;

    match apply_label_selector(df, &schema, &label_selector) {
        Some(dataframe) => df = dataframe,
        None => return Ok(vec![]),
    }

    let plan = df.logical_plan();

    let mut physical_plan = ctx.state().create_physical_plan(plan).await?;

    if cfg.common.print_key_sql {
        print_plan(&physical_plan, "before");
    }

    let remote_scan_node = RemoteScanNode {
        nodes: nodes.into_arc_vec(),
        opentelemetry_context: tracing::Span::current().context(),
        query_identifier: QueryIdentifier {
            trace_id: trace_id.to_string(),
            org_id: org_id.to_string(),
            stream_type: StreamType::Metrics.to_string(),
            partition: 0,
            job_id: "".to_string(),
        },
        search_infos: SearchInfos {
            plan: vec![],
            file_id_list: vec![],
            idx_file_list: vec![],
            start_time: time_range.0,
            end_time: time_range.1,
            timeout: cfg.limit.query_timeout as u64,
        },
        index_info: IndexInfo {
            use_inverted_index: false,
            index_condition: "".to_string(),
            equal_keys: vec![],
            match_all_keys: vec![],
            index_optimize_mode: None,
        },
        super_cluster_info: cluster_rpc::SuperClusterInfo {
            is_super_cluster: false,
            user_id: None,
            work_group: None,
            search_event_type: None,
        },
    };

    physical_plan = Arc::new(RemoteScanExec::new(physical_plan, remote_scan_node)?);

    if cfg.common.print_key_sql {
        print_plan(&physical_plan, "after");
    }

    // run datafusion
    let ret = datafusion::physical_plan::collect(physical_plan.clone(), ctx.task_ctx()).await;
    let mut visit = ScanStatsVisitor::new();
    let _ = visit_execution_plan(physical_plan.as_ref(), &mut visit);
    let (mut batches, ..) = if let Err(e) = ret {
        log::error!("[trace_id {trace_id}] flight->search: datafusion collect error: {e}");
        Err(e)
    } else {
        log::info!("[trace_id {trace_id}] flight->search: datafusion collect done");
        ret.map(|data| (data, visit.scan_stats, visit.partial_err))
    }?;

    for batch in batches.iter_mut() {
        *batch = adapt_batch(Arc::clone(&schema), batch);
    }

    Ok(batches)
}
