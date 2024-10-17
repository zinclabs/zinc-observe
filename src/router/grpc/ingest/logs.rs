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

use config::meta::cluster::get_internal_grpc_token;
use opentelemetry_proto::tonic::collector::logs::v1::{
    logs_service_client::LogsServiceClient, logs_service_server::LogsService,
    ExportLogsServiceRequest, ExportLogsServiceResponse,
};
use tonic::{codec::CompressionEncoding, metadata::MetadataValue, Request, Response, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::service::{grpc::get_ingester_channel, search::MetadataMap};

#[derive(Default)]
pub struct LogsServer;

#[tonic::async_trait]
impl LogsService for LogsServer {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let start = std::time::Instant::now();
        let (metadata, extensions, message) = request.into_parts();

        let cfg = config::get_config();
        // basic validation
        if !metadata.contains_key(&cfg.grpc.org_header_key) {
            return Err(Status::invalid_argument(format!(
                "Please specify organization id with header key '{}' ",
                &cfg.grpc.org_header_key
            )));
        }

        // call ingester
        let mut request = Request::from_parts(metadata, extensions, message);
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject_context(
                &tracing::Span::current().context(),
                &mut MetadataMap(request.metadata_mut()),
            )
        });

        let token: MetadataValue<_> = get_internal_grpc_token()
            .parse()
            .map_err(|_| Status::internal("invalid token".to_string()))?;
        let (addr, channel) = get_ingester_channel().await?;
        let client = LogsServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            Ok(req)
        });
        match client
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip)
            .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
            .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
            .export(request)
            .await
        {
            Ok(res) => {
                if res.get_ref().partial_success.is_some() {
                    log::error!(
                        "[Router:LOGS] export partial_success node: {addr}, response: {:?}",
                        res.get_ref(),
                    );
                }
                Ok(res)
            }
            Err(e) => {
                let time = start.elapsed().as_millis() as usize;
                log::error!("[Router:LOGS] export node: {addr}, status: {e}, took: {time} ms");
                Err(e)
            }
        }
    }
}
