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

use std::{sync::Arc, time::Duration};

use ::datafusion::{arrow::ipc, catalog::TableProvider, error::DataFusionError};
use arrow_schema::Schema;
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        bitvec::BitVec,
        cluster::RoleGroup,
        search::{ScanStats, SearchType, StorageType},
        stream::{FileKey, StreamPartition, StreamType},
    },
    utils::{inverted_index::split_token, record_batch_ext::format_recordbatch_by_schema},
    INDEX_FIELD_NAME_FOR_ALL, QUERY_WITH_NO_LIMIT,
};
use hashbrown::{HashMap, HashSet};
use infra::{
    errors::{Error, ErrorCodes},
    schema::unwrap_stream_settings,
};
use proto::cluster_rpc;

use crate::{
    common::infra::cluster as infra_cluster,
    service::{
        db,
        file_list::query_by_ids,
        search::{
            datafusion::exec,
            generate_search_schema, generate_select_start_search_schema,
            sql::{Sql, RE_SELECT_WILDCARD},
        },
    },
};
mod storage;
mod wal;

pub type SearchTable = Result<(Vec<Arc<dyn TableProvider>>, ScanStats), Error>;

#[tracing::instrument(name = "service:search:grpc:search", skip_all, fields(org_id = req.org_id))]
pub async fn search(
    req: &cluster_rpc::SearchRequest,
) -> Result<cluster_rpc::SearchResponse, Error> {
    let start = std::time::Instant::now();
    let sql = Arc::new(super::sql::Sql::new(req).await?);
    let stream_type = StreamType::from(req.stream_type.as_str());
    let work_group = req.work_group.clone();

    let cfg = get_config();
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

    // set target partitions based on cache type
    // construct latest schema map
    let schema_latest = infra::schema::get(&sql.org_id, &sql.stream_name, stream_type)
        .await
        .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?;
    let mut schema_latest_map = HashMap::with_capacity(schema_latest.fields().len());
    for field in schema_latest.fields() {
        schema_latest_map.insert(field.name(), field);
    }
    let stream_settings = unwrap_stream_settings(&schema_latest).unwrap_or_default();
    let defined_schema_fields = stream_settings.defined_schema_fields.unwrap_or_default();

    let select_wildcard = RE_SELECT_WILDCARD.is_match(sql.origin_sql.as_str());

    let (schema, _) = if select_wildcard {
        generate_select_start_search_schema(
            &sql,
            &schema_latest,
            &schema_latest_map,
            &defined_schema_fields,
        )?
    } else {
        generate_search_schema(&sql, &schema_latest, &schema_latest_map)?
    };

    let sql = Arc::new(Sql {
        schema: schema.as_ref().clone(),
        ..sql.as_ref().clone()
    });

    // get all tables
    let mut tables = Vec::new();
    let mut scan_stats = ScanStats::new();

    // binding release lock files
    let _release_file_guard = ReleaseFileGuard::new(&trace_id);

    // search in WAL parquet
    let skip_wal = req.query.as_ref().unwrap().skip_wal;
    if LOCAL_NODE.is_ingester() && !skip_wal {
        let (tbls, stats) =
            wal::search_parquet(&trace_id, sql.clone(), stream_type, &work_group).await?;
        tables.extend(tbls);
        scan_stats.add(&stats);
    }

    // search in WAL memory
    if LOCAL_NODE.is_ingester() && !skip_wal {
        let (tbls, stats) = wal::search_memtable(&trace_id, sql.clone(), stream_type).await?;
        tables.extend(tbls);
        scan_stats.add(&stats);
    }

    // search in object storage
    let req_stype = req.stype;
    if req_stype != cluster_rpc::SearchType::WalOnly as i32 {
        let ids = &req.file_id_list;
        let mut file_list = get_file_list_by_ids(
            &trace_id,
            ids,
            &sql,
            stream_type,
            &stream_settings.partition_keys,
        )
        .await;

        // YJDoc2, here we will need to fetch stuff from the ids
        log::warn!("HERERE I AM!!!!! This is the worker node code");
        log::warn!("req :{:?}", req);
        let file_list: Vec<FileKey> = req.file_list.iter().map(FileKey::from).collect();
        let (tbls, stats) =
            storage::search(&trace_id, sql.clone(), &file_list, stream_type, &work_group).await?;
        tables.extend(tbls);
        scan_stats.add(&stats);
    }

    let session = config::meta::search::Session {
        id: trace_id.to_string(),
        storage_type: StorageType::Memory,
        search_type: if !sql.meta.group_by.is_empty() {
            SearchType::Aggregation
        } else {
            SearchType::Normal
        },
        work_group: Some(work_group.to_string()),
        target_partitions: cfg.limit.cpu_num,
    };

    // run and get the RecordBatch
    #[cfg(feature = "enterprise")]
    let (abort_sender, abort_receiver) = tokio::sync::oneshot::channel();
    #[cfg(feature = "enterprise")]
    if crate::service::search::SEARCH_SERVER
        .insert_sender(&trace_id, abort_sender)
        .await
        .is_err()
    {
        log::info!(
            "[trace_id {}] search->storage: search canceled before call search->storage",
            session.id
        );
        return Err(Error::Message(format!(
            "[trace_id {}] search->storage: search canceled before call search->storage",
            session.id
        )));
    }

    if tables.is_empty() {
        return Ok(cluster_rpc::SearchResponse {
            job: req.job.clone(),
            took: start.elapsed().as_millis() as i32,
            idx_took: scan_stats.idx_took as i32,
            from: sql.meta.offset as i32,
            size: sql.meta.limit as i32,
            total: 0,
            hits: vec![],
            scan_stats: Some(cluster_rpc::ScanStats::from(&scan_stats)),
            is_partial: false,
        });
    }

    let ret = tokio::select! {
        ret = exec::sql(&session, &sql, tables) => {
            match ret {
                Ok(ret) => Ok(ret),
                Err(err) => {
                    log::error!("[trace_id {}] search->storage: datafusion execute error: {}", session.id, err);
                    Err(err)
                }
            }
        },
        _ = tokio::time::sleep(Duration::from_secs(timeout)) => {
            log::error!("[trace_id {}] search->storage: search timeout", session.id);
            Err(DataFusionError::ResourcesExhausted(format!(
                "[trace_id {}] search->storage: task timeout", session.id
            )))
        },
        _ = async {
            #[cfg(feature = "enterprise")]
            let _ = abort_receiver.await;
            #[cfg(not(feature = "enterprise"))]
            futures::future::pending::<()>().await;
        } => {
            log::info!("[trace_id {}] search->storage: search canceled", session.id);
            Err( DataFusionError::Execution(format!(
                "[trace_id {}] search->storage: task is cancel", session.id
            )))
        }
    };

    let mut results = match ret {
        Ok(v) => v,
        Err(err) => match err {
            DataFusionError::ResourcesExhausted(e) => {
                return Err(Error::ErrorCode(ErrorCodes::SearchTimeout(e)));
            }
            _ => return Err(err.into()),
        },
    };

    // format recordbatch with same schema
    let merge_schema = results
        .iter()
        .filter_map(|v| {
            if v.num_rows() > 0 {
                Some(v.schema())
            } else {
                None
            }
        })
        .next()
        .unwrap_or_else(|| Arc::new(Schema::empty()));
    if !merge_schema.fields().is_empty() {
        let mut schema = merge_schema.clone();
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
        idx_took: scan_stats.idx_took as i32,
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

#[tracing::instrument(skip(sql), fields(org_id = sql.org_id, stream_name = sql.stream_name))]
pub(crate) async fn get_file_list_by_ids(
    _trace_id: &str,
    ids: &[i64],
    sql: &super::sql::Sql,
    stream_type: StreamType,
    partition_keys: &[StreamPartition],
) -> Vec<FileKey> {
    let is_local = get_config().common.meta_store_external
        || infra_cluster::get_cached_online_querier_nodes(Some(RoleGroup::Interactive))
            .await
            .unwrap_or_default()
            .len()
            <= 1;
    let file_list = query_by_ids(ids, is_local).await.unwrap_or_default();

    let mut files = Vec::with_capacity(file_list.len());
    for file in file_list {
        if sql
            .match_source(&file, false, false, stream_type, partition_keys)
            .await
        {
            files.push(file.to_owned());
        }
    }
    files.sort_by(|a, b| a.key.cmp(&b.key));
    files.dedup_by(|a, b| a.key == b.key);
    files
}

struct ReleaseFileGuard {
    trace_id: String,
}

impl ReleaseFileGuard {
    fn new(trace_id: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
        }
    }
}

impl Drop for ReleaseFileGuard {
    fn drop(&mut self) {
        log::info!(
            "[trace_id {}] grpc->search: drop ReleaseFileGuard",
            self.trace_id
        );
        // clear session data
        crate::service::search::datafusion::storage::file_list::clear(&self.trace_id);
        // release wal lock files
        crate::common::infra::wal::release_request(&self.trace_id);
    }
}
