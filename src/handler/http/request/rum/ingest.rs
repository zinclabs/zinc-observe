// Copyright 2023 Zinc Labs Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_multipart::form::{bytes::Bytes, MultipartForm};
use actix_web::{post, web, HttpResponse};
use ahash::AHashMap;
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};
use std::io::{prelude::*, Error};

use crate::common::{
    meta::{http::HttpResponse as MetaHttpResponse, middleware_data::RumExtraData},
    utils::json,
};
use crate::service::logs;

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

/** Rum data ingestion API */
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
        (status = 200, description="Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "olympics","successful": 3,"failed": 0}]})),
        (status = 500, description="Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v1/{org_id}/rum")]
pub async fn data(
    path: web::Path<String>,
    body: web::Bytes,
    thread_id: web::Data<usize>,
    rum_query_data: web::ReqData<RumExtraData>,
) -> Result<HttpResponse, Error> {
    let org_id: String = path.into_inner();
    let extend_json = &rum_query_data.data;
    ingest_multi_json(&org_id, RUM_DATA_STREAM, body, extend_json, **thread_id).await
}

/** Rum log ingestion API */
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
        (status = 200, description="Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "olympics","successful": 3,"failed": 0}]})),
        (status = 500, description="Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v1/{org_id}/logs")]
pub async fn log(
    path: web::Path<String>,
    body: web::Bytes,
    thread_id: web::Data<usize>,
    rum_query_data: web::ReqData<RumExtraData>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let extend_json = &rum_query_data.data;
    ingest_multi_json(&org_id, RUM_LOG_STREAM, body, extend_json, **thread_id).await
}

/** Rum session-replay ingestion API */
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
        (status = 200, description="Success", content_type = "application/json", body = IngestionResponse, example = json!({"code": 200,"status": [{"name": "olympics","successful": 3,"failed": 0}]})),
        (status = 500, description="Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v1/{org_id}/replay")]
pub async fn sessionreplay(
    path: web::Path<String>,
    payload: MultipartForm<SegmentEvent>,
    thread_id: web::Data<usize>,
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

    let extend_json = &rum_query_data.data;
    let body = json::to_vec(&ingestion_payload).unwrap();
    ingest_multi_json(
        &org_id,
        RUM_SESSION_REPLAY_STREAM,
        body.into(),
        extend_json,
        **thread_id,
    )
    .await
}

async fn ingest_multi_json(
    org_id: &str,
    stream_name: &str,
    body: web::Bytes,
    extend_json: &AHashMap<String, serde_json::Value>,
    thread_id: usize,
) -> Result<HttpResponse, Error> {
    Ok(
        match logs::multi::ingest_with_keys(org_id, stream_name, body, extend_json, thread_id).await
        {
            Ok(v) => MetaHttpResponse::json(v),
            Err(e) => MetaHttpResponse::bad_request(e),
        },
    )
}
