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

use std::io;

use actix_web::{http::StatusCode, HttpResponse};
use config::ider;
use ipnetwork::IpNetwork;

use crate::{
    common::{
        infra::config::SYSLOG_ROUTES,
        meta::{
            http::HttpResponse as MetaHttpResponse,
            syslog::{SyslogRoute, SyslogRoutes, SyslogServer},
        },
    },
    job,
    service::db::syslog,
};

#[tracing::instrument(skip_all)]
pub async fn create_route(mut route: SyslogRoute) -> Result<HttpResponse, io::Error> {
    if route.org_id.trim().is_empty()
        || route.stream_name.trim().is_empty()
        || route.subnets.is_empty()
    {
        return Ok(Response::BadRequest(
            "Please provide stream name/org_id/subnets for route".to_owned(),
        )
        .into());
    }
    for (_, existing_route) in SYSLOG_ROUTES.clone() {
        let existing_subnets = &existing_route.subnets;
        let new_subnets = &route.subnets;

        if existing_subnets.iter().any(|existing_subnet| {
            new_subnets
                .iter()
                .any(|subnet| subnets_overlap(existing_subnet, subnet))
        }) {
            return Ok(Response::BadRequest(format!(
                "Provided subnet/s overlap with existing subnet/s for organization {}",
                &existing_route.org_id
            ))
            .into());
        }
    }

    route.id = ider::generate();
    if let Err(e) = syslog::set(&route).await {
        return Ok(Response::InternalServerError(e).into());
    }
    tracing::info!(id = route.id, "Syslog Route created");
    Ok(HttpResponse::Created().json(route))
}

#[tracing::instrument(skip_all)]
pub async fn update_route(id: &str, route: &mut SyslogRoute) -> Result<HttpResponse, io::Error> {
    if route.org_id.trim().is_empty()
        && route.stream_name.trim().is_empty()
        && route.subnets.is_empty()
    {
        return Ok(Response::BadRequest(
            "Please provide stream name/org_id/subnets for route to update".to_owned(),
        )
        .into());
    }

    if route.id.is_empty() {
        route.id = id.to_owned();
    }
    let old_route = match syslog::get(id).await {
        Ok(route) => route,
        Err(error) => {
            tracing::info!(%error, id, "Syslog Route not found");
            return Ok(Response::NotFound.into());
        }
    };
    if route.org_id.is_empty() {
        route.org_id = old_route.org_id.clone();
    }
    if route.stream_name.is_empty() {
        route.stream_name = old_route.stream_name.clone();
    }
    if route.subnets.is_empty() {
        route.subnets = old_route.subnets.clone();
    }

    if route == &old_route {
        return Ok(HttpResponse::Ok().json(route));
    }

    if let Err(error) = syslog::set(route).await {
        tracing::error!(%error, id, "Failed to save the syslog route");
        return Ok(Response::InternalServerError(error).into());
    }
    Ok(HttpResponse::Ok().json(route))
}

#[tracing::instrument]
pub async fn list_routes() -> Result<HttpResponse, io::Error> {
    Ok(HttpResponse::Ok().json(SyslogRoutes {
        routes: syslog::list().await.unwrap(),
    }))
}

#[tracing::instrument]
pub async fn get_route(id: &str) -> Result<HttpResponse, io::Error> {
    let resp = if let Ok(route) = syslog::get(id).await {
        HttpResponse::Ok().json(route)
    } else {
        Response::NotFound.into()
    };
    Ok(resp)
}

#[tracing::instrument]
pub async fn delete_route(id: &str) -> Result<HttpResponse, io::Error> {
    let resp = if syslog::delete(id).await.is_err() {
        Response::NotFound
    } else {
        Response::OkMessage("Syslog route deleted".to_owned())
    };
    Ok(resp.into())
}

#[tracing::instrument(skip_all)]
pub async fn toggle_state(server: SyslogServer) -> Result<HttpResponse, io::Error> {
    if job::syslog_server::run(server.state, false).await.is_err() {
        return Ok(Response::InternalServerError(anyhow::anyhow!(
            "Failed to toggle syslog server state"
        ))
        .into());
    }
    Ok(HttpResponse::Created().json(server.state))
}

#[derive(Debug)]
enum Response {
    OkMessage(String),
    NotFound,
    InternalServerError(anyhow::Error),
    BadRequest(String),
}

impl From<Response> for HttpResponse {
    fn from(resp: Response) -> Self {
        match resp {
            Response::OkMessage(message) => {
                Self::Ok().json(MetaHttpResponse::message(StatusCode::OK.into(), message))
            }
            Response::NotFound => Self::NotFound().json(MetaHttpResponse::error(
                StatusCode::NOT_FOUND.into(),
                "Syslog route not found".to_owned(),
            )),
            Response::InternalServerError(err) => Self::InternalServerError().json(
                MetaHttpResponse::error(StatusCode::INTERNAL_SERVER_ERROR.into(), err.to_string()),
            ),
            Response::BadRequest(err) => Self::BadRequest()
                .json(MetaHttpResponse::error(StatusCode::BAD_REQUEST.into(), err)),
        }
    }
}

fn subnets_overlap(net1: &IpNetwork, net2: &IpNetwork) -> bool {
    net1.contains(net2.network())
        || net1.contains(net2.broadcast())
        || net2.contains(net1.network())
        || net2.contains(net1.broadcast())
}
