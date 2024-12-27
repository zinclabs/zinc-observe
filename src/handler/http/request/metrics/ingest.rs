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

use std::io::Error;

use actix_web::{http, post, web, HttpRequest, HttpResponse};

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    handler::http::request::{CONTENT_TYPE_JSON, CONTENT_TYPE_PROTO},
    service::metrics,
};

/// _json ingestion API
#[utoipa::path(
    context_path = "/api",
    tag = "Metrics",
    operation_id = "MetricsIngestionJson",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = String, description = "Ingest data (json array)", content_type = "application/json", example = json!([{"__name__":"metrics stream name","__type__":"counter / gauge / histogram / summary","label_name1":"label_value1","label_name2":"label_value2", "_timestamp":1687175143,"value":1.2}])),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "up","successful": 3,"failed": 0}]})),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/ingest/metrics/_json")]
pub async fn json(org_id: web::Path<String>, body: web::Bytes) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    Ok(match metrics::json::ingest(&org_id, body).await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => {
            log::error!("Error processing request {org_id}/metrics/_json: {:?}", e);
            HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                e.to_string(),
            ))
        }
    })
}

/// MetricsIngest
// json example at: https://opentelemetry.io/docs/specs/otel/protocol/file-exporter/#examples
#[utoipa::path(
    context_path = "/api",
    tag = "Metrics",
    operation_id = "PostMetrics",
    request_body(content = String, description = "ExportMetricsServiceRequest", content_type = "application/x-protobuf"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200})),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/v1/metrics")]
pub async fn otlp_metrics_write(
    org_id: web::Path<String>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let content_type = req.headers().get("Content-Type").unwrap().to_str().unwrap();
    if content_type.eq(CONTENT_TYPE_PROTO) {
        metrics::otlp::otlp_proto(&org_id, body).await
    } else if content_type.starts_with(CONTENT_TYPE_JSON) {
        metrics::otlp::otlp_json(&org_id, body).await
    } else {
        Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            "Bad Request".to_string(),
        )))
    }
}
