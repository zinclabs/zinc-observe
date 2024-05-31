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

use std::str::FromStr;

use actix_web::http;
use config::{get_config, SMTP_CLIENT};
use cron::Schedule;
use lettre::{message::SinglePart, AsyncTransport, Message};

use crate::{
    common::{
        meta::{
            authz::Authz,
            synthetics::{Synthetics, SyntheticsFrequencyType},
        },
        utils::auth::{remove_ownership, set_ownership},
    },
    service::db,
};

pub async fn save(
    org_id: &str,
    name: &str,
    mut synthetics: Synthetics,
    create: bool,
) -> Result<(), anyhow::Error> {
    let cfg = get_config();
    // Check if SMTP is enabled, otherwise don't save the synthetic
    if !cfg.smtp.smtp_enabled {
        return Err(anyhow::anyhow!("SMTP configuration not enabled"));
    }

    if !name.is_empty() {
        synthetics.name = name.to_string();
    }
    if synthetics.name.is_empty() || org_id.is_empty() {
        return Err(anyhow::anyhow!("Synthetics name/org_id is required"));
    }
    if synthetics.name.contains('/') {
        return Err(anyhow::anyhow!("Synthetics name cannot contain '/'"));
    }
    synthetics.org_id = org_id.to_string();

    if synthetics.schedule.frequency_type == SyntheticsFrequencyType::Cron {
        // Check if the cron expression is valid
        Schedule::from_str(&synthetics.schedule.cron)?;
    } else if synthetics.schedule.interval == 0 {
        synthetics.schedule.interval = 1;
    }

    match db::synthetics::get(org_id, &synthetics.name).await {
        Ok(_) => {
            if create {
                return Err(anyhow::anyhow!("Synthetics already exists"));
            }
        }
        Err(_) => {
            if !create {
                return Err(anyhow::anyhow!("Synthetics not found"));
            }
        }
    }

    match db::synthetics::set(org_id, &synthetics, create).await {
        Ok(_) => {
            if name.is_empty() {
                set_ownership(org_id, "synthetics", Authz::new(&synthetics.name)).await;
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn get(org_id: &str, name: &str) -> Result<Synthetics, anyhow::Error> {
    db::synthetics::get(org_id, name)
        .await
        .map_err(|_| anyhow::anyhow!("Synthetics not found"))
}

pub async fn list(
    org_id: &str,
    permitted: Option<Vec<String>>,
) -> Result<Vec<Synthetics>, anyhow::Error> {
    match db::synthetics::list(org_id).await {
        Ok(synthetics) => {
            let mut result = Vec::new();
            for synthetic in synthetics {
                if permitted.is_none()
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("synthetics:{}", synthetic.name))
                    || permitted
                        .as_ref()
                        .unwrap()
                        .contains(&format!("synthetics:_all_{}", org_id))
                {
                    result.push(synthetic);
                }
            }
            Ok(result)
        }
        Err(e) => Err(e),
    }
}

pub async fn delete(org_id: &str, name: &str) -> Result<(), (http::StatusCode, anyhow::Error)> {
    if db::synthetics::get(org_id, name).await.is_err() {
        return Err((
            http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("Synthetics not found {}", name),
        ));
    }

    match db::synthetics::delete(org_id, name).await {
        Ok(_) => {
            remove_ownership(org_id, "synthetics", Authz::new(name)).await;
            Ok(())
        }
        Err(e) => Err((http::StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn trigger(org_id: &str, name: &str) -> Result<(), (http::StatusCode, anyhow::Error)> {
    let synthetic = match db::synthetics::get(org_id, name).await {
        Ok(synthetic) => synthetic,
        _ => {
            return Err((
                http::StatusCode::NOT_FOUND,
                anyhow::anyhow!("Synthetics not found"),
            ));
        }
    };
    synthetic
        .test_target()
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

pub async fn enable(
    org_id: &str,
    name: &str,
    value: bool,
) -> Result<(), (http::StatusCode, anyhow::Error)> {
    let mut synthetic = match db::synthetics::get(org_id, name).await {
        Ok(synthetic) => synthetic,
        _ => {
            return Err((
                http::StatusCode::NOT_FOUND,
                anyhow::anyhow!("Synthetics not found"),
            ));
        }
    };
    synthetic.enabled = value;
    db::synthetics::set(org_id, &synthetic, false)
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

impl Synthetics {
    /// Sends the synthetic to subscribers
    pub async fn test_target(&self) -> Result<(), anyhow::Error> {
        log::info!("Testing target for synthetics");
        // self.send_email().await
        log::info!("Target tested");
        Ok(())
    }

    /// Sends emails to the [`Synthetics`] recepients. Currently only one pdf data is supported.
    async fn send_email(&self) -> Result<(), anyhow::Error> {
        let cfg = get_config();
        if !cfg.smtp.smtp_enabled {
            return Err(anyhow::anyhow!("SMTP configuration not enabled"));
        }

        let mut recepients = vec![];
        for email in &self.alert.emails {
            recepients.push(email);
        }

        let mut email = Message::builder()
            .from(cfg.smtp.smtp_from_email.parse()?)
            .subject(format!("Openobserve Synthetics - {}", &self.name));

        for recepient in recepients {
            email = email.to(recepient.parse()?);
        }

        if !cfg.smtp.smtp_reply_to.is_empty() {
            email = email.reply_to(cfg.smtp.smtp_reply_to.parse()?);
        }

        let email = email
            .singlepart(SinglePart::html(self.alert.message.clone()))
            .unwrap();

        // Send the email
        match SMTP_CLIENT.as_ref().unwrap().send(email).await {
            Ok(_) => {
                log::info!("email sent successfully for the synthetic {}", &self.name);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Error sending email: {e}")),
        }
    }
}
