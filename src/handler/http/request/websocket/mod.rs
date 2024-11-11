pub mod session_handler;
pub mod utils;

use std::collections::HashMap;

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use config::{get_config, meta::stream::StreamType};
use serde::{Deserialize, Serialize};
use session_handler::SessionHandler;
use utils::sessions_cache_utils;

use crate::common::{
    meta::http::HttpResponse as MetaHttpResponse, utils::http::get_stream_type_from_request,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WSQueryParams {
    session_id: String,
    org_id: Option<String>,
    #[serde(rename = "type")]
    stream_type: Option<String>,
}

#[get("{org_id}/ws/{request_id}")]
pub async fn websocket(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    stream: web::Payload,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let cfg = get_config();
    let (org_id, request_id) = path.into_inner();

    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string())?;
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or(StreamType::Logs),
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };
    sessions_cache_utils::insert_session(&request_id, session.clone());
    log::info!(
        "[WEBSOCKET]: Got websocket request for request_id: {}",
        request_id,
    );

    let use_cache = query
        .get("use_cache")
        .map(|s| if s == "true" { true } else { false })
        .unwrap_or_default();
    let use_cache = cfg.common.result_cache_enabled && use_cache;
    let search_type = query.get("search_type").map(|s| s.as_str()).unwrap_or("");

    // Spawn the handler
    let session_handler = SessionHandler::new(
        session,
        msg_stream,
        &user_id,
        &request_id,
        &org_id,
        stream_type,
        use_cache,
        search_type,
    );
    actix_web::rt::spawn(session_handler.run());

    Ok(res)
}
