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

use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use config::meta::actions::action::ActionType;
use o2_enterprise::enterprise::actions::action_deployer::ACTION_DEPLOYER;

#[post("/{org_id}/v1/job")]
pub async fn create_job(path: web::Path<String>, body: web::Bytes) -> impl Responder {
    let org_id = path.into_inner();

    if let Some(deployer) = ACTION_DEPLOYER.get() {
        return match deployer.create_app(&org_id, body).await {
            Ok(created_at) => HttpResponse::Ok().body(created_at.to_rfc3339()),
            Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
        };
    }

    HttpResponse::InternalServerError().json("AppDeployer not initialized")
}

#[delete("/{org_id}/v1/job/{name}")]
pub async fn delete_job(path: web::Path<(String, String)>) -> impl Responder {
    let (org_id, name) = path.into_inner();

    if let Some(deployer) = ACTION_DEPLOYER.get() {
        return match deployer.delete_app(&org_id, &name).await {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
        };
    }
    HttpResponse::InternalServerError().json("AppDeployer not initialized")
}

#[get("/{org_id}/v1/job/{name}")]
pub async fn get_app_details(path: web::Path<(String, String)>) -> impl Responder {
    let (org_id, name) = path.into_inner();

    if let Some(deployer) = ACTION_DEPLOYER.get() {
        return match deployer.get_app_status(&org_id, &name).await {
            Ok(resp) => HttpResponse::Ok().json(resp),
            Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
        };
    }
    HttpResponse::InternalServerError().json("AppDeployer not initialized")
}

#[get("/{org_id}/v1/job")]
pub async fn list_deployed_apps(path: web::Path<String>) -> impl Responder {
    let org_id = path.into_inner();

    if let Some(deployer) = ACTION_DEPLOYER.get() {
        return match deployer.list_apps(&org_id).await {
            Ok(resp) => HttpResponse::Ok().json(resp),
            Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
        };
    }

    HttpResponse::InternalServerError().json("AppDeployer not initialized")
}

// Patch a resource
#[put("/{org_id}/v1/job/{id}")]
pub async fn patch_action(
    path: web::Path<(String, String)>,
    query: web::Query<HashMap<String, String>>,
    body: web::Bytes,
) -> impl Responder {
    let (org_id, id) = path.into_inner();

    // Extract the "action_type" from query parameters and handle missing cases properly
    let action_type: ActionType = match query.get("action_type") {
        Some(value) => match value.clone().as_str().try_into() {
            Ok(action_type) => action_type,
            Err(e) => return HttpResponse::BadRequest().json(e.to_string()),
        },
        None => return HttpResponse::BadRequest().body("Missing required 'action_type' parameter"),
    };

    if let Some(deployer) = ACTION_DEPLOYER.get() {
        return match deployer
            .update_action(&org_id, &id, action_type, body)
            .await
        {
            Ok(modified_at) => HttpResponse::Ok().body(modified_at.to_rfc3339()),
            Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
        };
    }

    HttpResponse::InternalServerError().json("AppDeployer not initialized")
}
