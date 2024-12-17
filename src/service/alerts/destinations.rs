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

use config::meta::destinations::{Destination, DestinationType, Module, Template};

use crate::{
    common::{
        infra::config::STREAM_ALERTS,
        meta::authz::Authz,
        utils::auth::{is_ofga_unsupported, remove_ownership, set_ownership},
    },
    service::db::{self, alerts::destinations::DestinationError, user},
};

pub async fn save(
    org_id: &str,
    name: &str,
    mut destination: Destination,
    create: bool,
) -> Result<(), DestinationError> {
    // First validate the `destination` according to its `destination_type`
    match &mut destination.module {
        Module::Alert {
            destination_type, ..
        } => match destination_type {
            DestinationType::Email(email) => {
                if email.recipients.is_empty() {
                    return Err(DestinationError::EmptyEmail);
                }
                if !config::get_config().smtp.smtp_enabled {
                    return Err(DestinationError::SMTPUnavailable);
                }
                let mut lowercase_emails = vec![];
                for email in email.recipients.iter() {
                    let email = email.trim().to_lowercase();
                    // Check if the email is part of the org
                    let res = user::get(Some(org_id), &email).await;
                    if res.is_err() || res.is_ok_and(|usr| usr.is_none()) {
                        return Err(DestinationError::UserNotPermitted);
                    }
                    lowercase_emails.push(email);
                }
                email.recipients = lowercase_emails;
            }
            DestinationType::Http(endpoint) => {
                if endpoint.url.is_empty() {
                    return Err(DestinationError::EmptyUrl);
                }
            }
            DestinationType::Sns(aws_sns) => {
                if aws_sns.sns_topic_arn.is_empty() || aws_sns.aws_region.is_empty() {
                    return Err(DestinationError::InvalidSns);
                }
            }
        },
        Module::Pipeline { endpoint, .. } => {
            if endpoint.url.is_empty() {
                return Err(DestinationError::EmptyUrl);
            }
        }
    }

    if !name.is_empty() {
        destination.name = name.to_string();
    }
    destination.name = destination.name.trim().to_string();
    if destination.name.is_empty() {
        return Err(DestinationError::EmptyName);
    }
    if destination.name.contains('/') || is_ofga_unsupported(&destination.name) {
        return Err(DestinationError::InvalidName);
    }

    match db::alerts::destinations::get(org_id, &destination.name).await {
        Ok(_) => {
            if create {
                return Err(DestinationError::AlreadyExists);
            }
        }
        Err(_) => {
            if !create {
                return Err(DestinationError::NotFound);
            }
        }
    }

    let saved = db::alerts::destinations::set(org_id, destination).await?;
    if name.is_empty() {
        set_ownership(org_id, "destinations", Authz::new(&saved.name)).await;
    }
    Ok(())
}

pub async fn get(org_id: &str, name: &str) -> Result<Destination, DestinationError> {
    db::alerts::destinations::get(org_id, name).await
}

// pub async fn get_with_template(
//     org_id: &str,
//     name: &str,
// ) -> Result<DestinationWithTemplate, anyhow::Error> {
//     let dest = get(org_id, name).await?;
//     let template = db::alerts::templates::get(org_id, &dest.template).await?;
//     Ok(dest.with_template(template))
// }

pub async fn list(
    org_id: &str,
    permitted: Option<Vec<String>>,
) -> Result<Vec<Destination>, DestinationError> {
    Ok(db::alerts::destinations::list(org_id)
        .await?
        .into_iter()
        .filter(|dest| {
            permitted.is_none()
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("destination:{}", dest.name))
                || permitted
                    .as_ref()
                    .unwrap()
                    .contains(&format!("destination:_all_{}", org_id))
        })
        .collect())
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), DestinationError> {
    let cacher = STREAM_ALERTS.read().await;
    for (stream_key, alerts) in cacher.iter() {
        for alert in alerts.iter() {
            if stream_key.starts_with(org_id) && alert.destinations.contains(&name.to_string()) {
                return Err(DestinationError::InUse(alert.name.to_string()));
            }
        }
    }
    drop(cacher);

    db::alerts::destinations::delete(org_id, name).await?;
    remove_ownership(org_id, "destinations", Authz::new(name)).await;
    Ok(())
}
