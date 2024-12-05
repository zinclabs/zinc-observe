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

use actix_web::http;
use config::meta::alerts::templates::Template;

use crate::{
    common::{
        infra::config::ALERTS_DESTINATIONS,
        meta::authz::Authz,
        utils::auth::{is_ofga_unsupported, remove_ownership, set_ownership},
    },
    service::db,
};

pub async fn save(
    org_id: &str,
    name: &str,
    mut template: Template,
    create: bool,
) -> Result<(), anyhow::Error> {
    if template.body.is_empty() {
        return Err(anyhow::anyhow!("Alert template body empty"));
    }
    if !name.is_empty() {
        template.name = name.to_owned();
    }
    template.name = template.name.trim().to_string();
    // Don't allow the characters not supported by ofga
    if is_ofga_unsupported(&template.name) {
        return Err(anyhow::anyhow!(
            "Alert template name cannot contain ':', '#', '?', '&', '%', quotes and space characters"
        ));
    }
    if template.name.is_empty() {
        return Err(anyhow::anyhow!("Alert template name is required"));
    }
    if template.name.contains('/') {
        return Err(anyhow::anyhow!("Alert template name cannot contain '/'"));
    }

    match db::alerts::templates::get(org_id, &template.name).await {
        Ok(_) => {
            if create {
                return Err(anyhow::anyhow!("Alert template already exists"));
            }
        }
        Err(_) => {
            if !create {
                return Err(anyhow::anyhow!("Alert template not found"));
            }
        }
    }

    let template_name = template.name.clone();
    match db::alerts::templates::set(org_id, template).await {
        Ok(_) => {
            if name.is_empty() {
                set_ownership(org_id, "templates", Authz::new(&template_name)).await;
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn get(org_id: &str, name: &str) -> Result<Template, anyhow::Error> {
    db::alerts::templates::get(org_id, name)
        .await
        .map_err(|_| anyhow::anyhow!("Alert template not found"))
}

pub async fn list(
    org_id: &str,
    permitted: Option<Vec<String>>,
) -> Result<Vec<Template>, anyhow::Error> {
    match db::alerts::templates::list(org_id).await {
        Ok(templates) => {
            let mut result = Vec::new();
            for template in templates {
                if permitted.is_none()
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("template:{}", template.name))
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("template:_all_{}", org_id))
                {
                    result.push(template);
                }
            }
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), (http::StatusCode, anyhow::Error)> {
    for dest in ALERTS_DESTINATIONS.iter() {
        if dest.key().starts_with(org_id) && dest.value().template.eq(&name) {
            return Err((
                http::StatusCode::CONFLICT,
                anyhow::anyhow!(
                    "Alert template is in use for destination {}",
                    &dest.value().name.clone()
                ),
            ));
        }
    }

    if db::alerts::templates::get(org_id, name).await.is_err() {
        return Err((
            http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("Alert template not found {}", name),
        ));
    }
    match db::alerts::templates::delete(org_id, name).await {
        Ok(_) => {
            remove_ownership(org_id, "templates", Authz::new(name)).await;
            Ok(())
        }
        Err(e) => Err((http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
