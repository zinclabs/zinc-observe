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

use std::{cmp::max, sync::Arc};

use arrow_schema::{DataType, Field, Schema};
use chrono::Duration;
use config::{
    get_config, ider,
    meta::{
        cluster::RoleGroup,
        search,
        sql::OrderBy,
        stream::{FileKey, StreamParams, StreamPartition, StreamType},
        usage::{RequestStats, UsageType},
    },
    metrics,
    utils::{
        base64,
        sql::is_aggregate_query,
        str::{find, StringExt},
    },
};
use hashbrown::HashMap;
use infra::{
    errors::{Error, ErrorCodes},
    schema::{
        get_stream_setting_index_fields, unwrap_partition_time_level, unwrap_stream_settings,
    },
};
use once_cell::sync::Lazy;
use opentelemetry::trace::TraceContextExt;
use proto::cluster_rpc::{self, SearchQuery};
use regex::Regex;
use sql::Sql;
use tokio::runtime::Runtime;
#[cfg(not(feature = "enterprise"))]
use tokio::sync::Mutex;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
#[cfg(feature = "enterprise")]
use {
    crate::service::grpc::get_cached_channel,
    config::meta::cluster::get_internal_grpc_token,
    o2_enterprise::enterprise::search::TaskStatus,
    std::collections::HashSet,
    tonic::{codec::CompressionEncoding, metadata::MetadataValue, Request},
    tracing::info_span,
};

use super::usage::report_request_usage_stats;
use crate::{
    common::infra::cluster as infra_cluster, handler::grpc::request::search::Searcher,
    service::format_partition_key,
};

pub(crate) mod cache;
pub(crate) mod cluster;
pub(crate) mod datafusion;
pub(crate) mod grpc;
pub(crate) mod request;
pub(crate) mod sql;
#[cfg(feature = "enterprise")]
pub(crate) mod super_cluster;
pub(crate) mod utlis;

// Checks for #ResultArray#
pub static RESULT_ARRAY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#[ \s]*Result[ \s]*Array[ \s]*#").unwrap());

// search manager
pub static SEARCH_SERVER: Lazy<Searcher> = Lazy::new(Searcher::new);

#[cfg(not(feature = "enterprise"))]
pub(crate) static QUEUE_LOCKER: Lazy<Arc<Mutex<bool>>> =
    Lazy::new(|| Arc::new(Mutex::const_new(false)));

pub static DATAFUSION_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .thread_name("datafusion_runtime")
        .worker_threads(config::get_config().limit.cpu_num)
        .enable_all()
        .build()
        .unwrap()
});

#[tracing::instrument(name = "service:search:enter", skip_all)]
pub async fn search(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    user_id: Option<String>,
    in_req: &search::Request,
) -> Result<search::Response, Error> {
    let start = std::time::Instant::now();
    let started_at = chrono::Utc::now().timestamp_micros();
    let cfg = get_config();

    let trace_id = if trace_id.is_empty() {
        if cfg.common.tracing_enabled || cfg.common.tracing_search_enabled {
            let ctx = tracing::Span::current().context();
            ctx.span().span_context().trace_id().to_string()
        } else {
            ider::uuid()
        }
    } else {
        trace_id.to_string()
    };

    #[cfg(feature = "enterprise")]
    {
        let sql = Some(in_req.query.sql.clone());
        let start_time = Some(in_req.query.start_time);
        let end_time = Some(in_req.query.end_time);
        // set search task
        SEARCH_SERVER
            .insert(
                trace_id.clone(),
                TaskStatus::new_leader(
                    vec![],
                    true,
                    user_id.clone(),
                    Some(org_id.to_string()),
                    Some(stream_type.to_string()),
                    sql,
                    start_time,
                    end_time,
                ),
            )
            .await;
    }

    #[cfg(not(feature = "enterprise"))]
    let req_regions = vec![];
    #[cfg(not(feature = "enterprise"))]
    let req_clusters = vec![];
    #[cfg(feature = "enterprise")]
    let req_regions = in_req.regions.clone();
    #[cfg(feature = "enterprise")]
    let req_clusters = in_req.clusters.clone();

    let query: SearchQuery = in_req.query.clone().into();
    let req_query = query.clone();
    let request = crate::service::search::request::Request::new(
        trace_id.clone(),
        org_id.to_string(),
        stream_type,
        in_req.timeout,
        user_id.clone(),
        Some((query.start_time, query.end_time)),
        in_req.search_type.map(|v| v.to_string()),
        in_req.index_type.optional(),
    );

    let span = tracing::span::Span::current();
    let handle = tokio::task::spawn(
        async move { cluster::http::search(request, query, req_regions, req_clusters).await }
            .instrument(span),
    );
    let res = match handle.await {
        Ok(Ok(res)) => Ok(res),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(Error::Message(e.to_string())),
    };
    log::info!("[trace_id {trace_id}] in leader task finish");

    // remove task because task if finished
    #[cfg(feature = "enterprise")]
    SEARCH_SERVER.remove(&trace_id, false).await;

    metrics::QUERY_RUNNING_NUMS
        .with_label_values(&[org_id])
        .dec();

    // do this because of clippy warning
    match res {
        Ok(res) => {
            let time = start.elapsed().as_secs_f64();
            let (report_usage, search_type) = match in_req.search_type {
                Some(search_type) => match search_type {
                    search::SearchEventType::UI => (false, None),
                    search::SearchEventType::Dashboards => (true, in_req.search_type),
                    search::SearchEventType::Reports => (true, in_req.search_type),
                    search::SearchEventType::Alerts => (true, in_req.search_type),
                    search::SearchEventType::DerivedStream => (true, in_req.search_type),
                    search::SearchEventType::RUM => (true, in_req.search_type),
                    search::SearchEventType::Values => (false, None),
                    search::SearchEventType::Other => (false, None),
                },
                None => (false, None),
            };

            if report_usage {
                let stream_name = match config::meta::sql::Sql::new(&req_query.sql) {
                    Ok(v) => v.source.to_string(),
                    Err(e) => {
                        log::error!("report_usage: parse sql error: {:?}", e);
                        "".to_string()
                    }
                };
                let req_stats = RequestStats {
                    records: res.hits.len() as i64,
                    response_time: time,
                    size: res.scan_size as f64,
                    request_body: Some(req_query.sql.clone()),
                    function: if req_query.query_fn.is_empty() {
                        None
                    } else {
                        Some(req_query.query_fn.clone())
                    },
                    user_email: user_id,
                    min_ts: Some(req_query.start_time),
                    max_ts: Some(req_query.end_time),
                    cached_ratio: Some(res.cached_ratio),
                    search_type,
                    trace_id: Some(trace_id),
                    took_wait_in_queue: if res.took_detail.is_some() {
                        let resp_took = res.took_detail.as_ref().unwrap();
                        // Consider only the cluster wait queue duration
                        Some(resp_took.cluster_wait_queue)
                    } else {
                        None
                    },
                    ..Default::default()
                };
                report_request_usage_stats(
                    req_stats,
                    org_id,
                    &stream_name,
                    StreamType::Logs,
                    UsageType::Search,
                    0,
                    started_at,
                )
                .await;
            }
            Ok(res)
        }
        Err(e) => Err(e),
    }
}

#[tracing::instrument(name = "service:search_partition", skip(req))]
pub async fn search_partition(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    req: &search::SearchPartitionRequest,
) -> Result<search::SearchPartitionResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let query = cluster_rpc::SearchQuery {
        start_time: req.start_time,
        end_time: req.end_time,
        sql: req.sql.to_string(),
        ..Default::default()
    };
    let sql = Sql::new(&query, org_id, stream_type).await?;

    // check for vrl
    let apply_over_hits = match req.query_fn.as_ref() {
        None => false,
        Some(v) => {
            if v.is_empty() {
                false
            } else {
                let v = base64::decode_url(v).unwrap_or(v.to_string());
                RESULT_ARRAY.is_match(&v)
            }
        }
    };

    // if there is no _timestamp field in the query, return single partitions
    let is_aggregate = is_aggregate_query(&req.sql).unwrap_or(false);
    let ts_column = cache::cacher::get_ts_col(&sql, &cfg.common.column_timestamp, is_aggregate);
    let skip_get_file_list = ts_column.is_none() || apply_over_hits;

    let mut files = Vec::new();
    let mut max_query_range = 0;
    for (stream, schema) in sql.schemas.iter() {
        let stream_settings = unwrap_stream_settings(schema.schema()).unwrap_or_default();
        let partition_time_level =
            unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);
        if !skip_get_file_list {
            let stream_files = crate::service::file_list::query_ids(
                &sql.org_id,
                stream_type,
                stream,
                partition_time_level,
                sql.time_range,
            )
            .await?;
            max_query_range = max(max_query_range, stream_settings.max_query_range);
            files.extend(stream_files);
        }
    }

    let file_list_took = start.elapsed().as_millis() as usize;
    log::info!(
        "[trace_id {trace_id}] search_partition: get file_list time_range: {:?}, num: {}, took: {} ms",
        (req.start_time, req.end_time),
        files.len(),
        file_list_took,
    );

    if skip_get_file_list {
        let mut response = search::SearchPartitionResponse::default();
        response.partitions.push([req.start_time, req.end_time]);
        response.max_query_range = max_query_range;
        response.histogram_interval = sql.histogram_interval;
        return Ok(response);
    };

    let nodes = infra_cluster::get_cached_online_querier_nodes(Some(RoleGroup::Interactive))
        .await
        .unwrap_or_default();
    if nodes.is_empty() {
        log::error!("no querier node online");
        return Err(Error::Message("no querier node online".to_string()));
    }
    let cpu_cores = nodes.iter().map(|n| n.cpu_num).sum::<u64>() as usize;

    let (records, original_size) = files.iter().fold((0, 0), |(records, original_size), f| {
        (records + f.records, original_size + f.original_size)
    });
    let mut resp = search::SearchPartitionResponse {
        trace_id: trace_id.to_string(),
        file_num: files.len(),
        records: records as usize,
        original_size: original_size as usize,
        compressed_size: 0, // there is no compressed size in file list
        histogram_interval: sql.histogram_interval,
        max_query_range,
        partitions: vec![],
        order_by: OrderBy::Desc,
    };

    let mut min_step = Duration::try_seconds(1)
        .unwrap()
        .num_microseconds()
        .unwrap();
    if is_aggregate && ts_column.is_some() {
        min_step *= sql.histogram_interval.unwrap_or(1);
    }

    let mut total_secs = resp.original_size / cfg.limit.query_group_base_speed / cpu_cores;
    if total_secs * cfg.limit.query_group_base_speed * cpu_cores < resp.original_size {
        total_secs += 1;
    }
    let mut part_num = max(1, total_secs / cfg.limit.query_partition_by_secs);
    if part_num * cfg.limit.query_partition_by_secs < total_secs {
        part_num += 1;
    }
    // if the partition number is too large, we limit it to 1000
    if part_num > 1000 {
        part_num = 1000;
    }
    let mut step = (req.end_time - req.start_time) / part_num as i64;
    // step must be times of min_step
    if step < min_step {
        step = min_step;
    }
    if step % min_step > 0 {
        step = step - step % min_step;
    }
    // this is to ensure we create partitions less than max_query_range
    if max_query_range > 0 && step > max_query_range {
        step = if min_step < max_query_range {
            max_query_range - max_query_range % min_step
        } else {
            max_query_range
        };
    }

    // Generate partitions by DESC order
    let mut partitions = Vec::with_capacity(part_num);
    let mut end = req.end_time;
    let mut last_partition_step = end % min_step;
    let duration = req.end_time - req.start_time;
    while end > req.start_time {
        let mut start = max(end - step, req.start_time);
        if last_partition_step > 0 && duration > min_step && part_num > 1 {
            partitions.push([end - last_partition_step, end]);
            start -= last_partition_step;
            end -= last_partition_step;
        } else {
            start = max(start - last_partition_step, req.start_time);
        }
        partitions.push([start, end]);
        end = start;
        last_partition_step = 0;
    }
    if partitions.is_empty() {
        partitions.push([req.start_time, req.end_time]);
    }

    // We need to reverse partitions if query is ASC order
    if let Some((field, order_by)) = sql.order_by.first() {
        if field == &ts_column.unwrap() && order_by == &OrderBy::Asc {
            resp.order_by = OrderBy::Asc;
            partitions.reverse();
        }
    }

    resp.partitions = partitions;
    Ok(resp)
}

#[cfg(feature = "enterprise")]
pub async fn query_status() -> Result<search::QueryStatusResponse, Error> {
    // get nodes from cluster
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("query_status: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let nodes = nodes;

    // make cluster request
    let mut tasks = Vec::new();
    for node in nodes.iter().cloned() {
        let node_addr = node.grpc_addr.clone();
        let grpc_span = info_span!(
            "service:search:cluster:grpc_query_status",
            node_id = node.id,
            node_addr = node_addr.as_str(),
        );

        let task = tokio::task::spawn(
            async move {
                let cfg = get_config();
                let mut request = tonic::Request::new(proto::cluster_rpc::QueryStatusRequest {});
                request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));

                opentelemetry::global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(
                        &tracing::Span::current().context(),
                        &mut MetadataMap(request.metadata_mut()),
                    )
                });

                let token: MetadataValue<_> = get_internal_grpc_token()
                    .parse()
                    .map_err(|_| Error::Message("invalid token".to_string()))?;
                let channel = get_cached_channel(&node_addr).await.map_err(|err| {
                    log::error!(
                        "search->grpc: node: {}, connect err: {:?}",
                        &node.grpc_addr,
                        err
                    );
                    server_internal_error("connect search node error")
                })?;
                let mut client = cluster_rpc::search_client::SearchClient::with_interceptor(
                    channel,
                    move |mut req: Request<()>| {
                        req.metadata_mut().insert("authorization", token.clone());
                        Ok(req)
                    },
                );
                client = client
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip)
                    .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
                    .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);
                let response = match client.query_status(request).await {
                    Ok(res) => res.into_inner(),
                    Err(err) => {
                        log::error!(
                            "search->grpc: node: {}, search err: {:?}",
                            &node.grpc_addr,
                            err
                        );
                        if err.code() == tonic::Code::Internal {
                            let err = ErrorCodes::from_json(err.message())?;
                            return Err(Error::ErrorCode(err));
                        }
                        return Err(server_internal_error("search node error"));
                    }
                };
                Ok(response)
            }
            .instrument(grpc_span),
        );
        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(res) => match res {
                Ok(res) => results.push(res),
                Err(err) => {
                    return Err(err);
                }
            },
            Err(e) => {
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    e.to_string(),
                )));
            }
        }
    }

    let mut status = vec![];
    let mut set = HashSet::new();
    for result in results.into_iter().flat_map(|v| v.status.into_iter()) {
        if set.contains(&result.trace_id) {
            continue;
        } else {
            set.insert(result.trace_id.clone());
        }
        let query = result.query.as_ref().map(|query| search::QueryInfo {
            sql: query.sql.clone(),
            start_time: query.start_time,
            end_time: query.end_time,
        });
        let scan_stats = result
            .scan_stats
            .as_ref()
            .map(|scan_stats| search::ScanStats {
                files: scan_stats.files,
                records: scan_stats.records,
                original_size: scan_stats.original_size / 1024 / 1024, // change to MB
                compressed_size: scan_stats.compressed_size / 1024 / 1024, // change to MB
                querier_files: scan_stats.querier_files,
                querier_memory_cached_files: scan_stats.querier_memory_cached_files,
                querier_disk_cached_files: scan_stats.querier_disk_cached_files,
                idx_scan_size: scan_stats.idx_scan_size / 1024 / 1024, // change to MB
                idx_took: scan_stats.idx_took,
            });
        let query_status = if result.is_queue {
            "waiting"
        } else {
            "processing"
        };
        let work_group = if result.work_group.is_none() {
            "Unknown"
        } else if *result.work_group.as_ref().unwrap() == 0 {
            "Short"
        } else {
            "Long"
        };
        status.push(search::QueryStatus {
            trace_id: result.trace_id,
            created_at: result.created_at,
            started_at: result.started_at,
            status: query_status.to_string(),
            user_id: result.user_id,
            org_id: result.org_id,
            stream_type: result.stream_type,
            query,
            scan_stats,
            work_group: work_group.to_string(),
        });
    }

    Ok(search::QueryStatusResponse { status })
}

#[cfg(feature = "enterprise")]
pub async fn cancel_query(
    _org_id: &str,
    trace_id: &str,
) -> Result<search::CancelQueryResponse, Error> {
    // get nodes from cluster
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("cancel_query: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let nodes = nodes;

    // make cluster request
    let mut tasks = Vec::new();
    for node in nodes.iter().cloned() {
        let node_addr = node.grpc_addr.clone();
        let grpc_span = info_span!(
            "service:search:cluster:grpc_cancel_query",
            node_id = node.id,
            node_addr = node_addr.as_str(),
        );

        let trace_id = trace_id.to_string();
        let task = tokio::task::spawn(
            async move {
                let cfg = get_config();
                let mut request =
                    tonic::Request::new(proto::cluster_rpc::CancelQueryRequest { trace_id });
                request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));
                opentelemetry::global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(
                        &tracing::Span::current().context(),
                        &mut MetadataMap(request.metadata_mut()),
                    )
                });

                let token: MetadataValue<_> = get_internal_grpc_token()
                    .parse()
                    .map_err(|_| Error::Message("invalid token".to_string()))?;
                let channel = get_cached_channel(&node_addr).await.map_err(|err| {
                    log::error!(
                        "grpc_cancel_query: node: {}, connect err: {:?}",
                        &node.grpc_addr,
                        err
                    );
                    server_internal_error("connect search node error")
                })?;
                let mut client = cluster_rpc::search_client::SearchClient::with_interceptor(
                    channel,
                    move |mut req: Request<()>| {
                        req.metadata_mut().insert("authorization", token.clone());
                        Ok(req)
                    },
                );
                client = client
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip)
                    .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
                    .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);
                let response: cluster_rpc::CancelQueryResponse =
                    match client.cancel_query(request).await {
                        Ok(res) => res.into_inner(),
                        Err(err) => {
                            log::error!(
                                "grpc_cancel_query: node: {}, search err: {:?}",
                                &node.grpc_addr,
                                err
                            );
                            if err.code() == tonic::Code::Internal {
                                let err = ErrorCodes::from_json(err.message())?;
                                return Err(Error::ErrorCode(err));
                            }
                            return Err(server_internal_error("search node error"));
                        }
                    };
                Ok(response)
            }
            .instrument(grpc_span),
        );
        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(res) => match res {
                Ok(res) => results.push(res),
                Err(err) => {
                    return Err(err);
                }
            },
            Err(e) => {
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    e.to_string(),
                )));
            }
        }
    }

    let mut is_success = false;
    for res in results {
        if res.is_success {
            is_success = true;
            break;
        }
    }

    Ok(search::CancelQueryResponse {
        trace_id: trace_id.to_string(),
        is_success,
    })
}

/// match a source is a valid file or not
#[allow(clippy::too_many_arguments)]
pub async fn match_file(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    time_range: Option<(i64, i64)>,
    source: &FileKey,
    match_min_ts_only: bool,
    is_wal: bool,
    partition_keys: &[StreamPartition],
    equal_items: &[(String, String)],
) -> bool {
    // fast path
    if partition_keys.is_empty() || !source.key.contains('=') {
        return true;
    }

    // slow path
    let mut filters = generate_filter_from_equal_items(equal_items);
    let partition_keys: HashMap<&String, &StreamPartition> =
        partition_keys.iter().map(|v| (&v.field, v)).collect();
    for (key, value) in filters.iter_mut() {
        if let Some(partition_key) = partition_keys.get(key) {
            for val in value.iter_mut() {
                *val = partition_key.get_partition_value(val);
            }
        }
    }
    match_source(
        StreamParams::new(org_id, stream_name, stream_type),
        time_range,
        filters.as_slice(),
        source,
        is_wal,
        match_min_ts_only,
    )
    .await
}

/// before [("a", "3"), ("b", "5"), ("a", "4"), ("b", "6")]
/// after [("a", ["3", "4"]), ("b", ["5", "6"])]
pub fn generate_filter_from_equal_items(
    equal_items: &[(String, String)],
) -> Vec<(String, Vec<String>)> {
    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    for (field, value) in equal_items {
        filters
            .entry(field.to_string())
            .or_default()
            .push(value.to_string());
    }
    filters.into_iter().collect()
}

/// match a source is a valid file or not
pub async fn match_source(
    stream: StreamParams,
    time_range: Option<(i64, i64)>,
    filters: &[(String, Vec<String>)],
    source: &FileKey,
    is_wal: bool,
    match_min_ts_only: bool,
) -> bool {
    if stream.stream_type.eq(&StreamType::Metrics)
        && source.key.starts_with(
            format!(
                "files/{}/{}/{}/",
                stream.org_id, stream.stream_type, stream.org_id
            )
            .as_str(),
        )
    {
        return true;
    }

    // match org_id & table
    if !source.key.starts_with(
        format!(
            "files/{}/{}/{}/",
            stream.org_id, stream.stream_type, stream.stream_name
        )
        .as_str(),
    ) {
        return false;
    }

    // check partition key
    if !filter_source_by_partition_key(&source.key, filters) {
        return false;
    }

    if is_wal {
        return true;
    }

    // check time range
    if source.meta.min_ts == 0 || source.meta.max_ts == 0 {
        return true;
    }
    log::trace!(
        "time range: {:?}, file time: {}-{}, {}",
        time_range,
        source.meta.min_ts,
        source.meta.max_ts,
        source.key
    );

    // match partition clause
    if let Some((time_min, time_max)) = time_range {
        if match_min_ts_only && time_min > 0 {
            return source.meta.min_ts >= time_min && source.meta.min_ts < time_max;
        }
        if time_min > 0 && time_min > source.meta.max_ts {
            return false;
        }
        if time_max > 0 && time_max < source.meta.min_ts {
            return false;
        }
    }
    true
}

/// match a source is a needed file or not, return true if needed
fn filter_source_by_partition_key(source: &str, filters: &[(String, Vec<String>)]) -> bool {
    !filters.iter().any(|(k, v)| {
        let field = format_partition_key(&format!("{k}="));
        find(source, &format!("/{field}"))
            && !v.iter().any(|v| {
                let value = format_partition_key(&format!("{k}={v}"));
                find(source, &format!("/{value}/"))
            })
    })
}

pub fn server_internal_error(error: impl ToString) -> Error {
    Error::ErrorCode(ErrorCodes::ServerInternalError(error.to_string()))
}

#[tracing::instrument(name = "service:search_partition_multi", skip(req))]
pub async fn search_partition_multi(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    req: &search::MultiSearchPartitionRequest,
) -> Result<search::SearchPartitionResponse, Error> {
    let mut res = search::SearchPartitionResponse::default();
    let mut total_rec = 0;
    for query in &req.sql {
        match search_partition(
            trace_id,
            org_id,
            stream_type,
            &search::SearchPartitionRequest {
                start_time: req.start_time,
                end_time: req.end_time,
                sql: query.to_string(),
                encoding: req.encoding,
                regions: req.regions.clone(),
                clusters: req.clusters.clone(),
                query_fn: req.query_fn.clone(),
            },
        )
        .await
        {
            Ok(resp) => {
                if resp.partitions.len() > res.partitions.len() {
                    total_rec += resp.records;
                    res = resp;
                }
            }
            Err(err) => {
                log::error!("search_partition_multi error: {:?}", err);
            }
        };
    }
    res.records = total_rec;
    Ok(res)
}

pub struct MetadataMap<'a>(pub &'a mut tonic::metadata::MetadataMap);

impl<'a> opentelemetry::propagation::Injector for MetadataMap<'a> {
    /// Set a key and value in the MetadataMap.  Does nothing if the key or
    /// value are not valid inputs
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::try_from(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

// generate parquet file search schema
fn generate_search_schema_diff(
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
) -> Result<HashMap<String, DataType>, Error> {
    // calculate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();

    for field in schema.fields().iter() {
        if let Some(latest_field) = schema_latest_map.get(field.name()) {
            if field.data_type() != latest_field.data_type() {
                diff_fields.insert(field.name().clone(), latest_field.data_type().clone());
            }
        }
    }

    Ok(diff_fields)
}

pub fn is_use_inverted_index(sql: &Arc<Sql>) -> (bool, Vec<(String, String)>) {
    // parquet format inverted index only support single table
    if sql.stream_names.len() != 1 {
        return (false, vec![]);
    }

    let cfg = get_config();
    let index_terms = if sql.equal_items.len() == 1 {
        let schema = sql.schemas.values().next().unwrap().schema();
        let stream_settings = infra::schema::unwrap_stream_settings(schema);
        let index_fields = get_stream_setting_index_fields(&stream_settings);
        filter_index_fields(sql.equal_items.values().next().unwrap(), &index_fields)
    } else {
        vec![]
    };

    let use_inverted_index = sql.stream_type != StreamType::Index
        && sql.use_inverted_index
        && cfg.common.inverted_index_enabled
        && !cfg.common.feature_query_without_index
        && (sql.match_items.is_some() || !index_terms.is_empty());

    (use_inverted_index, index_terms)
}

pub fn filter_index_fields(
    items: &[(String, String)],
    index_fields: &[String],
) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for item in items {
        if index_fields.contains(&item.0) {
            result.push(item.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use config::meta::sql::SqlOperator;

    use super::*;

    #[test]
    fn test_matches_by_partition_key_with_str() {
        let path = "files/default/logs/gke-fluentbit/2023/04/14/08/kuberneteshost=gke-dev1/kubernetesnamespacename=ziox-dev/7052558621820981249.parquet";
        let filters = vec![
            (vec![], true),
            (
                vec![("kuberneteshost".to_string(), vec!["gke-dev1".to_string()])],
                true,
            ),
            (
                vec![("kuberneteshost".to_string(), vec!["gke-dev2".to_string()])],
                false,
            ),
            (
                vec![(
                    "kuberneteshost".to_string(),
                    vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                )],
                true,
            ),
            (
                vec![("some_other_key".to_string(), vec!["no-matter".to_string()])],
                true,
            ),
            (
                vec![
                    ("kuberneteshost".to_string(), vec!["gke-dev1".to_string()]),
                    (
                        "kubernetesnamespacename".to_string(),
                        vec!["ziox-dev".to_string()],
                    ),
                ],
                true,
            ),
            (
                vec![
                    ("kuberneteshost".to_string(), vec!["gke-dev1".to_string()]),
                    (
                        "kubernetesnamespacename".to_string(),
                        vec!["abcdefg".to_string()],
                    ),
                ],
                false,
            ),
            (
                vec![
                    ("kuberneteshost".to_string(), vec!["gke-dev2".to_string()]),
                    (
                        "kubernetesnamespacename".to_string(),
                        vec!["ziox-dev".to_string()],
                    ),
                ],
                false,
            ),
            (
                vec![
                    ("kuberneteshost".to_string(), vec!["gke-dev2".to_string()]),
                    (
                        "kubernetesnamespacename".to_string(),
                        vec!["abcdefg".to_string()],
                    ),
                ],
                false,
            ),
            (
                vec![
                    (
                        "kuberneteshost".to_string(),
                        vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                    ),
                    (
                        "kubernetesnamespacename".to_string(),
                        vec!["ziox-dev".to_string()],
                    ),
                ],
                true,
            ),
            (
                vec![
                    (
                        "kuberneteshost".to_string(),
                        vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                    ),
                    (
                        "kubernetesnamespacename".to_string(),
                        vec!["abcdefg".to_string()],
                    ),
                ],
                false,
            ),
            (
                vec![
                    (
                        "kuberneteshost".to_string(),
                        vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                    ),
                    ("some_other_key".to_string(), vec!["no-matter".to_string()]),
                ],
                true,
            ),
        ];
        for (filter, expected) in filters {
            assert_eq!(filter_source_by_partition_key(path, &filter), expected);
        }
    }

    pub fn generate_filter_from_quick_text(
        data: &[(String, String, SqlOperator)],
    ) -> Vec<(&str, Vec<String>)> {
        let quick_text_len = data.len();
        let mut filters = HashMap::with_capacity(quick_text_len);
        for i in 0..quick_text_len {
            let (k, v, op) = &data[i];
            if op == &SqlOperator::And
                || (op == &SqlOperator::Or && (i + 1 == quick_text_len || k == &data[i + 1].0))
            {
                let entry = filters.entry(k.as_str()).or_insert_with(Vec::new);
                entry.push(v.to_string());
            } else {
                filters.clear();
                break;
            }
        }
        filters.into_iter().collect::<Vec<(_, _)>>()
    }

    #[test]
    fn test_matches_by_partition_key_with_sql() {
        use config::meta::sql;
        let path = "files/default/logs/gke-fluentbit/2023/04/14/08/kuberneteshost=gke-dev1/kubernetesnamespacename=ziox-dev/7052558621820981249.parquet";
        let sqls = vec![
            ("SELECT * FROM tbl", true),
            ("SELECT * FROM tbl WHERE kuberneteshost='gke-dev1'", true),
            ("SELECT * FROM tbl WHERE kuberneteshost='gke-dev2'", false),
            ("SELECT * FROM tbl WHERE some_other_key = 'no-matter'", true),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' AND kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' AND kubernetesnamespacename='abcdefg'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' AND kubernetesnamespacename='ziox-dev'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' AND kubernetesnamespacename='abcdefg'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' OR kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' OR kubernetesnamespacename='abcdefg'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' OR kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' OR kubernetesnamespacename='abcdefg'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' OR kuberneteshost='gke-dev2'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1')",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev2')",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2')",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') AND kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') AND kubernetesnamespacename='abcdefg'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') OR kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') OR kubernetesnamespacename='abcdefg'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') OR some_other_key='abcdefg'",
                true,
            ),
        ];

        for (tsql, expected) in sqls {
            let meta = sql::Sql::new(tsql).unwrap();
            let filter = generate_filter_from_quick_text(&meta.quick_text);
            assert_eq!(
                filter_source_by_partition_key(
                    path,
                    &filter
                        .into_iter()
                        .map(|(f1, f2)| (f1.to_owned(), f2))
                        .collect::<Vec<(String, Vec<String>)>>()
                ),
                expected
            );
        }
    }
}
