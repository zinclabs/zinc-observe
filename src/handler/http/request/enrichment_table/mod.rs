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

use actix_multipart::Multipart;
use actix_web::{post, web, HttpRequest, HttpResponse};
use config::SIZE_IN_MB;
use hashbrown::HashMap;

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    service::enrichment_table::{extract_multipart, save_enrichment_data},
};

/// CreateEnrichmentTable
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "CreateUpdateEnrichmentTable",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("table_name" = String, Path, description = "Table name"),
    ),
    responses(
        (status = StatusCode::CREATED, description = "Saved enrichment table", body = HttpResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad Request", body = HttpResponse),
    ),
)]
#[post("/{org_id}/enrichment_tables/{table_name}")]
pub async fn save_enrichment_table(
    path: web::Path<(String, String)>,
    payload: Multipart,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, table_name) = path.into_inner();
    let content_type = req.headers().get("content-type");
    let content_length = match req.headers().get("content-length") {
        None => 0.0,
        Some(content_length) => {
            content_length
                .to_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0)
                / SIZE_IN_MB
        }
    };
    let cfg = config::get_config();
    if content_length > cfg.limit.enrichment_table_limit as f64 {
        return Ok(MetaHttpResponse::bad_request(format!(
            "exceeds allowed limit of {} mb",
            cfg.limit.enrichment_table_limit
        )));
    }
    match content_type {
        Some(content_type) => {
            if content_type
                .to_str()
                .unwrap_or("")
                .starts_with("multipart/form-data")
            {
                let query =
                    web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
                let append_data = match query.get("append") {
                    Some(append_data) => append_data.parse::<bool>().unwrap_or(false),
                    None => false,
                };
                let json_record = extract_multipart(payload).await?;
                save_enrichment_data(&org_id, &table_name, json_record, append_data).await
            } else {
                Ok(MetaHttpResponse::bad_request(
                    "Bad Request, content-type must be multipart/form-data",
                ))
            }
        }
        None => Ok(MetaHttpResponse::bad_request(
            "Bad Request, content-type must be multipart/form-data",
        )),
    }
}
