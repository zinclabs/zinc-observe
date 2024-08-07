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

use arrow::ipc;
use config::meta::stream::StreamType;
use infra::errors::Result;
use proto::cluster_rpc;

pub async fn search(mut req: cluster_rpc::SearchRequest) -> Result<cluster_rpc::SearchResponse> {
    let start = std::time::Instant::now();
    let trace_id = req.job.as_ref().unwrap().trace_id.clone();
    let stream_type = StreamType::from(req.stream_type.as_str());
    let job = req.job.clone();

    // handle request time range
    let meta = super::super::sql::Sql::new(&req).await?;
    if meta.rewrite_sql != req.query.as_ref().unwrap().sql {
        req.query.as_mut().unwrap().sql = meta.rewrite_sql.clone();
    }
    let sql = Arc::new(meta);

    // set this value to null & use it later on results ,
    // this being to avoid performance impact of query fn being applied during query
    // execution
    let _query_fn = req.query.as_ref().unwrap().query_fn.clone();
    req.query.as_mut().unwrap().query_fn = "".to_string();

    log::info!(
        "[trace_id {trace_id}] grpc->cluster_search in: part_id: {}, stream: {}/{}/{}, time range: {:?}",
        req.job.as_ref().unwrap().partition,
        sql.org_id,
        stream_type,
        sql.stream_name,
        sql.meta.time_range
    );

    // handle query function
    let (merge_results, scan_stats, _, is_partial, idx_took) =
        super::search(&trace_id, sql.clone(), req).await?;

    // final result
    let mut hits_buf = Vec::new();
    let mut hits_total = 0;
    if !merge_results.is_empty() {
        let schema = merge_results[0].schema();
        let ipc_options = ipc::writer::IpcWriteOptions::default();
        let ipc_options = ipc_options
            .try_with_compression(Some(ipc::CompressionType::ZSTD))
            .unwrap();
        let buf = Vec::new();
        let mut writer =
            ipc::writer::FileWriter::try_new_with_options(buf, &schema, ipc_options).unwrap();
        for batch in merge_results {
            if batch.num_rows() > 0 {
                hits_total += batch.num_rows();
                if let Err(e) = writer.write(&batch) {
                    log::error!(
                        "[trace_id {trace_id}] write record batch to ipc error: {}",
                        e
                    );
                }
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

    let result = cluster_rpc::SearchResponse {
        job,
        took: start.elapsed().as_millis() as i32,
        idx_took: idx_took as i32,
        from: sql.meta.offset as i32,
        size: sql.meta.limit as i32,
        total: hits_total as i64,
        hits: hits_buf,
        scan_stats: Some(cluster_rpc::ScanStats::from(&scan_stats)),
        is_partial,
    };

    Ok(result)
}
