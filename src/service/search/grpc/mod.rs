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

use std::{collections::HashSet, sync::Arc};

use ::datafusion::arrow::{ipc, record_batch::RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use config::{
    cluster, get_config,
    meta::{
        search::ScanStats,
        stream::{FileKey, StreamType},
    },
    FxIndexSet,
};
use futures::future::try_join_all;
use hashbrown::HashMap;
use infra::errors::{Error, ErrorCodes};
use proto::cluster_rpc;
use tracing::{info_span, Instrument};

use super::{datafusion, sql::Sql};
use crate::service::db;
mod storage;
mod wal;

pub type SearchResult = Result<(HashMap<String, Vec<RecordBatch>>, ScanStats), Error>;

#[tracing::instrument(name = "service:search:grpc:search", skip_all, fields(trace_id = req.job.as_ref().unwrap().trace_id, org_id = req.org_id))]
pub async fn search(
    req: &cluster_rpc::SearchRequest,
) -> Result<cluster_rpc::SearchResponse, Error> {
    let start = std::time::Instant::now();
    let sql = Arc::new(super::sql::Sql::new(req).await?);
    let stream_type = StreamType::from(req.stream_type.as_str());
    let work_group = req.work_group.clone();

    let trace_id = Arc::new(req.job.as_ref().unwrap().trace_id.to_string());
    let timeout = if req.timeout > 0 {
        req.timeout as u64
    } else {
        get_config().limit.query_timeout
    };

    // check if we are allowed to search
    if db::compact::retention::is_deleting_stream(&sql.org_id, stream_type, &sql.stream_name, None)
    {
        return Err(Error::ErrorCode(ErrorCodes::SearchStreamNotFound(format!(
            "stream [{}] is being deleted",
            &sql.stream_name
        ))));
    }

    log::info!(
        "[trace_id {trace_id}] grpc->search in: part_id: {}, stream: {}/{}/{}, time range: {:?}",
        req.job.as_ref().unwrap().partition,
        sql.org_id,
        stream_type,
        sql.stream_name,
        sql.meta.time_range
    );

    // search in WAL parquet
    let skip_wal = req.query.as_ref().unwrap().skip_wal;
    let work_group1 = work_group.clone();
    let trace_id1 = trace_id.clone();
    let sql1 = sql.clone();
    let wal_parquet_span = info_span!(
        "service:search:grpc:in_wal_parquet",
        trace_id = trace_id1.as_ref().clone(),
        org_id = sql.org_id,
        stream_name = sql.stream_name,
        stream_type = stream_type.to_string(),
    );
    let task1 = tokio::task::spawn(
        async move {
            if cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) && !skip_wal {
                wal::search_parquet(&trace_id1, sql1, stream_type, &work_group1, timeout).await
            } else {
                Ok((HashMap::new(), ScanStats::default()))
            }
        }
        .instrument(wal_parquet_span),
    );

    // search in WAL memory
    let work_group2 = work_group.clone();
    let trace_id2 = trace_id.clone();
    let sql2 = sql.clone();
    let wal_mem_span = info_span!(
        "service:search:grpc:in_wal_memory",
        trace_id = trace_id2.as_ref().clone(),
        org_id = sql.org_id,
        stream_name = sql.stream_name,
        stream_type = stream_type.to_string(),
    );
    let task2 = tokio::task::spawn(
        async move {
            if cluster::is_ingester(&cluster::LOCAL_NODE_ROLE) && !skip_wal {
                wal::search_memtable(&trace_id2, sql2, stream_type, &work_group2, timeout).await
            } else {
                Ok((HashMap::new(), ScanStats::default()))
            }
        }
        .instrument(wal_mem_span),
    );

    // search in object storage
    let req_stype = req.stype;
    let work_group3 = work_group.clone();
    let trace_id3 = trace_id.clone();
    let sql3 = sql.clone();
    let file_list: Vec<FileKey> = req.file_list.iter().map(FileKey::from).collect();
    let storage_span = info_span!(
        "service:search:grpc:in_storage",
        trace_id = trace_id3.as_ref().clone(),
        org_id = sql.org_id,
        stream_name = sql.stream_name,
        stream_type = stream_type.to_string(),
    );
    let task3 = tokio::task::spawn(
        async move {
            if req_stype == cluster_rpc::SearchType::WalOnly as i32 {
                Ok((HashMap::new(), ScanStats::default()))
            } else {
                storage::search(
                    &trace_id3,
                    sql3,
                    &file_list,
                    stream_type,
                    &work_group3,
                    timeout,
                )
                .await
            }
        }
        .instrument(storage_span),
    );

    // merge result
    let mut results = HashMap::new();
    let mut scan_stats = ScanStats::new();
    let tasks = try_join_all(vec![task1, task2, task3])
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?;
    for task in tasks {
        let (batches, stats) = task?;
        scan_stats.add(&stats);
        for (key, batch) in batches {
            if !batch.is_empty() {
                let value = results.entry(key).or_insert_with(Vec::new);
                value.extend(batch);
            }
        }
    }

    // convert select field to schema::Field
    let select_fields = sql
        .meta
        .fields
        .iter()
        .filter_map(|f| {
            sql.schema
                .field_with_name(f)
                .ok()
                .map(|f| Arc::new(f.clone()))
        })
        .collect::<Vec<_>>();

    // merge all batches
    let (offset, limit) = (0, sql.meta.offset + sql.meta.limit);
    let mut merge_results = HashMap::new();
    for (name, batches) in results {
        let (merge_sql, select_fields) = if name == "query" {
            (sql.origin_sql.clone(), select_fields.clone())
        } else {
            (
                sql.aggs
                    .get(name.strip_prefix("agg_").unwrap())
                    .unwrap()
                    .0
                    .clone(),
                vec![],
            )
        };

        #[cfg(feature = "enterprise")]
        let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
        #[cfg(feature = "enterprise")]
        if crate::service::search::SEARCH_SERVER
            .insert_sender(&trace_id, abort_sender)
            .await
            .is_err()
        {
            log::info!("[trace_id {trace_id}] task is cancel after get first stage result");
            return Err(Error::Message(format!(
                "[trace_id {trace_id}] task is cancel after get first stage result"
            )));
        }

        let merge_batches;
        tokio::select! {
            result = super::datafusion::exec::merge(
                &sql.org_id,
                offset,
                limit,
                &merge_sql,
                &batches,
                &select_fields,
                false,
                sql.meta.subquery.is_some()
            ) => {
                match result {
                    Ok(res) => merge_batches = res,
                    Err(err) => {
                        log::error!("[trace_id {trace_id}] datafusion merge error: {}", err);
                        return Err(err.into());
                    }
                }
            },
            _ = async {
                #[cfg(feature = "enterprise")]
                let _ = abort_receiver.await;
                #[cfg(not(feature = "enterprise"))]
                futures::future::pending::<()>().await;
            } => {
                log::info!("[trace_id {trace_id}] in node merge task is cancel");
                return Err(Error::Message(format!("[trace_id {trace_id}] in node merge task is cancel")));
            }
        }

        merge_results.insert(name.to_string(), merge_batches);
    }

    // clear session data
    datafusion::storage::file_list::clear(&trace_id);

    log::info!("[trace_id {trace_id}] in node merge task finish");

    // final result
    let mut hits_buf = Vec::new();
    let mut hits_total = 0;
    let result_query = merge_results.get("query").cloned().unwrap_or_default();
    if !result_query.is_empty() {
        let schema = result_query[0].schema();
        let ipc_options = ipc::writer::IpcWriteOptions::default();
        let ipc_options = ipc_options
            .try_with_compression(Some(ipc::CompressionType::ZSTD))
            .unwrap();
        let mut writer =
            ipc::writer::FileWriter::try_new_with_options(hits_buf, &schema, ipc_options).unwrap();
        for batch in result_query {
            if batch.num_rows() > 0 {
                hits_total += batch.num_rows();
                writer.write(&batch).unwrap();
            }
        }
        writer.finish().unwrap();
        hits_buf = writer.into_inner().unwrap();
    }

    // finally aggs result
    let mut aggs_buf = Vec::new();
    for (key, batches) in merge_results {
        if key == "query" || batches.is_empty() {
            continue;
        }
        let mut buf = Vec::new();
        let schema = batches[0].schema();
        let ipc_options = ipc::writer::IpcWriteOptions::default();
        let ipc_options = ipc_options
            .try_with_compression(Some(ipc::CompressionType::ZSTD))
            .unwrap();
        let mut writer =
            ipc::writer::FileWriter::try_new_with_options(buf, &schema, ipc_options).unwrap();
        for batch in batches {
            writer.write(&batch).unwrap();
        }
        writer.finish().unwrap();
        buf = writer.into_inner().unwrap();
        aggs_buf.push(cluster_rpc::SearchAggResponse {
            name: key.strip_prefix("agg_").unwrap().to_string(),
            hits: buf,
        });
    }

    scan_stats.format_to_mb();
    let result = cluster_rpc::SearchResponse {
        job: req.job.clone(),
        took: start.elapsed().as_millis() as i32,
        from: sql.meta.offset as i32,
        size: sql.meta.limit as i32,
        total: hits_total as i64,
        hits: hits_buf,
        aggs: aggs_buf,
        scan_stats: Some(cluster_rpc::ScanStats::from(&scan_stats)),
        is_partial: false,
    };

    Ok(result)
}

fn check_memory_circuit_breaker(trace_id: &str, scan_stats: &ScanStats) -> Result<(), Error> {
    let cfg = get_config();
    let scan_size = if scan_stats.compressed_size > 0 {
        scan_stats.compressed_size
    } else {
        scan_stats.original_size
    };
    if let Some(cur_memory) = memory_stats::memory_stats() {
        // left memory < datafusion * breaker_ratio and scan_size >=  left memory
        let left_mem = cfg.limit.mem_total - cur_memory.physical_mem;
        if (left_mem
            < (cfg.memory_cache.datafusion_max_size * cfg.common.memory_circuit_breaker_ratio
                / 100))
            && (scan_size >= left_mem as i64)
        {
            let err = format!(
                "fire memory_circuit_breaker, try to alloc {} bytes, now current memory usage is {} bytes, left memory {} bytes, left memory more than limit of [{} bytes] or scan_size more than left memory , please submit a new query with a short time range",
                scan_size,
                cur_memory.physical_mem,
                left_mem,
                cfg.memory_cache.datafusion_max_size * cfg.common.memory_circuit_breaker_ratio
                    / 100
            );
            log::warn!("[{trace_id}] {}", err);
            return Err(Error::Message(err.to_string()));
        }
    }
    Ok(())
}

// generate parquet file search schema
fn generate_search_schema(
    sql: &Arc<Sql>,
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
) -> Result<(Arc<Schema>, HashMap<String, DataType>), Error> {
    // cacluate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();
    let mut new_fields = Vec::new();

    for field in generate_used_fields_in_query(sql).iter() {
        let group_field = schema.field_with_name(field).ok();
        let latest_field = schema_latest_map.get(field).map(|f| f.as_ref());

        match (group_field, latest_field) {
            // When group_field is None and latest_field is Some, clone latest_field
            (None, Some(field)) => new_fields.push(Arc::new(field.clone())),

            // When both group_field and latest_field are Some, compare their data types
            (Some(group_field), Some(latest_field)) => {
                if group_field.data_type() != latest_field.data_type() {
                    diff_fields.insert(field.to_string(), latest_field.data_type().clone());
                }
                new_fields.push(Arc::new(group_field.clone()));
            }

            // should we return error
            _ => {}
        }
    }

    for (field, alias) in sql.meta.field_alias.iter() {
        if let Some(v) = diff_fields.get(field) {
            diff_fields.insert(alias.to_string(), v.clone());
        }
    }

    let mut schema = Schema::new(new_fields);
    let timestamp = &get_config().common.column_timestamp;
    if schema.field_with_name(timestamp).is_err() {
        // self add timestamp column if no exist
        let field = Arc::new(Field::new(timestamp, DataType::Int64, false));
        schema = Schema::try_merge(vec![Schema::new(vec![field]), schema])?;
    }

    Ok((Arc::new(schema), diff_fields))
}

// generate parquet file search schema
fn generate_select_start_search_schema(
    sql: &Arc<Sql>,
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
    defined_schema_fields: &[String],
) -> Result<(Arc<Schema>, HashMap<String, DataType>), Error> {
    let schema_fields_map = schema
        .fields()
        .iter()
        .map(|f| (f.name(), f))
        .collect::<HashMap<_, _>>();
    // cacluate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();
    for field in schema.fields().iter() {
        if let Some(f) = schema_latest_map.get(field.name()) {
            if f.data_type() != field.data_type() {
                diff_fields.insert(field.name().clone(), f.data_type().clone());
            }
        }
    }
    for (field, alias) in sql.meta.field_alias.iter() {
        if let Some(v) = diff_fields.get(field) {
            diff_fields.insert(alias.to_string(), v.clone());
        }
    }
    // add not exists field in group schema but used in sql
    let mut new_fields = Vec::new();
    for field in generate_used_fields_in_query(sql).iter() {
        if schema_fields_map.get(field).is_none() {
            if let Some(field) = schema_latest_map.get(field) {
                new_fields.push(Arc::new(field.as_ref().clone()));
            }
        }
    }
    let cfg = get_config();
    let schema = if !defined_schema_fields.is_empty() {
        let mut fields: HashSet<String> = defined_schema_fields.iter().cloned().collect();
        if !fields.contains(&cfg.common.column_timestamp) {
            fields.insert(cfg.common.column_timestamp.to_string());
        }
        if !cfg.common.feature_query_exclude_all && !fields.contains(&cfg.common.column_all) {
            fields.insert(cfg.common.column_all.to_string());
        }
        let new_fields = fields
            .iter()
            .filter_map(|f| match schema_fields_map.get(f) {
                Some(f) => Some((*f).clone()),
                None => schema_latest_map.get(f).map(|f| (*f).clone()),
            })
            .collect::<Vec<_>>();
        Schema::new(new_fields)
    } else if !new_fields.is_empty() {
        let new_schema = Schema::new(new_fields);
        Schema::try_merge(vec![schema.to_owned(), new_schema])?
    } else {
        schema.clone()
    };
    Ok((Arc::new(schema), diff_fields))
}

fn generate_used_fields_in_query(sql: &Arc<Sql>) -> Vec<String> {
    let alias_map: HashSet<&String> = sql.meta.field_alias.iter().map(|(_, v)| v).collect();

    // note field name maybe equal to alias name
    let mut used_fields: FxIndexSet<_> = sql
        .meta
        .group_by
        .iter()
        .chain(sql.meta.order_by.iter().map(|(f, _)| f))
        .filter(|f| !alias_map.contains(*f))
        .chain(&sql.meta.fields)
        .cloned()
        .collect();

    for (_, (_, meta)) in &sql.aggs {
        used_fields.extend(meta.fields.iter().cloned());
    }

    used_fields.into_iter().collect()
}
