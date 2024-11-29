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

use ::datafusion::arrow::record_batch::RecordBatch;
use config::{
    meta::{function::VRLResultResolver, search},
    utils::{
        arrow::record_batches_to_json_rows,
        flatten,
        json::{self, get_int_value},
    },
};
use infra::errors::{Error, ErrorCodes, Result};
use proto::cluster_rpc::SearchQuery;
use vector_enrichment::TableRegistry;

use crate::service::search::{cluster::flight, request::Request, sql::Sql};

#[tracing::instrument(name = "service:search:cluster", skip_all)]
pub async fn search(
    req: Request,
    query: SearchQuery,
    _req_regions: Vec<String>,
    _req_clusters: Vec<String>,
) -> Result<search::Response> {
    let start = std::time::Instant::now();
    let trace_id = req.trace_id.clone();
    let query_type = query.query_type.to_lowercase();
    let track_total_hits = query.track_total_hits;

    // handle request time range
    let meta = Sql::new_from_req(&req, &query).await?;
    let sql = Arc::new(meta);

    // set this value to null & use it later on results ,
    // this being to avoid performance impact of query fn being applied during query
    // execution
    let use_query_fn = query.uses_zo_fn;
    let mut query_fn = query.query_fn.clone();

    #[cfg(feature = "enterprise")]
    let local_cluster_search = _req_regions == vec!["local"]
        && !_req_clusters.is_empty()
        && (_req_clusters == vec!["local"] || _req_clusters == vec![config::get_cluster_name()]);

    // handle query function
    #[cfg(feature = "enterprise")]
    let ret = if o2_enterprise::enterprise::common::infra::config::get_config()
        .super_cluster
        .enabled
        && !local_cluster_search
    {
        super::super::super_cluster::leader::search(
            &trace_id,
            sql.clone(),
            req,
            _req_regions,
            _req_clusters,
        )
        .await
    } else {
        flight::search(&trace_id, sql.clone(), req, query).await
    };
    #[cfg(not(feature = "enterprise"))]
    let ret = flight::search(&trace_id, sql.clone(), req, query).await;

    let (merge_batches, scan_stats, took_wait, is_partial, idx_took, partial_err) = match ret {
        Ok(v) => v,
        Err(e) => {
            log::error!("[trace_id {trace_id}] http->search: err: {:?}", e);
            return Err(e);
        }
    };

    // final result
    let mut result = search::Response::new(sql.offset, sql.limit);

    // hits
    if !merge_batches.is_empty() {
        let schema = merge_batches[0].schema();
        let batches_query_ref: Vec<&RecordBatch> = merge_batches.iter().collect();
        let json_rows = record_batches_to_json_rows(&batches_query_ref)
            .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?;
        let mut sources: Vec<json::Value> = if query_fn.is_empty() {
            json_rows
                .into_iter()
                .filter(|v| !v.is_empty())
                .map(json::Value::Object)
                .collect()
        } else {
            // compile vrl function & apply the same before returning the response
            let input_fn = query_fn.trim();

            let apply_over_hits = super::super::RESULT_ARRAY.is_match(input_fn);
            if apply_over_hits {
                query_fn = super::super::RESULT_ARRAY.replace(input_fn, "").to_string();
            }
            let mut runtime = crate::common::utils::functions::init_vrl_runtime();
            let program =
                match crate::service::ingestion::compile_vrl_function(&query_fn, &sql.org_id) {
                    Ok(program) => {
                        let registry = program.config.get_custom::<TableRegistry>().unwrap();
                        registry.finish_load();
                        Some(program)
                    }
                    Err(err) => {
                        log::error!("[trace_id {trace_id}] search->vrl: compile err: {:?}", err);
                        result.function_error = err.to_string();
                        None
                    }
                };
            match program {
                Some(program) => {
                    if apply_over_hits {
                        let (ret_val, _) = crate::service::ingestion::apply_vrl_fn(
                            &mut runtime,
                            &VRLResultResolver {
                                program: program.program.clone(),
                                fields: program.fields.clone(),
                            },
                            json::Value::Array(
                                json_rows
                                    .into_iter()
                                    .filter(|v| !v.is_empty())
                                    .map(json::Value::Object)
                                    .collect(),
                            ),
                            &sql.org_id,
                            &sql.stream_names,
                        );
                        ret_val
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(|v| {
                                (!v.is_null()).then_some(flatten::flatten(v.clone()).unwrap())
                            })
                            .collect()
                    } else {
                        json_rows
                            .into_iter()
                            .filter(|v| !v.is_empty())
                            .filter_map(|hit| {
                                let (ret_val, _) = crate::service::ingestion::apply_vrl_fn(
                                    &mut runtime,
                                    &VRLResultResolver {
                                        program: program.program.clone(),
                                        fields: program.fields.clone(),
                                    },
                                    json::Value::Object(hit),
                                    &sql.org_id,
                                    &sql.stream_names,
                                );
                                (!ret_val.is_null()).then_some(flatten::flatten(ret_val).unwrap())
                            })
                            .collect()
                    }
                }
                None => json_rows
                    .into_iter()
                    .filter(|v| !v.is_empty())
                    .map(json::Value::Object)
                    .collect(),
            }
        };
        // handle query type: json, metrics, table
        if query_type == "table" {
            (result.columns, sources) = super::handle_table_response(schema, sources);
        } else if query_type == "metrics" {
            sources = super::handle_metrics_response(sources);
        }

        if use_query_fn {
            for source in sources {
                result
                    .add_hit(&flatten::flatten(source).map_err(|e| Error::Message(e.to_string()))?);
            }
        } else {
            for source in sources {
                result.add_hit(&source);
            }
        }
    }

    let total = if !track_total_hits {
        result.hits.len()
    } else {
        result
            .hits
            .first()
            .map(|v| {
                v.get("zo_sql_num")
                    .map(|v| get_int_value(v) as usize)
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };

    result.set_total(total);
    result.set_histogram_interval(sql.histogram_interval);
    result.set_partial(is_partial, partial_err);
    result.set_cluster_took(start.elapsed().as_millis() as usize, took_wait);
    result.set_file_count(scan_stats.files as usize);
    result.set_scan_size(scan_stats.original_size as usize);
    result.set_scan_records(scan_stats.records as usize);
    result.set_cached_ratio(
        (((scan_stats.querier_memory_cached_files + scan_stats.querier_disk_cached_files) * 100)
            as f64
            / scan_stats.querier_files as f64) as usize,
    );
    result.set_idx_scan_size(scan_stats.idx_scan_size as usize);

    result.set_idx_took(if idx_took > 0 {
        idx_took
    } else {
        scan_stats.idx_took as usize
    });

    if query_type == "table" {
        result.response_type = "table".to_string();
    } else if query_type == "metrics" {
        result.response_type = "matrix".to_string();
    }

    // set order by
    if let Some(&ref order_by) = sql.order_by.first() {
        result.set_order_by(Some(order_by.1));
    }

    log::info!(
        "[trace_id {trace_id}] search->result: total: {}, took: {} ms, scan_size: {}",
        result.total,
        result.took,
        result.scan_size,
    );

    Ok(result)
}
