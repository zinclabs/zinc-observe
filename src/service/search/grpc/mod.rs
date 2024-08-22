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
        bitvec::BitVec,
        inverted_index::{Contains, IndexReader},
        search::ScanStats,
        stream::{FileKey, StreamType},
    },
    utils::record_batch_ext::format_recordbatch_by_schema,
    INDEX_FIELD_NAME_FOR_ALL,
};
use fst::{automaton::Str, IntoStreamer, Streamer};
use futures::future::try_join_all;
use hashbrown::HashSet;
use infra::errors::{Error, ErrorCodes};
use itertools::Itertools;
use proto::cluster_rpc::{self, FullTextTerms};
use tracing::Instrument;

use super::datafusion;
use crate::{
    job::files::idx::convert_parquet_idx_file_name,
    service::db::{self},
};
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
    let mut file_list: Vec<FileKey> = req.file_list.iter().map(FileKey::from).collect();
    // update file_list here through inverted index
    filter_file_list_by_inverted_index(&mut file_list, &req).await?;

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

/// Filter file list using inverted index
/// This function will load the index file corresponding to each file in the file list.
/// FSTs in those files are used to match the incoming query in `SearchRequest`.
/// If the query does not match any FST in the index file, the file will be filtered out.
/// If the query does match then the segment IDs for the file will be updated.
///
/// ! WARNING: all the files in the file list must have an corresponding index file.
/// ! If the index file is missing, the file will be filtered out.
async fn filter_file_list_by_inverted_index(
    file_list: &mut Vec<FileKey>,
    req: &cluster_rpc::SearchRequest,
) -> Result<(), Error> {
    // iterate through file list
    // Fetch the FSTs from the corresponding index files
    let mut tasks = Vec::new();

    // Return if there are not terms to be searched for
    if req.ft_terms.is_none() && req.index_terms.is_none() {
        return Ok(());
    }
    let full_text_terms = Arc::new(req.ft_terms.clone());
    let index_terms = Arc::new(req.index_terms.clone());
    // we can be iterating over a lot of files
    for (file_id, file) in file_list.iter().enumerate() {
        let full_text_term_clone = full_text_terms.clone();
        let index_terms_clone = index_terms.clone();
        let file_name = file.key.clone();
        // Spawn a task for each file, wherein full text search and
        // index search queries are executed
        let task = tokio::task::spawn(async move {
            let res = inverted_index_search_in_file(
                file_id,
                &file_name,
                full_text_term_clone,
                index_terms_clone,
            )
            .await;
            res
        });
        tasks.push(task)
    }

    for result in futures::future::try_join_all(tasks).await.map_err(|e| {
        Error::Message(format!(
            "Error while filtering file list by inverted index: {}",
            e
        ))
    })? {
        if let Err(e) = result {
            log::error!("Error while filtering file list by inverted index: {}", e);
            continue;
        }

        // Each result corresponds to a file in the file list
        match result {
            Ok((file_id, bitvec)) => {
                if let Some(res) = bitvec {
                    // TODO: Confirm: Is this right?
                    // Replace the segment IDs in the existing `FileKey` with the new found segments
                    file_list[file_id].segment_ids =
                        Some(res.iter_ones().map(|x| x as u8).collect_vec());
                } else {
                    file_list.remove(file_id);
                    continue;
                }
            }
            Err(e) => {
                log::error!("Error while filtering file list by inverted index: {}", e);
                continue;
            }
        }
    }
    Ok(())
}

async fn inverted_index_search_in_file(
    file_id: usize,
    parquet_file_name: &String,
    ft_terms: Arc<Option<FullTextTerms>>,
    index_terms_map: Arc<Option<cluster_rpc::IndexTermMap>>,
) -> anyhow::Result<(usize, Option<BitVec>)> {
    let mut res = BitVec::new();
    let index_file_name = convert_parquet_idx_file_name(parquet_file_name.as_str());
    let index_bytes = tokio::fs::read(index_file_name).await?;
    let mut index_reader = IndexReader::new(futures::io::Cursor::new(index_bytes));
    let file_meta = index_reader.metadata().await.unwrap();

    if let Some(full_text_terms) = ft_terms.as_ref() {
        if let Some(column_index_meta) = file_meta.metas.get(INDEX_FIELD_NAME_FOR_ALL) {
            let fst_offset =
                column_index_meta.base_offset + column_index_meta.relative_fst_offset as u64;
            let fst_size = column_index_meta.fst_size as u32;
            let fst_map = index_reader.fst(fst_offset, fst_size).await.unwrap();

            // construct automatons for multiple full text search terms
            let matchers = full_text_terms
                .terms
                .iter()
                .map(|term| Contains::new(term))
                .collect::<Vec<Contains>>();

            for matcher in matchers {
                // Stream for matched keys and their bitmap offsets
                let mut stream = fst_map.search(matcher).into_stream();
                // We do not care about the key at this point, only the offset
                while let Some((_, value)) = stream.next() {
                    let bitmap = index_reader.get_bitmap(column_index_meta, value).await?;
                    // here we are doing bitwise OR to combine the bitmaps of all the terms
                    res |= bitmap;
                }
            }
        }
    }

    if let Some(index_term_map) = index_terms_map.as_ref() {
        for (col, index_terms) in index_term_map.entires.iter() {
            if let Some(column_index_meta) = file_meta.metas.get(col) {
                let fst_offset =
                    column_index_meta.base_offset + column_index_meta.relative_fst_offset as u64;
                let fst_size = column_index_meta.fst_size as u32;
                let fst_map = index_reader.fst(fst_offset, fst_size).await.unwrap();

                // construct automatons for multiple full text search terms
                let matchers = index_terms
                    .terms
                    .iter()
                    .map(|term| Str::new(term))
                    .collect::<Vec<Str>>();

                for matcher in matchers {
                    // Stream for matched keys and their bitmap offsets
                    let mut stream = fst_map.search(matcher).into_stream();
                    // We do not care about the key at this point, only the offset
                    while let Some((_, value)) = stream.next() {
                        let bitmap = index_reader.get_bitmap(column_index_meta, value).await?;
                        // here we are doing bitwise OR to combine the bitmaps of all the terms
                        res |= bitmap;
                    }
                }
            }
        }
    }

    Ok((file_id, Some(res)))
}
