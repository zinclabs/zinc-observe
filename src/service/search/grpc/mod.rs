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

use ::datafusion::arrow::{ipc, record_batch::RecordBatch};
use arrow_schema::Schema;
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        search::ScanStats,
        stream::{FileKey, StreamType},
    },
    utils::record_batch_ext::format_recordbatch_by_schema,
};
use futures::future::try_join_all;
use hashbrown::HashSet;
use infra::errors::{Error, ErrorCodes};
use proto::cluster_rpc;
use tracing::Instrument;

use super::datafusion;
use crate::service::db;
mod storage;
mod wal;

pub type SearchResult = Result<(Vec<Vec<RecordBatch>>, ScanStats), Error>;

#[tracing::instrument(name = "service:search:grpc:search", skip_all, fields(org_id = req.org_id))]
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
    let wal_parquet_span = tracing::span::Span::current();
    let task1 = tokio::task::spawn(async move {
        if LOCAL_NODE.is_ingester() && !skip_wal {
            wal::search_parquet(&trace_id1, sql1, stream_type, &work_group1, timeout)
                .instrument(wal_parquet_span)
                .await
        } else {
            Ok((vec![], ScanStats::default()))
        }
    });

    // search in WAL memory
    let work_group2 = work_group.clone();
    let trace_id2 = trace_id.clone();
    let sql2 = sql.clone();
    let wal_mem_span = tracing::span::Span::current();
    let task2 = tokio::task::spawn(async move {
        if LOCAL_NODE.is_ingester() && !skip_wal {
            wal::search_memtable(&trace_id2, sql2, stream_type, &work_group2, timeout)
                .instrument(wal_mem_span)
                .await
        } else {
            Ok((vec![], ScanStats::default()))
        }
    });

    // search in object storage
    let req_stype = req.stype;
    let work_group3 = work_group.clone();
    let trace_id3 = trace_id.clone();
    let sql3 = sql.clone();
    let file_list: Vec<FileKey> = req.file_list.iter().map(FileKey::from).collect();
    let storage_span = tracing::span::Span::current();
    let task3 = tokio::task::spawn(async move {
        if req_stype == cluster_rpc::SearchType::WalOnly as i32 {
            Ok((vec![], ScanStats::default()))
        } else {
            storage::search(
                &trace_id3,
                sql3,
                &file_list,
                stream_type,
                &work_group3,
                timeout,
            )
            .instrument(storage_span)
            .await
        }
    });

    // merge result
    let mut results = Vec::new();
    let mut scan_stats = ScanStats::new();
    let tasks = try_join_all(vec![task1, task2, task3])
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?;
    for task in tasks {
        let (batches, stats) = task?;
        scan_stats.add(&stats);
        results.extend(batches.into_iter().flatten().filter(|v| v.num_rows() > 0));
    }

    // format recordbatch with same schema
    if !results.is_empty() {
        let mut schema = results[0].schema();
        let schema_fields = schema
            .fields()
            .iter()
            .map(|f| f.name())
            .collect::<HashSet<_>>();
        let mut new_fields = HashSet::new();
        let mut need_format = false;
        for batch in results.iter() {
            if batch.num_rows() == 0 {
                continue;
            }
            if batch.schema().fields() != schema.fields() {
                need_format = true;
            }
            for field in batch.schema().fields() {
                if !schema_fields.contains(field.name()) {
                    new_fields.insert(field.clone());
                }
            }
        }
        drop(schema_fields);
        if !new_fields.is_empty() {
            need_format = true;
            let new_schema = Schema::new(new_fields.into_iter().collect::<Vec<_>>());
            schema =
                Arc::new(Schema::try_merge(vec![schema.as_ref().clone(), new_schema]).unwrap());
        }
        if need_format {
            let mut new_batches = Vec::new();
            for batch in results {
                if batch.num_rows() == 0 {
                    continue;
                }
                new_batches.push(format_recordbatch_by_schema(schema.clone(), batch));
            }
            results = new_batches;
        }
    }
    log::info!("[trace_id {trace_id}] in node merge task finish");

    // clear session data
    datafusion::storage::file_list::clear(&trace_id);

    // final result
    let mut hits_total = 0;
    let mut hits_buf = vec![];
    let results = results.into_iter().collect::<Vec<_>>();
    if !results.is_empty() {
        let schema = results[0].schema();
        let ipc_options = ipc::writer::IpcWriteOptions::default();
        let ipc_options = ipc_options
            .try_with_compression(Some(ipc::CompressionType::ZSTD))
            .unwrap();
        let buf = Vec::new();
        let mut writer =
            ipc::writer::FileWriter::try_new_with_options(buf, &schema, ipc_options).unwrap();
        for batch in results {
            hits_total += batch.num_rows();
            if let Err(e) = writer.write(&batch) {
                log::error!(
                    "[trace_id {trace_id}] write record batch to ipc error: {}",
                    e
                );
            }
        }
        if let Err(e) = writer.finish() {
            log::error!(
                "[trace_id {trace_id}] convert record batch to ipc error: {}",
                e
            );
        }
        if let Ok(v) = writer.into_inner() {
            hits_buf = v;
        }
    }

    scan_stats.format_to_mb();
    let result = cluster_rpc::SearchResponse {
        job: req.job.clone(),
        took: start.elapsed().as_millis() as i32,
        idx_took: 0,
        from: sql.meta.offset as i32,
        size: sql.meta.limit as i32,
        total: hits_total as i64,
        hits: hits_buf,
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
