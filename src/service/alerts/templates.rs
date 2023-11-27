// Copyright 2023 Zinc Labs Inc.
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

use crate::common::infra::config::ALERTS_DESTINATIONS;
use crate::common::meta::alerts::DestinationTemplate;
use crate::service::db;

#[tracing::instrument(skip_all)]
pub async fn save_template(
    org_id: &str,
    name: &str,
    mut template: DestinationTemplate,
) -> Result<(), anyhow::Error> {
    if template.body.is_null() {
        return Err(anyhow::anyhow!("Alert template body empty"));
    }
    template.name = Some(name.to_string());
    db::alerts::templates::set(org_id, name, template.clone()).await
}

#[tracing::instrument]
pub async fn list_templates(org_id: &str) -> Result<Vec<DestinationTemplate>, anyhow::Error> {
    db::alerts::templates::list(org_id).await
}

#[tracing::instrument]
pub async fn delete_template(
    org_id: &str,
    name: &str,
) -> Result<(), (http::StatusCode, anyhow::Error)> {
    for dest in ALERTS_DESTINATIONS.iter() {
        if dest.key().starts_with(org_id) && dest.value().template.eq(&name) {
            return Err((
                http::StatusCode::FORBIDDEN,
                anyhow::anyhow!(
                    "Alert template is in use for destination {}",
                    &dest.value().clone().name.unwrap()
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
    db::alerts::templates::delete(org_id, name)
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[tracing::instrument]
pub async fn get_template(org_id: &str, name: &str) -> Result<DestinationTemplate, anyhow::Error> {
    db::alerts::templates::get(org_id, name)
        .await
        .map_err(|_| anyhow::anyhow!("Alert template not found"))
}
