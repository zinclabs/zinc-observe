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

mod ws_proxy;

use ::config::{
    get_config,
    meta::cluster::{Role, RoleGroup},
    utils::rand::get_rand_element,
};
use actix_web::{http::Error, route, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use tokio::sync::mpsc;

use crate::{common::infra::cluster, router::http::ws_proxy::CustomWebSocketHandlers};

const QUERIER_ROUTES: [&str; 19] = [
    "/config",
    "/summary",
    "/organizations",
    "/settings",
    "/schema",
    "/streams",
    "/clusters",
    "/query_manager",
    "/ws",
    "/_search",
    "/_around",
    "/_values",
    "/functions?page_num=",
    "/prometheus/api/v1/series",
    "/prometheus/api/v1/query_range",
    "/prometheus/api/v1/query",
    "/prometheus/api/v1/metadata",
    "/prometheus/api/v1/labels",
    "/prometheus/api/v1/label/",
];

const FIXED_QUERIER_ROUTES: [&str; 3] = ["/summary", "/schema", "/streams"];

#[inline]
fn check_querier_route(path: &str) -> bool {
    QUERIER_ROUTES.iter().any(|x| path.contains(x))
}

#[inline]
fn is_fixed_querier_route(path: &str) -> bool {
    FIXED_QUERIER_ROUTES.iter().any(|x| path.contains(x))
}

#[route(
    "/config",
    method = "GET",
    method = "POST",
    method = "PUT",
    method = "DELETE"
)]
pub async fn config(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    dispatch(req, payload, client).await
}

#[route(
    "/config/{path:.*}",
    method = "GET",
    method = "POST",
    method = "PUT",
    method = "DELETE"
)]
pub async fn config_paths(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    dispatch(req, payload, client).await
}

#[route(
    "/api/{path:.*}",
    method = "GET",
    method = "POST",
    method = "PUT",
    method = "DELETE"
)]
pub async fn api(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    dispatch(req, payload, client).await
}

#[route(
    "/aws/{path:.*}",
    method = "GET",
    method = "POST",
    method = "PUT",
    method = "DELETE"
)]
pub async fn aws(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    dispatch(req, payload, client).await
}

#[route(
    "/gcp/{path:.*}",
    method = "GET",
    method = "POST",
    method = "PUT",
    method = "DELETE"
)]
pub async fn gcp(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    dispatch(req, payload, client).await
}

#[route(
    "/rum/{path:.*}",
    // method = "GET",
    method = "POST",
)]
pub async fn rum(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    dispatch(req, payload, client).await
}

async fn dispatch(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
) -> actix_web::Result<HttpResponse, Error> {
    let start = std::time::Instant::now();

    // get online nodes
    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("");
    let new_url = get_url(path).await;
    if new_url.is_error {
        return Ok(HttpResponse::ServiceUnavailable().body(new_url.value));
    }

    dbg!(&path);

    // check if the request is a websocket request
    if path.starts_with("/ws") {
        log::info!(
            "Websocket request received on dispatcher: {}",
            new_url.value.to_string()
        );

        // Upgrade to a websocket connection
        let ws_res = ws::start(
            CustomWebSocketHandlers {
                tx: mpsc::unbounded_channel().0,
                url: new_url.value.clone(),
            },
            &req,
            payload,
        );
        let res = match ws_res {
            Ok(res) => Ok(res),
            Err(e) => {
                log::error!("WebSocket upgrade failed: {}", e);
                Ok(HttpResponse::ServiceUnavailable().body(e.to_string()))
            }
        };
        return res;
    }

    // send query
    let cfg = get_config();
    let resp = if cfg.route.connection_pool_disabled {
        let client = awc::Client::builder()
            .timeout(std::time::Duration::from_secs(cfg.route.timeout))
            .disable_redirects()
            .finish();
        client
            .request_from(new_url.value.clone(), req.head())
            .insert_header((awc::http::header::CONNECTION, "close"))
            .send_stream(payload)
            .await
    } else {
        client
            .request_from(new_url.value.clone(), req.head())
            .send_stream(payload)
            .await
    };
    if let Err(e) = resp {
        log::error!(
            "dispatch: {}, error: {}, took: {} ms",
            new_url.value,
            e,
            start.elapsed().as_millis()
        );
        return Ok(HttpResponse::ServiceUnavailable().body(e.to_string()));
    }

    // handle response
    let mut resp = resp.unwrap();
    let mut new_resp = HttpResponse::build(resp.status());

    // copy headers
    for (key, value) in resp.headers() {
        if !key.eq("content-encoding") {
            new_resp.insert_header((key.clone(), value.clone()));
        }
    }

    // set body
    let body = match resp
        .body()
        .limit(get_config().limit.req_payload_limit)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            log::error!("{}: {}", new_url.value, e);
            return Ok(HttpResponse::ServiceUnavailable().body(e.to_string()));
        }
    };
    Ok(new_resp.body(body))
}

async fn get_url(path: &str) -> URLDetails {
    let node_type;
    let is_querier_path = check_querier_route(path);

    let nodes = if is_querier_path {
        node_type = Role::Querier;
        let nodes = cluster::get_cached_online_querier_nodes(Some(RoleGroup::Interactive)).await;
        if is_fixed_querier_route(path) && nodes.is_some() && !nodes.as_ref().unwrap().is_empty() {
            nodes.map(|v| v.into_iter().take(1).collect())
        } else {
            nodes
        }
    } else {
        node_type = Role::Ingester;
        cluster::get_cached_online_ingester_nodes().await
    };
    if nodes.is_none() || nodes.as_ref().unwrap().is_empty() {
        let cfg = get_config();
        if node_type == Role::Ingester && !cfg.route.ingester_srv_url.is_empty() {
            return URLDetails {
                is_error: false,
                value: format!(
                    "http://{}:{}{}",
                    cfg.route.ingester_srv_url, cfg.http.port, path
                ),
            };
        }
        return URLDetails {
            is_error: true,
            value: format!("No online {node_type} nodes"),
        };
    }

    let nodes = nodes.unwrap();
    let node = get_rand_element(&nodes);
    URLDetails {
        is_error: false,
        value: format!("{}{}", node.http_addr, path),
    }
}

struct URLDetails {
    is_error: bool,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_querier_route() {
        assert!(check_querier_route("/api/_search"));
        assert!(check_querier_route("/api/_around"));
        assert!(!check_querier_route("/api/_bulk"));
    }
}
