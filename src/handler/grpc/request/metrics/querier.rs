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

use arrow::ipc::writer::StreamWriter;
use config::{
    ider,
    meta::stream::{FileKey, FileMeta, StreamParams, StreamType},
    metrics,
    utils::{
        file::{get_file_contents, scan_files},
        parquet::{parse_time_range_from_filename, read_metadata_from_bytes},
        schema_ext::SchemaExt,
    },
};
use infra::errors;
use opentelemetry::global;
use tonic::{Request, Response, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
    common::infra::wal,
    handler::grpc::{
        cluster_rpc::{
            metrics_server::Metrics, MetricsQueryRequest, MetricsQueryResponse, MetricsWalFile,
            MetricsWalFileRequest, MetricsWalFileResponse,
        },
        request::MetadataMap,
    },
    service::{promql::search as SearchService, search::match_source},
};

pub struct MetricsQuerier;

#[tonic::async_trait]
impl Metrics for MetricsQuerier {
    #[tracing::instrument(name = "grpc:metrics:query", skip_all, fields(org_id = req.get_ref().org_id))]
    async fn query(
        &self,
        req: Request<MetricsQueryRequest>,
    ) -> Result<Response<MetricsQueryResponse>, Status> {
        let start = std::time::Instant::now();
        let parent_cx =
            global::get_text_map_propagator(|prop| prop.extract(&MetadataMap(req.metadata())));
        tracing::Span::current().set_parent(parent_cx);

        let req: &MetricsQueryRequest = req.get_ref();
        let org_id = &req.org_id;
        let stream_type = StreamType::Metrics.to_string();
        let result = SearchService::grpc::search(req).await.map_err(|err| {
            let time = start.elapsed().as_secs_f64();
            metrics::GRPC_RESPONSE_TIME
                .with_label_values(&["/metrics/query", "500", org_id, "", &stream_type])
                .observe(time);
            metrics::GRPC_INCOMING_REQUESTS
                .with_label_values(&["/metrics/query", "500", org_id, "", &stream_type])
                .inc();
            let message = if let errors::Error::ErrorCode(code) = err {
                code.to_json()
            } else {
                err.to_string()
            };
            Status::internal(message)
        })?;

        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/metrics/query", "200", org_id, "", &stream_type])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/metrics/query", "200", org_id, "", &stream_type])
            .inc();

        Ok(Response::new(result))
    }

    #[tracing::instrument(name = "grpc:metrics:wal_file", skip_all, fields(org_id = req.get_ref().org_id, stream_name = req.get_ref().stream_name))]
    async fn wal_file(
        &self,
        req: Request<MetricsWalFileRequest>,
    ) -> Result<Response<MetricsWalFileResponse>, Status> {
        let start = std::time::Instant::now();
        let start_time = req.get_ref().start_time;
        let end_time = req.get_ref().end_time;
        let org_id = &req.get_ref().org_id;
        let stream_name = &req.get_ref().stream_name;
        let filters = req
            .get_ref()
            .filters
            .iter()
            .map(|f| (f.field.as_str(), f.value.clone()))
            .collect::<Vec<_>>();
        let mut resp = MetricsWalFileResponse::default();

        // get memtable records
        let mut mem_data = ingester::read_from_memtable(
            org_id,
            StreamType::Metrics.to_string().as_str(),
            stream_name,
            Some((start_time, end_time)),
        )
        .await
        .unwrap_or_default();

        // get immutable records
        mem_data.extend(
            ingester::read_from_immutable(
                org_id,
                StreamType::Metrics.to_string().as_str(),
                stream_name,
                Some((start_time, end_time)),
            )
            .await
            .unwrap_or_default(),
        );

        // write memory data into arrow files
        let mut arrow_files = vec![];
        for (schema, batches) in mem_data {
            let mut size = 0;
            let mut body = Vec::new();
            let mut writer = StreamWriter::try_new(&mut body, &schema).unwrap();
            for batch in batches {
                size += batch.data_json_size;
                writer.write(&batch.data).unwrap();
            }
            writer.finish().unwrap();
            drop(writer);

            let name = format!("{}.arrow", ider::generate());
            let schema_key = schema.hash_key();
            arrow_files.push(MetricsWalFile {
                name,
                schema_key,
                body,
                size: size as i64,
            });
        }

        // get parquet files
        let wal_dir =
            match std::path::Path::new(&config::get_config().common.data_wal_dir).canonicalize() {
                Ok(path) => {
                    let mut path = path.to_str().unwrap().to_string();
                    // Hack for windows
                    if path.starts_with("\\\\?\\") {
                        path = path[4..].to_string();
                        path = path.replace('\\', "/");
                    }
                    path
                }
                Err(_) => {
                    return Ok(Response::new(resp));
                }
            };

        let pattern = format!("{wal_dir}/files/{org_id}/metrics/{stream_name}/");
        let files = scan_files(&pattern, "parquet", None).unwrap_or_default();

        if arrow_files.is_empty() && files.is_empty() {
            return Ok(Response::new(resp));
        }

        // lock theses files
        let files = files
            .iter()
            .map(|f| {
                f.strip_prefix(&wal_dir)
                    .unwrap()
                    .to_string()
                    .replace('\\', "/")
                    .trim_start_matches('/')
                    .to_string()
            })
            .collect::<Vec<_>>();
        wal::lock_files(&files);

        for file in files.iter() {
            // check time range by filename
            let (file_min_ts, file_max_ts) = parse_time_range_from_filename(file);
            if (file_min_ts > 0 && file_max_ts > 0)
                && ((end_time > 0 && file_min_ts > end_time)
                    || (start_time > 0 && file_max_ts < start_time))
            {
                log::debug!(
                    "skip wal parquet file: {} time_range: [{},{}]",
                    &file,
                    file_min_ts,
                    file_max_ts
                );
                wal::release_files(&[file.clone()]);
                continue;
            }
            // filter by partition keys
            let file_key = FileKey::new(
                file,
                FileMeta {
                    min_ts: file_min_ts,
                    max_ts: file_max_ts,
                    ..Default::default()
                },
                false,
            );
            if !match_source(
                StreamParams::new(org_id, stream_name, StreamType::Metrics),
                Some((start_time, end_time)),
                filters.as_slice(),
                &file_key,
                true,
                false,
            )
            .await
            {
                wal::release_files(&[file.clone()]);
                continue;
            }
            // check time range by parquet metadata
            let source_file = wal_dir.to_string() + "/" + file;
            let Ok(body) = get_file_contents(&source_file) else {
                continue;
            };
            let body = bytes::Bytes::from(body);
            let parquet_meta = read_metadata_from_bytes(&body).await.unwrap_or_default();
            if (start_time > 0 && parquet_meta.max_ts < start_time)
                || (end_time > 0 && parquet_meta.min_ts > end_time)
            {
                log::debug!(
                    "skip wal parquet file: {} time_range: [{},{}]",
                    file,
                    parquet_meta.min_ts,
                    parquet_meta.max_ts
                );
                continue;
            }

            let columns = file.split('/').collect::<Vec<&str>>();
            let len = columns.len();
            let name = columns[len - 1].to_string();
            let schema_key = columns[len - 2].to_string();
            resp.files.push(MetricsWalFile {
                name,
                schema_key,
                body: body.to_vec(),
                size: parquet_meta.original_size,
            });
        }

        // release all files
        wal::release_files(&files);

        // append arrow files
        resp.files.extend(arrow_files);

        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/metrics/wal_file", "200", org_id, stream_name, "metrics"])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/metrics/wal_file", "200", org_id, stream_name, "metrics"])
            .inc();

        Ok(Response::new(resp))
    }
}
