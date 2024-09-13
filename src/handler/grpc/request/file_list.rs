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

use config::{
    meta::stream::{PartitionTimeLevel, StreamType},
    metrics,
};
use infra::file_list as infra_file_list;
use opentelemetry::global;
use proto::cluster_rpc::{
    filelist_server::Filelist, EmptyRequest, FileByIdQueryRequest, FileData, FileDataList, FileKey,
    FileList, FileListQueryRequest, FileQueryData, IdList, MaxIdResponse,
};
use tonic::{Request, Response, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub struct Filelister;

#[tonic::async_trait]
impl Filelist for Filelister {
    async fn max_id(&self, req: Request<EmptyRequest>) -> Result<Response<MaxIdResponse>, Status> {
        let start = std::time::Instant::now();
        let parent_cx = global::get_text_map_propagator(|prop| {
            prop.extract(&super::MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx);

        let max_id = infra_file_list::get_max_pk_value()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // metrics
        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/file_list/max_id", "200", "", "", ""])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/file_list/max_id", "200", "", "", ""])
            .inc();

        Ok(Response::new(MaxIdResponse { max_id }))
    }

    async fn query(
        &self,
        req: Request<FileListQueryRequest>,
    ) -> Result<Response<FileList>, Status> {
        let start = std::time::Instant::now();
        let parent_cx = global::get_text_map_propagator(|prop| {
            prop.extract(&super::MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx);

        let req: &FileListQueryRequest = req.get_ref();
        let org_id = &req.org_id;
        let stream_type = StreamType::from(req.stream_type.as_str());
        let stream_name = &req.stream_name;
        let time_level = PartitionTimeLevel::from(req.time_level.as_str());
        let time_range = Some((req.start_time, req.end_time));
        let files = infra_file_list::query(
            org_id,
            stream_type,
            stream_name,
            time_level,
            time_range,
            None,
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;
        let items: Vec<FileKey> = files
            .into_iter()
            .map(|(file, meta)| FileKey {
                key: file,
                meta: Some((&meta).into()),
                deleted: false,
                segment_ids: None,
            })
            .collect::<_>();

        // metrics
        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/file_list/query", "200", "", "", ""])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/file_list/query", "200", "", "", ""])
            .inc();

        Ok(Response::new(FileList { items }))
    }

    async fn query_ids(
        &self,
        req: Request<FileListQueryRequest>,
    ) -> Result<Response<IdList>, Status> {
        let start = std::time::Instant::now();
        let parent_cx = global::get_text_map_propagator(|prop| {
            prop.extract(&super::MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx);

        let req: &FileListQueryRequest = req.get_ref();
        let org_id = &req.org_id;
        let stream_type = StreamType::from(req.stream_type.as_str());
        let stream_name = &req.stream_name;
        let time_level = PartitionTimeLevel::from(req.time_level.as_str());
        let time_range = Some((req.start_time, req.end_time));
        let files = infra_file_list::query_ids(
            org_id,
            stream_type,
            stream_name,
            time_level,
            time_range,
            None,
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .into_iter()
        .map(|f| FileQueryData { id: f.id })
        .collect();

        // metrics
        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/file_list/query_ids", "200", "", "", ""])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/file_list/query_ids", "200", "", "", ""])
            .inc();

        Ok(Response::new(IdList { items: files }))
    }

    async fn query_by_ids(
        &self,
        req: Request<FileByIdQueryRequest>,
    ) -> Result<Response<FileDataList>, Status> {
        let start = std::time::Instant::now();
        let parent_cx = global::get_text_map_propagator(|prop| {
            prop.extract(&super::MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx);

        let req: &FileByIdQueryRequest = req.get_ref();
        let ids = &req.ids;
        let files = infra_file_list::query_by_ids(ids)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let items: Vec<FileData> = files
            .into_iter()
            .map(|(id, key, meta)| FileData {
                id,
                file_key: Some(FileKey {
                    key,
                    meta: Some((&meta).into()),
                    deleted: false,
                    segment_ids: None,
                }),
            })
            .collect::<_>();

        // metrics
        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/file_list/query_by_ids", "200", "", "", ""])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/file_list/query_by_ids", "200", "", "", ""])
            .inc();

        Ok(Response::new(FileDataList { items }))
    }
}
