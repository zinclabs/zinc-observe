pub mod session_handler;
pub mod utils;

use std::collections::HashMap;

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use config::meta::stream::StreamType;
use serde::{Deserialize, Serialize};
use session_handler::SessionHandler;
use tokio_stream::StreamExt;
use utils::sessions_cache_utils;

use crate::common::{
    meta::{self, http::HttpResponse as MetaHttpResponse},
    utils::http::get_stream_type_from_request,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WSQueryParams {
    request_id: String,
    org_id: Option<String>,
    #[serde(rename = "type")]
    stream_type: Option<String>,
}

#[get("/ws/{user_id}")]
pub async fn websocket(
    user_id: web::Path<String>,
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<WSQueryParams>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let user_id = user_id.into_inner();
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    log::debug!(
        "[WEBSOCKET]: Got websocket request for user_id: {} and request_id: {}",
        user_id,
        &query.request_id,
    );

    sessions_cache_utils::insert_session(&query.request_id, session.clone());

    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = match get_stream_type_from_request(&query) {
        Ok(v) => v.unwrap_or(StreamType::Logs),
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };
    let request_id = query.get("request_id").map(|s| s.as_str()).unwrap_or("");
    let org_id = query.get("org_id").map(|s| s.as_str()).unwrap_or("");
    let use_cache = query
        .get("use_cache")
        .map(|s| if s == "true" { true } else { false })
        .unwrap_or_default();
    let search_type = query.get("search_type").map(|s| s.as_str()).unwrap_or("");

    // Spawn the handler
    let session_handler = SessionHandler::new(
        session,
        msg_stream,
        &user_id,
        request_id,
        org_id,
        stream_type,
        use_cache,
        search_type,
    );
    actix_web::rt::spawn(session_handler.run());

    Ok(res)
}
