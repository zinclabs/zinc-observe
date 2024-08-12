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

use async_trait::async_trait;
use config::utils::json;
use proto::cluster_rpc::{ingest_server::Ingest, IngestionRequest, IngestionResponse, StreamType};
use tonic::{Request, Response, Status};

use crate::service::ingestion::create_log_ingestion_req;

#[derive(Debug, Default)]
pub struct Ingester;

#[async_trait]
impl Ingest for Ingester {
    async fn ingest(
        &self,
        request: Request<IngestionRequest>,
    ) -> Result<Response<IngestionResponse>, Status> {
        // let metadata = request.metadata().clone();
        let req = request.into_inner();
        let org_id = req.org_id;
        let stream_name = req.stream_name;
        let in_data = req.data.unwrap_or_default();

        let resp = match StreamType::try_from(req.stream_type) {
            Ok(StreamType::Logs) => {
                let log_ingestion_type = req.ingestion_type.unwrap_or_default();
                let data = bytes::Bytes::from(in_data.data);
                match create_log_ingestion_req(log_ingestion_type, &data) {
                    Err(e) => Err(e),
                    Ok(ingestion_req) => crate::service::logs::ingest::ingest(
                        &org_id,
                        &stream_name,
                        ingestion_req,
                        "",
                        None,
                    )
                    .await
                    .map_or_else(Err, |_| Ok(())),
                }
            }
            Ok(StreamType::EnrichmentTables) => {
                let json_records: Vec<json::Map<String, json::Value>> =
                    json::from_slice(&in_data.data).unwrap_or({
                        let vec_value: Vec<json::Value> = json::from_slice(&in_data.data).unwrap();
                        vec_value
                            .into_iter()
                            .filter_map(|v| match v {
                                json::Value::Object(map) => Some(map),
                                _ => None,
                            })
                            .collect()
                    });
                crate::service::enrichment_table::save_enrichment_data(
                    &org_id,
                    &stream_name,
                    json_records,
                    true,
                )
                .await
                .map_or_else(
                    |e| {
                        Err(anyhow::anyhow!(
                            "Internal gPRC ingestion service errors saving enrichment data: {}",
                            e.to_string()
                        ))
                    },
                    |res| {
                        if res.status().as_u16() != 200 {
                            log::error!(
                                "Internal gPRC ingestion service errors saving enrichment data: {:?}",
                                res
                            );
                            Err(anyhow::anyhow!(
                                "Internal gPRC ingestion service errors saving enrichment data: {}",
                                res.error().map_or("".to_string(), |err| err.to_string())
                            ))
                        } else {
                            Ok(())
                        }
                    },
                )
            }
            _ => Err(anyhow::anyhow!(
                "Internal gPRC ingestion service currently only supports Logs and EnrichmentTables",
            )),
        };

        let reply = match resp {
            Ok(_) => IngestionResponse {
                status_code: 200,
                message: "OK".to_string(),
            },
            Err(err) => IngestionResponse {
                status_code: 500,
                message: err.to_string(),
            },
        };
        Ok(Response::new(reply))
    }
}
