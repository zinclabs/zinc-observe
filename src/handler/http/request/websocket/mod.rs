pub mod session_handler;
pub mod utils;

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use config::get_config;
use session_handler::SessionHandler;
use utils::sessions_cache_utils;

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

    sessions_cache_utils::insert_session(&request_id, session.clone());
    log::info!(
        "[WS_HANDLER]: Node Role: {} Got websocket request for request_id: {}",
        cfg.common.node_role,
        request_id,
    );

    // Spawn the handler
    let session_handler = SessionHandler::new(session, msg_stream, &user_id, &request_id, &org_id);
    actix_web::rt::spawn(session_handler.run());

    Ok(res)
}
