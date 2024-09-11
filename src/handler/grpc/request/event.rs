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

use chrono::{Duration, Utc};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        cluster::{Role, RoleGroup},
        stream::FileKey,
    },
    metrics,
    utils::parquet::read_recordbatch_from_bytes,
};
use hashbrown::HashSet;
use infra::{file_list as infra_file_list, schema::STREAM_SCHEMAS_FIELDS};
use opentelemetry::global;
use proto::cluster_rpc::{event_server::Event, EmptyResponse, FileList};
use tonic::{Request, Response, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::common::infra::cluster::get_node_from_consistent_hash;

pub struct Eventer;

#[tonic::async_trait]
impl Event for Eventer {
    async fn send_file_list(
        &self,
        req: Request<FileList>,
    ) -> Result<Response<EmptyResponse>, Status> {
        let start = std::time::Instant::now();
        let parent_cx = global::get_text_map_propagator(|prop| {
            prop.extract(&super::MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx);

        let req = req.get_ref();
        let put_items = req
            .items
            .iter()
            .filter(|v| !v.deleted)
            .map(FileKey::from)
            .collect::<Vec<_>>();
        let del_items = req
            .items
            .iter()
            .filter(|v| v.deleted)
            .map(|v| v.key.clone())
            .collect::<Vec<_>>();
        let cfg = get_config();
        // Warning: external meta store should not accept any file list
        // querier and compactor can accept add new files
        // ingester only accept remove old files
        if !cfg.common.meta_store_external {
            if LOCAL_NODE.is_ingester() || LOCAL_NODE.is_compactor() {
                if let Err(e) = infra_file_list::batch_add(&put_items).await {
                    // metrics
                    let time = start.elapsed().as_secs_f64();
                    metrics::GRPC_RESPONSE_TIME
                        .with_label_values(&["/event/send_file_list", "500", "", "", ""])
                        .observe(time);
                    metrics::GRPC_INCOMING_REQUESTS
                        .with_label_values(&["/event/send_file_list", "500", "", "", ""])
                        .inc();
                    return Err(Status::internal(e.to_string()));
                }
            }
            if let Err(e) = infra_file_list::batch_remove(&del_items).await {
                // metrics
                let time = start.elapsed().as_secs_f64();
                metrics::GRPC_RESPONSE_TIME
                    .with_label_values(&["/event/send_file_list", "500", "", "", ""])
                    .observe(time);
                metrics::GRPC_INCOMING_REQUESTS
                    .with_label_values(&["/event/send_file_list", "500", "", "", ""])
                    .inc();
                return Err(Status::internal(e.to_string()));
            }
        }

        // cache latest files for querier
        if cfg.memory_cache.cache_latest_files && LOCAL_NODE.is_querier() {
            let mut cached_field_stream = HashSet::new();
            for item in put_items.iter() {
                let Some(node_name) = get_node_from_consistent_hash(
                    &item.key,
                    &Role::Querier,
                    Some(RoleGroup::Interactive),
                )
                .await
                else {
                    continue; // no querier node
                };
                if LOCAL_NODE.name.ne(&node_name) {
                    continue; // not this node
                }
                if infra::cache::file_data::download("download", &item.key)
                    .await
                    .is_ok()
                    && cfg.limit.quick_mode_file_list_enabled
                {
                    let columns = item.key.split('/').collect::<Vec<&str>>();
                    if columns[2] != "logs" {
                        continue; // only cache fields for logs
                    }
                    let stream_key = columns[1..4].join("/");
                    if cached_field_stream.contains(&stream_key) {
                        continue;
                    }
                    if cache_latest_fields(&stream_key, &item.key).await.is_ok() {
                        cached_field_stream.insert(stream_key);
                    }
                }
            }
        }

        // metrics
        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/event/send_file_list", "200", "", "", ""])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/event/send_file_list", "200", "", "", ""])
            .inc();

        Ok(Response::new(EmptyResponse {}))
    }
}

async fn cache_latest_fields(stream: &str, file: &str) -> Result<(), anyhow::Error> {
    let fr = STREAM_SCHEMAS_FIELDS.read().await;
    let field_cache_time = fr.get(stream).map(|v| v.0).unwrap_or(0);
    drop(fr);

    if field_cache_time
        + Duration::try_seconds(get_config().limit.quick_mode_file_list_interval)
            .unwrap()
            .num_microseconds()
            .unwrap()
        >= Utc::now().timestamp_micros()
    {
        return Ok(());
    }

    let buf = match infra::cache::file_data::memory::get(file, None).await {
        Some(buf) => Some(buf),
        _ => infra::cache::file_data::disk::get(file, None).await,
    };
    let Some(buf) = buf else {
        return Ok(());
    };

    let (schema, batches) = read_recordbatch_from_bytes(&buf).await?;
    let mut new_batch = arrow::compute::concat_batches(&schema, &batches)?;
    // delete all null values column
    let mut null_columns = Vec::new();
    for i in 0..new_batch.num_columns() {
        let fi = i - null_columns.len();
        if new_batch.column(fi).null_count() == new_batch.num_rows() {
            null_columns.push(i);
            new_batch.remove_column(fi);
        }
    }
    let new_chema = if null_columns.is_empty() {
        schema
    } else {
        new_batch.schema()
    };
    let mut fields = new_chema
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect::<Vec<_>>();
    fields.sort();
    if fields.is_empty() {
        return Ok(());
    }

    log::debug!("cached latest stream: {}, fields: {}", stream, fields.len());

    let mut fw = STREAM_SCHEMAS_FIELDS.write().await;
    let entry = fw.entry(stream.to_string()).or_insert((0, Vec::new()));
    entry.0 = Utc::now().timestamp_micros();
    entry.1 = fields;
    drop(fw);

    Ok(())
}
