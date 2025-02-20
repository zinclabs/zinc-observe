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

use std::collections::HashMap;

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use config::{get_config, utils::json};
use o2_enterprise::enterprise::cloud::billings as o2_cloud_billings;

use crate::{
    common::{
        meta::http::HttpResponse as MetaHttpResponse,
        utils::{auth::UserEmail, redirect_response::RedirectResponseBuilder},
    },
    handler::http::models::billings::{
        CheckoutSessionDetailRequestQuery, ListInvoicesResponseBody, ListSubscriptionResponseBody,
    },
    service::organization,
};

pub mod org_usage;

/// GetSubscriptionUrl
#[utoipa::path(
    context_path = "/api",
    tag = "Billings",
    operation_id = "GetSubscriptionUrl",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",   content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/billings/hosted_subscription_url")]
pub async fn create_checkout_session(
    path: web::Path<String>,
    user_email: UserEmail,
    req: HttpRequest,
) -> impl Responder {
    let org_id = path.into_inner();
    let email = user_email.user_id.as_str();

    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let Some(sub_type) = query.get("plan") else {
        return o2_cloud_billings::BillingError::SubTypeMissing.into_http_response();
    };

    if organization::get_org(&org_id).await.is_none() {
        return o2_cloud_billings::BillingError::OrgNotFound.into_http_response();
    }

    match o2_cloud_billings::create_checkout_session(
        &get_config().common.web_url,
        email,
        &org_id,
        sub_type,
    )
    .await
    {
        Err(err) => err.into_http_response(),
        Ok(o2_cloud_billings::CheckoutResult::RedirectUrl(redirect_url)) => {
            log::debug!("redirect url: {redirect_url}");
            RedirectResponseBuilder::new(&redirect_url)
                .build()
                .redirect_http()
        }
        Ok(o2_cloud_billings::CheckoutResult::Session(checkout_session)) => {
            log::debug!("created checkout session");
            MetaHttpResponse::json(checkout_session)
        }
    }
}

/// ProcessSessionDetail
#[utoipa::path(
    context_path = "/api",
    tag = "Billings",
    operation_id = "ProcessSessionDetail",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",   content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/billings/checkout_session_detail")]
pub async fn process_session_detail(
    path: web::Path<String>,
    query: web::Query<CheckoutSessionDetailRequestQuery>,
) -> impl Responder {
    let org_id = path.into_inner();
    if query.status != "success" {
        return o2_cloud_billings::BillingError::InvalidStatus.into_http_response();
    }

    if organization::get_org(&org_id).await.is_none() {
        return o2_cloud_billings::BillingError::OrgNotFound.into_http_response();
    }

    log::debug!("handling checkout session detail");
    match o2_cloud_billings::process_checkout_session_details(
        &org_id,
        &query.session_id,
        &query.plan,
    )
    .await
    {
        Err(e) => e.into_http_response(),
        Ok(()) => {
            let redirect_url = format!(
                "{}/web/billings/plans?org_identifier={}",
                &get_config().common.web_url,
                &org_id
            );
            RedirectResponseBuilder::new(&redirect_url)
                .build()
                .redirect_http()
        }
    }
}

/// Unsubscribe
#[utoipa::path(
    context_path = "/api",
    tag = "Billings",
    operation_id = "Unsubscribe",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",   content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/billings/unsubscribe")]
pub async fn unsubscribe(path: web::Path<String>, user_email: UserEmail) -> impl Responder {
    let org_id = path.into_inner();
    let email = user_email.user_id.as_str();

    if organization::get_org(&org_id).await.is_none() {
        return o2_cloud_billings::BillingError::OrgNotFound.into_http_response();
    }

    match o2_cloud_billings::unsubscribe(&org_id, email).await {
        Err(err) => err.into_http_response(),
        Ok(()) => {
            HttpResponse::Ok().body("Subscription will be cancelled at the end of billing cycle.")
        }
    }
}

/// ListInvoices
#[utoipa::path(
    context_path = "/api",
    tag = "Billings",
    operation_id = "ListInvoices",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/billings/invoices")]
pub async fn list_invoices(path: web::Path<String>, user_email: UserEmail) -> impl Responder {
    let org_id = path.into_inner();
    let email = user_email.user_id.as_str();
    if organization::get_org(&org_id).await.is_none() {
        return o2_cloud_billings::BillingError::OrgNotFound.into_http_response();
    }
    match o2_cloud_billings::list_invoice(&org_id, email).await {
        Ok(invoices) => {
            let body = ListInvoicesResponseBody {
                invoices: invoices.unwrap_or_default(),
            };
            HttpResponse::Ok().json(body)
        }
        Err(e) => e.into_http_response(),
    }
}

/// ListSubscription
#[utoipa::path(
    context_path = "/api",
    tag = "Billings",
    operation_id = "ListSubscription",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",   content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/billings/list_subscription")]
pub async fn list_subscription(path: web::Path<String>, user_email: UserEmail) -> impl Responder {
    let org_id = path.into_inner();
    let email = user_email.user_id.as_str();
    match o2_cloud_billings::get_subscription(&org_id, email).await {
        Ok(cb) => MetaHttpResponse::json(ListSubscriptionResponseBody::from(cb)),
        Err(e) => e.into_http_response(),
    }
}

/// CreateBillingPortalSession
#[utoipa::path(
    context_path = "/api",
    tag = "Billings",
    operation_id = "CreateBillingPortalSession",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",   content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/billings/billing_portal")]
pub async fn create_billing_portal_session(
    path: web::Path<String>,
    req: HttpRequest,
) -> impl Responder {
    let org_id = path.into_inner();

    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let Some(customer_id) = query.get("customer_id") else {
        return o2_cloud_billings::BillingError::CustomerIdMissing.into_http_response();
    };

    let return_url = format!(
        "{}/web/billings/plans?update_org={}&org_identifier={}",
        get_config().common.web_url,
        chrono::Utc::now().timestamp_millis(),
        org_id
    );

    match o2_cloud_billings::create_customer_portal_session(&customer_id, &return_url).await {
        Err(err) => err.into_http_response(),
        Ok(billing_session) => MetaHttpResponse::json(billing_session),
    }
}

/// StripeWebhookEvent
#[utoipa::path(
    context_path = "/webhook",
    tag = "Billings",
    responses(
        (status = 200, description="Status OK", content_type = "application/json", body = HttpResponse)
    )
)]
#[post("/stripe")]
pub async fn handle_stripe_event(
    req: HttpRequest,
    payload: web::Bytes, // Raw body bytes
) -> impl Responder {
    // Convert payload bytes to string
    let payload_str = match String::from_utf8(payload.to_vec()) {
        Ok(str) => str,
        Err(_) => {
            return o2_cloud_billings::BillingError::InvalidStripePayload.into_http_response()
        }
    };

    // Get Stripe signature from headers
    let signature = match req.headers().get("Stripe-Signature") {
        Some(sig) => match sig.to_str() {
            Ok(s) => s,
            Err(_) => {
                return o2_cloud_billings::BillingError::InvalidStripeSig.into_http_response()
            }
        },
        None => return o2_cloud_billings::BillingError::StripeSigMissing.into_http_response(),
    };

    match o2_cloud_billings::handle_strip_wb_event(signature, &payload_str).await {
        Ok(()) => HttpResponse::Ok().json(json::json!({
            "status": "success"
        })),
        Err(err) => err.into_http_response(),
    }
}

// BillingsError extension
pub trait IntoHttpResponse {
    fn into_http_response(self) -> HttpResponse;
}

impl IntoHttpResponse for o2_cloud_billings::BillingError {
    fn into_http_response(self) -> HttpResponse {
        match self {
            o2_cloud_billings::BillingError::InfraError(err) => {
                MetaHttpResponse::internal_error(err)
            }
            o2_cloud_billings::BillingError::OrgNotFound => {
                MetaHttpResponse::not_found(self.to_string())
            }
            o2_cloud_billings::BillingError::SessionIdNotFound => {
                MetaHttpResponse::not_found(self.to_string())
            }
            o2_cloud_billings::BillingError::SubscriptionNotFound => {
                MetaHttpResponse::not_found(self.to_string())
            }
            _ => MetaHttpResponse::bad_request(self.to_string()),
        }
    }
}
