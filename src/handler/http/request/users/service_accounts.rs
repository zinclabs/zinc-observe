use actix_web::{post, web, Error, HttpRequest, HttpResponse};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::{
    common::{auditor::AuditMessage, infra::config::get_config as get_o2_config},
    dex::service::auth::get_dex_jwks,
};

#[cfg(feature = "enterprise")]
use crate::service::self_reporting::audit;
#[cfg(feature = "enterprise")]
use crate::{common::utils::jwt::verify_decode_token, handler::http::auth::jwt::process_token};

#[cfg(feature = "enterprise")]
#[post("/token")]
pub async fn exchange_token(
    req: HttpRequest,
    body: web::Json<o2_enterprise::enterprise::dex::meta::auth::TokenExchangeRequest>,
) -> Result<HttpResponse, Error> {
    let result =
        o2_enterprise::enterprise::dex::service::token_exchange::exchange_token(&body.into_inner())
            .await;

    let mut audit_message = AuditMessage {
        user_email: "".to_string(),
        org_id: "".to_string(),
        method: req.method().to_string(),
        path: req.path().to_string(),
        body: "".to_string(),
        query_params: req.query_string().to_string(),
        response_code: 200,
        _timestamp: chrono::Utc::now().timestamp_micros(),
    };
    match result {
        Ok(response) => {
            let keys = get_dex_jwks().await;
            let token_ver = verify_decode_token(
                &response.access_token,
                &keys,
                &get_o2_config().dex.client_id,
                true,
                false,
            )
            .await;
            match token_ver {
                Ok(res) => {
                    audit_message.user_email = res.0.user_email.clone();
                    process_token(res).await
                }
                Err(e) => {
                    audit_message.response_code = 401;
                    audit_message._timestamp = chrono::Utc::now().timestamp_micros();
                    audit(audit_message).await;
                    return Ok(HttpResponse::Unauthorized().json(e.to_string()));
                }
            }
            audit_message._timestamp = chrono::Utc::now().timestamp_micros();
            audit(audit_message).await;
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            log::error!("Error: {}", e);
            audit_message.response_code = 401;
            audit_message._timestamp = chrono::Utc::now().timestamp_micros();
            audit(audit_message).await;
            Ok(HttpResponse::Unauthorized().json(e.to_string()))
        }
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/token")]
pub async fn exchange_token(
    _req: HttpRequest,
    _body: web::Json<String>,
) -> Result<HttpResponse, Error> {
    use actix_web::error::ErrorForbidden;
    Err(ErrorForbidden("Not allowed"))
}
