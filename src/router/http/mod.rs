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

use ::config::{
    get_config,
    meta::{
        cluster::{Role, RoleGroup},
        promql::RequestRangeQuery,
    },
    utils::rand::get_rand_element,
};
use actix_web::{http::Error, route, web, FromRequest, HttpRequest, HttpResponse};

use crate::common::{infra::cluster, utils::http::get_search_type_from_request};

mod ws;

const QUERIER_ROUTES: [&str; 20] = [
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
    "/prometheus/api/v1/query",
    "/prometheus/api/v1/query_range",
    "/prometheus/api/v1/query_exemplars",
    "/prometheus/api/v1/metadata",
    "/prometheus/api/v1/labels",
    "/prometheus/api/v1/label/",
];
const QUERIER_ROUTES_BY_BODY: [&str; 2] = [
    "/prometheus/api/v1/query_range",
    "/prometheus/api/v1/query_exemplars",
];
const FIXED_QUERIER_ROUTES: [&str; 3] = ["/summary", "/schema", "/streams"];

#[inline]
fn is_querier_route(path: &str) -> bool {
    QUERIER_ROUTES.iter().any(|x| path.contains(x))
}

#[inline]
fn is_querier_route_by_body(path: &str) -> bool {
    QUERIER_ROUTES_BY_BODY.iter().any(|x| path.contains(x))
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
    let cfg = get_config();

    // get online nodes
    let path = req
        .uri()
        .path_and_query()
        .map(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let new_url = get_url(&path).await;
    if new_url.is_error {
        return Ok(HttpResponse::ServiceUnavailable().body(new_url.value));
    }

    // check if the request is a websocket request
    let path_columns: Vec<&str> = path.split('/').collect();
    if *path_columns.get(3).unwrap_or(&"") == "ws" {
        return proxy_ws(req, payload, new_url, start).await;
    }

    // check if the request need to be proxied by body
    if cfg.common.metrics_cache_enabled && is_querier_route_by_body(&path) {
        return proxy_querier_by_body(req, payload, client, new_url, start, &path).await;
    }

    // send query
    default_proxy(req, payload, client, new_url, start).await
}

async fn get_url(path: &str) -> URLDetails {
    let node_type;
    let is_querier_path = is_querier_route(path);

    let nodes = if is_querier_path {
        node_type = Role::Querier;
        let query_str = path[path.find("?").unwrap_or(path.len())..].to_string();
        let node_group = web::Query::<HashMap<String, String>>::from_query(&query_str)
            .map(|query_params| {
                get_search_type_from_request(&query_params)
                    .unwrap_or(None)
                    .map(RoleGroup::from)
                    .unwrap_or(RoleGroup::Interactive)
            })
            .unwrap_or(RoleGroup::Interactive);
        let nodes = cluster::get_cached_online_querier_nodes(Some(node_group)).await;
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

async fn default_proxy(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
    new_url: URLDetails,
    start: std::time::Instant,
) -> actix_web::Result<HttpResponse, Error> {
    // send query
    let resp = client
        .request_from(new_url.value.clone(), req.head())
        .send_stream(payload)
        .await;
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

async fn proxy_querier_by_body(
    req: HttpRequest,
    payload: web::Payload,
    client: web::Data<awc::Client>,
    new_url: URLDetails,
    start: std::time::Instant,
    path: &str,
) -> actix_web::Result<HttpResponse, Error> {
    let (key, payload) = if path.contains("/prometheus/api/v1/query_range")
        || path.contains("/prometheus/api/v1/query_exemplars")
    {
        if req.method() == "GET" {
            let Ok(query) = web::Query::<RequestRangeQuery>::from_query(req.query_string()) else {
                return Ok(HttpResponse::BadRequest().body("Failed to parse query string"));
            };
            (query.query.clone().unwrap_or_default(), None)
        } else {
            let Ok(query) =
                web::Form::<RequestRangeQuery>::from_request(&req, &mut payload.into_inner()).await
            else {
                return Ok(HttpResponse::BadRequest().body("Failed to parse form data"));
            };
            (query.query.clone().unwrap_or_default(), Some(query))
        }
    } else {
        return default_proxy(req, payload, client, new_url, start).await;
    };

    // get node name by consistent hash
    let Some(node_name) = cluster::get_node_from_consistent_hash(&key, &Role::Querier, None).await
    else {
        return Ok(HttpResponse::ServiceUnavailable().body("No online querier nodes"));
    };

    // get node by name
    let Some(node) = cluster::get_cached_node_by_name(&node_name).await else {
        return Ok(HttpResponse::ServiceUnavailable().body("No online querier nodes"));
    };
    let new_url = format!("{}{}", node.http_addr, path);

    // send query
    let resp = if let Some(payload) = payload {
        client
            .request_from(new_url.clone(), req.head())
            .send_form(&payload)
            .await
    } else {
        client
            .request_from(new_url.clone(), req.head())
            .send()
            .await
    };
    if let Err(e) = resp {
        log::error!(
            "dispatch: {}, error: {}, took: {} ms",
            new_url,
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
            log::error!("{}: {}", new_url, e);
            return Ok(HttpResponse::ServiceUnavailable().body(e.to_string()));
        }
    };
    Ok(new_resp.body(body))
}

async fn proxy_ws(
    req: HttpRequest,
    payload: web::Payload,
    new_url: URLDetails,
    start: std::time::Instant,
) -> actix_web::Result<HttpResponse, Error> {
    let cfg = get_config();
    if cfg.common.websocket_enabled {
        // Convert the HTTP/HTTPS URL to a WebSocket URL (WS/WSS)
        let ws_url = match ws::convert_to_websocket_url(&new_url.value) {
            Ok(url) => url,
            Err(e) => {
                log::error!("Error converting URL to WebSocket: {}", e);
                return Ok(HttpResponse::BadRequest().body("Invalid WebSocket URL"));
            }
        };

        match ws::ws_proxy(req, payload, ws_url.clone()).await {
            Ok(res) => {
                log::info!(
                "[WS_ROUTER] Successfully proxied WebSocket connection to backend: {}, took: {} ms",
                ws_url,
                start.elapsed().as_millis()
            );
                Ok(res)
            }
            Err(e) => {
                log::error!("[WS_ROUTER] failed: {}", e);
                Ok(HttpResponse::InternalServerError().body("WebSocket proxy error"))
            }
        }
    } else {
        log::info!(
            "[WS_ROUTER]: Node Role: {} Websocket is disabled",
            cfg.common.node_role
        );
        Ok(HttpResponse::NotFound().body("WebSocket is disabled"))
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
    fn test_router_is_querier_route() {
        assert!(is_querier_route("/api/_search"));
        assert!(is_querier_route("/api/_around"));
        assert!(!is_querier_route("/api/_bulk"));
    }

    #[test]
    fn test_router_is_querier_route_by_body() {
        assert!(is_querier_route_by_body("/prometheus/api/v1/query_range"));
        assert!(is_querier_route_by_body(
            "/prometheus/api/v1/query_exemplars"
        ));
        assert!(!is_querier_route_by_body("/prometheus/api/v1/query"));
    }
}
