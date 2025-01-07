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

use std::io::{prelude::*, Error};

use actix_multipart::form::{bytes::Bytes, MultipartForm};
use actix_web::{post, web, HttpResponse};
use config::utils::json;
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};

use crate::{
    common::meta::{
        http::HttpResponse as MetaHttpResponse, ingestion::IngestionRequest,
        middleware_data::RumExtraData,
    },
    service::logs,
};

pub const RUM_LOG_STREAM: &str = "_rumlog";
pub const RUM_SESSION_REPLAY_STREAM: &str = "_sessionreplay";
pub const RUM_DATA_STREAM: &str = "_rumdata";

/// Multipart form data being ingested in the form of session-replay
#[derive(MultipartForm)]
pub struct SegmentEvent {
    pub segment: Bytes,
    pub event: Bytes,
}

#[derive(Serialize, Deserialize)]
pub struct SegmentEventSerde {
    pub segment: String,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(rename = "raw_segment_size")]
    pub raw_segment_size: i64,
    #[serde(rename = "compressed_segment_size")]
    pub compressed_segment_size: i64,
    pub start: i64,
    pub end: i64,
    #[serde(rename = "creation_reason")]
    pub creation_reason: String,
    #[serde(rename = "records_count")]
    pub records_count: i64,
    #[serde(rename = "has_full_snapshot")]
    pub has_full_snapshot: bool,
    #[serde(rename = "index_in_view")]
    pub index_in_view: i64,
    pub source: String,
    pub application: Application,
    pub session: Session,
    pub view: View,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: String,
}

/// Rum data ingestion API
#[utoipa::path(
    context_path = "/rum",
    tag = "Rum",
    operation_id = "RumIngestionMulti",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = String, description = "Ingest data (multiple line json)", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "olympics","successful": 3,"failed": 0}]})),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v1/{org_id}/rum")]
pub async fn data(
    path: web::Path<String>,
    body: web::Bytes,
    rum_query_data: web::ReqData<RumExtraData>,
) -> Result<HttpResponse, Error> {
    let org_id: String = path.into_inner();
    let extend_json = &rum_query_data.data;
    Ok(
        match logs::ingest::ingest(
            0,
            &org_id,
            RUM_DATA_STREAM,
            IngestionRequest::RUM(&body),
            "",
            Some(extend_json),
        )
        .await
        {
            Ok(v) => MetaHttpResponse::json(v),
            Err(e) => MetaHttpResponse::bad_request(e),
        },
    )
}

/// Rum log ingestion API
#[utoipa::path(
    context_path = "/rum",
    tag = "Rum",
    operation_id = "LogIngestionJson",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = String, description = "Ingest data (json array)", content_type = "application/json", example = json!([{"Year": 1896, "City": "Athens", "Sport": "Aquatics", "Discipline": "Swimming", "Athlete": "Alfred", "Country": "HUN"},{"Year": 1896, "City": "Athens", "Sport": "Aquatics", "Discipline": "Swimming", "Athlete": "HERSCHMANN", "Country":"CHN"}])),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "olympics","successful": 3,"failed": 0}]})),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v1/{org_id}/logs")]
pub async fn log(
    path: web::Path<String>,
    body: web::Bytes,
    rum_query_data: web::ReqData<RumExtraData>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let extend_json = &rum_query_data.data;
    Ok(
        match logs::ingest::ingest(
            0,
            &org_id,
            RUM_LOG_STREAM,
            IngestionRequest::RUM(&body),
            "",
            Some(extend_json),
        )
        .await
        {
            Ok(v) => MetaHttpResponse::json(v),
            Err(e) => MetaHttpResponse::bad_request(e),
        },
    )
}

/// Rum session-replay ingestion API
#[utoipa::path(
    context_path = "/rum",
    tag = "Rum",
    operation_id = "ReplayIngestionJson",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = String, description = "Ingest data (json array)", content_type = "application/json", example = json!([{"Year": 1896, "City": "Athens", "Sport": "Aquatics", "Discipline": "Swimming", "Athlete": "Alfred", "Country": "HUN"},{"Year": 1896, "City": "Athens", "Sport": "Aquatics", "Discipline": "Swimming", "Athlete": "HERSCHMANN", "Country":"CHN"}])),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "olympics","successful": 3,"failed": 0}]})),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v1/{org_id}/replay")]
pub async fn sessionreplay(
    path: web::Path<String>,
    payload: MultipartForm<SegmentEvent>,
    rum_query_data: web::ReqData<RumExtraData>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let mut segment_payload = String::new();
    if let Err(_e) =
        ZlibDecoder::new(&payload.segment.data[..]).read_to_string(&mut segment_payload)
    {
        return Ok(MetaHttpResponse::bad_request(
            "Failed to decompress the incoming payload",
        ));
    }

    let event: Event = json::from_slice(&payload.event.data[..]).unwrap();
    let ingestion_payload = SegmentEventSerde {
        segment: segment_payload,
        event,
    };

    let body = json::to_vec(&ingestion_payload).unwrap();
    let extend_json = &rum_query_data.data;
    Ok(
        match logs::ingest::ingest(
            0,
            &org_id,
            RUM_SESSION_REPLAY_STREAM,
            IngestionRequest::RUM(&body.into()),
            "",
            Some(extend_json),
        )
        .await
        {
            Ok(v) => MetaHttpResponse::json(v),
            Err(e) => MetaHttpResponse::bad_request(e),
        },
    )
}
