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

use ::datafusion::{
    common::tree_node::TreeNode, datasource::TableProvider, physical_plan::ExecutionPlan,
    prelude::SessionContext,
};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        bitvec::BitVec,
        inverted_index::InvertedIndexOptimizeMode,
        search::ScanStats,
        stream::{FileKey, StreamPartition, StreamType},
    },
    utils::json,
};
use datafusion::physical_plan::union::UnionExec;
use datafusion_proto::bytes::physical_plan_from_bytes_with_extension_codec;
use hashbrown::HashMap;
use infra::{
    errors::{Error, ErrorCodes},
    schema::{
        get_stream_setting_fts_fields, get_stream_setting_index_setting_timestamp,
        unwrap_stream_created_at, unwrap_stream_settings,
    },
};
use itertools::Itertools;
use proto::cluster_rpc;
use rayon::slice::ParallelSliceMut;

use crate::service::{
    db,
    search::{
        datafusion::{
            distributed_plan::{
                codec::{ComposedPhysicalExtensionCodec, EmptyExecPhysicalExtensionCodec},
                empty_exec::NewEmptyExec,
                NewEmptyExecVisitor, ReplaceTableScanExec,
            },
            exec::{prepare_datafusion_context, register_udf},
            plan::tantivy_count_exec::TantivyCountExec,
            table_provider::uniontable::NewUnionTable,
        },
        index::IndexCondition,
        match_file,
        request::FlightSearchRequest,
    },
};

#[tracing::instrument(name = "service:search:grpc:flight:do_get::search", skip_all, fields(org_id = req.query_identifier.org_id))]
pub async fn search(
    req: &FlightSearchRequest,
) -> Result<(SessionContext, Arc<dyn ExecutionPlan>, ScanStats), Error> {
    // let start = std::time::Instant::now();
    let cfg = get_config();

    let org_id = req.query_identifier.org_id.to_string();
    let stream_type = StreamType::from(req.query_identifier.stream_type.as_str());
    let work_group = req.super_cluster_info.work_group.clone();

    let trace_id = Arc::new(req.query_identifier.trace_id.to_string());
    log::info!("[trace_id {trace_id}] flight->search: start");

    // create datafusion context, just used for decode plan, the params can use default
    let mut ctx =
        prepare_datafusion_context(work_group.clone(), vec![], false, cfg.limit.cpu_num).await?;

    // register UDF
    register_udf(&ctx, &org_id)?;
    datafusion_functions_json::register_all(&mut ctx)?;

    // Decode physical plan from bytes
    let proto = ComposedPhysicalExtensionCodec {
        codecs: vec![Arc::new(EmptyExecPhysicalExtensionCodec {})],
    };
    let mut physical_plan =
        physical_plan_from_bytes_with_extension_codec(&req.search_info.plan, &ctx, &proto)?;

    // replace empty table to real table
    let mut visitor = NewEmptyExecVisitor::default();
    if physical_plan.visit(&mut visitor).is_err() || visitor.get_data().is_none() {
        return Err(Error::Message(
            "flight->search: physical plan visit error: there is no EmptyTable".to_string(),
        ));
    }
    let empty_exec = visitor
        .get_data()
        .unwrap()
        .as_any()
        .downcast_ref::<NewEmptyExec>()
        .unwrap();

    // here need reset the option because when init ctx we don't know this information
    if empty_exec.sorted_by_time() {
        ctx.state_ref().write().config_mut().options_mut().set(
            "datafusion.execution.split_file_groups_by_statistics",
            "true",
        )?;
    }

    // get stream name
    let stream_name = empty_exec.name();

    // check if we are allowed to search
    if db::compact::retention::is_deleting_stream(&org_id, stream_type, stream_name, None) {
        return Err(Error::ErrorCode(ErrorCodes::SearchStreamNotFound(format!(
            "stream [{}] is being deleted",
            &stream_name
        ))));
    }

    log::info!(
        "[trace_id {trace_id}] flight->search: part_id: {}, stream: {}/{}/{}",
        req.query_identifier.partition,
        org_id,
        stream_type,
        stream_name,
    );

    // construct latest schema map
    let schema_latest = empty_exec.full_schema();
    let mut schema_latest_map = HashMap::with_capacity(schema_latest.fields().len());
    for field in schema_latest.fields() {
        schema_latest_map.insert(field.name(), field);
    }

    // construct index condition
    let index_condition = generate_index_condition(&req.index_info.index_condition)?;

    let stream_settings = unwrap_stream_settings(schema_latest.as_ref());
    let stream_created_at = unwrap_stream_created_at(schema_latest.as_ref());
    let fst_fields = get_stream_setting_fts_fields(&stream_settings)
        .into_iter()
        .filter_map(|v| {
            if schema_latest_map.contains_key(&v) {
                Some(v)
            } else {
                None
            }
        })
        .collect_vec();
    let index_setting_timestamp =
        get_stream_setting_index_setting_timestamp(&stream_settings, stream_created_at);

    // construct partition filters
    let search_partition_keys: Vec<(String, String)> = req
        .index_info
        .equal_keys
        .iter()
        .filter_map(|v| {
            if schema_latest_map.contains_key(&v.key) {
                Some((v.key.to_string(), v.value.to_string()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let query_params = Arc::new(super::QueryParams {
        trace_id: trace_id.to_string(),
        org_id: org_id.clone(),
        job_id: req.query_identifier.job_id.clone(),
        stream_type,
        stream_name: stream_name.to_string(),
        time_range: Some((req.search_info.start_time, req.search_info.end_time)),
        work_group: work_group.clone(),
        use_inverted_index: req.index_info.use_inverted_index,
    });

    let mut idx_optimize_rule: Option<InvertedIndexOptimizeMode> =
        req.index_info.index_optimize_mode.clone().map(|x| x.into());

    // get all tables
    let mut tables = Vec::new();
    let mut scan_stats = ScanStats::new();
    let file_stats_cache = ctx.runtime_env().cache_manager.get_file_statistic_cache();

    // search in object storage
    let mut tantivy_file_list = Vec::new();
    if !req.search_info.file_id_list.is_empty() {
        let stream_settings = infra::schema::get_settings(&org_id, stream_name, stream_type)
            .await
            .unwrap_or_default();
        let (mut file_list, file_list_took) = get_file_list_by_ids(
            &trace_id,
            &org_id,
            stream_type,
            stream_name,
            query_params.time_range,
            &stream_settings.partition_keys,
            &search_partition_keys,
            &req.search_info.file_id_list,
            &req.search_info.idx_file_list,
        )
        .await?;
        log::info!(
            "[trace_id {trace_id}] flight->search in: part_id: {}, get file_list by ids, num: {}, took: {} ms",
            req.query_identifier.partition,
            file_list.len(),
            file_list_took,
        );

        if physical_plan.name() == "AggregateExec"
            && physical_plan.schema().fields().len() == 1
            && matches!(
                idx_optimize_rule,
                Some(InvertedIndexOptimizeMode::SimpleCount)
            )
        {
            let (tantivy_files, datafusion_files) = split_file_list_by_time_range(
                file_list,
                req.search_info.start_time,
                req.search_info.end_time,
                index_setting_timestamp,
            );
            tantivy_file_list = tantivy_files;
            file_list = datafusion_files;
            idx_optimize_rule = None;
        }

        let (tbls, stats) = match super::storage::search(
            query_params.clone(),
            schema_latest.clone(),
            &file_list,
            empty_exec.sorted_by_time(),
            file_stats_cache.clone(),
            index_condition.clone(),
            fst_fields.clone(),
            idx_optimize_rule,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                // clear session data
                super::super::datafusion::storage::file_list::clear(&trace_id);
                log::error!(
                    "[trace_id {}] flight->search: search storage parquet error: {}",
                    trace_id,
                    e
                );
                return Err(e);
            }
        };
        tables.extend(tbls);
        scan_stats.add(&stats);
    }

    // search in WAL parquet
    if LOCAL_NODE.is_ingester() {
        let (tbls, stats) = match super::wal::search_parquet(
            query_params.clone(),
            schema_latest.clone(),
            &search_partition_keys,
            empty_exec.sorted_by_time(),
            file_stats_cache.clone(),
            index_condition.clone(),
            fst_fields.clone(),
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                // clear session data
                super::super::datafusion::storage::file_list::clear(&trace_id);
                log::error!(
                    "[trace_id {}] flight->search: search wal parquet error: {}",
                    trace_id,
                    e
                );
                return Err(e);
            }
        };
        tables.extend(tbls);
        scan_stats.add(&stats);
    }

    // search in WAL memory
    if LOCAL_NODE.is_ingester() {
        let (tbls, stats) = match super::wal::search_memtable(
            query_params.clone(),
            schema_latest.clone(),
            &search_partition_keys,
            empty_exec.sorted_by_time(),
            index_condition.clone(),
            fst_fields.clone(),
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "[trace_id {}] flight->search: search wal memtable error: {}",
                    trace_id,
                    e
                );
                return Err(e);
            }
        };
        tables.extend(tbls);
        scan_stats.add(&stats);
    }

    // create a Union Plan to merge all tables
    let start = std::time::Instant::now();
    let union_table = Arc::new(NewUnionTable::try_new(empty_exec.schema().clone(), tables)?);

    let union_exec = union_table
        .scan(
            &ctx.state(),
            empty_exec.projection(),
            empty_exec.filters(),
            empty_exec.limit(),
        )
        .await?;
    let mut rewriter = ReplaceTableScanExec::new(union_exec);
    physical_plan = physical_plan.rewrite(&mut rewriter)?.data;

    if !tantivy_file_list.is_empty() {
        scan_stats.add(&collect_stats(&tantivy_file_list));
        let tantivy_exec = Arc::new(TantivyCountExec::new(
            query_params,
            physical_plan.schema(),
            tantivy_file_list,
            index_condition.unwrap(),
        ));
        physical_plan = Arc::new(UnionExec::new(vec![physical_plan, tantivy_exec as _]));
    }

    log::info!(
        "[trace_id {trace_id}] flight->search: generated physical plan, took: {} ms",
        start.elapsed().as_millis()
    );

    Ok((ctx, physical_plan, scan_stats))
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(org_id = org_id, stream_name = stream_name))]
async fn get_file_list_by_ids(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    time_range: Option<(i64, i64)>,
    partition_keys: &[StreamPartition],
    equal_items: &[(String, String)],
    ids: &[i64],
    idx_file_list: &[cluster_rpc::IdxFileName],
) -> Result<(Vec<FileKey>, usize), Error> {
    let start = std::time::Instant::now();
    let file_list = crate::service::file_list::query_by_ids(trace_id, ids).await?;
    // if there are any files in idx_files_list, use them to filter the files we got from ids,
    // otherwise use all the files we got from ids
    let file_list = if idx_file_list.is_empty() {
        file_list
    } else {
        let mut files = Vec::with_capacity(idx_file_list.len());
        let file_list_map: HashMap<_, _> =
            file_list.into_iter().map(|f| (f.key.clone(), f)).collect();
        for idx_file in idx_file_list.iter() {
            if let Some(file) = file_list_map.get(&idx_file.key) {
                let mut new_file = file.clone();
                if let Some(segment_ids) = idx_file.segment_ids.as_ref() {
                    let segment_ids = BitVec::from_slice(segment_ids);
                    new_file.with_segment_ids(segment_ids);
                }
                files.push(new_file);
            }
        }
        files
    };

    let mut files = Vec::with_capacity(file_list.len());
    for file in file_list {
        if match_file(
            org_id,
            stream_type,
            stream_name,
            time_range,
            &file,
            partition_keys,
            equal_items,
        )
        .await
        {
            files.push(file);
        }
    }
    files.par_sort_unstable_by(|a, b| a.key.cmp(&b.key));
    files.dedup_by(|a, b| a.key == b.key);
    Ok((files, start.elapsed().as_millis() as usize))
}

fn generate_index_condition(index_condition: &str) -> Result<Option<IndexCondition>, Error> {
    Ok(if !index_condition.is_empty() {
        let condition: IndexCondition = match json::from_str(index_condition) {
            Ok(cond) => cond,
            Err(e) => {
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(format!(
                    "Invalid index condition JSON: {}",
                    e
                ))));
            }
        };
        Some(condition)
    } else {
        None
    })
}

// if the file in the [start_time, end_time], it will be in tantivy group
// otherwise it will be in the datafusion group
// (tantivy group, datafusion group)
fn split_file_list_by_time_range(
    file_list: Vec<FileKey>,
    start_time: i64,
    end_time: i64,
    index_setting_timestamp: i64,
) -> (Vec<FileKey>, Vec<FileKey>) {
    file_list.into_iter().partition(|file| {
        file.meta.min_ts >= start_time
            && file.meta.max_ts <= end_time
            && file.meta.min_ts > index_setting_timestamp
            && file.meta.index_size > 0
    })
}

fn collect_stats(files: &[FileKey]) -> ScanStats {
    let mut scan_stats = ScanStats::new();
    scan_stats.files = files.len() as i64;
    for file in files.iter() {
        scan_stats.idx_scan_size += file.meta.index_size;
        scan_stats.records += file.meta.records;
        scan_stats.original_size += file.meta.original_size;
        scan_stats.compressed_size += file.meta.compressed_size;
    }
    scan_stats
}
