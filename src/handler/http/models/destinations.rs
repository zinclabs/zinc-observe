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

//! These models define the schemas of HTTP request and response JSON bodies in
//! folders API endpoints.

use std::fmt;

use config::{ider, meta::destinations as meta_dest};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::service::db::alerts::destinations::DestinationError;

impl From<meta_dest::Destination> for Destination {
    fn from(value: meta_dest::Destination) -> Self {
        match value.module {
            meta_dest::Module::Alert {
                template,
                destination_type,
            } => match destination_type {
                meta_dest::DestinationType::Email(email) => Self {
                    name: value.name,
                    emails: email.recipients,
                    template: Some(template),
                    destination_type: DestinationType::Email,
                    ..Default::default()
                },
                meta_dest::DestinationType::Http(endpoint) => Self {
                    name: value.name,
                    url: endpoint.url,
                    method: endpoint.method,
                    skip_tls_verify: endpoint.skip_tls_verify,
                    headers: endpoint.headers,
                    destination_type: DestinationType::Http,
                    template: Some(template),
                    ..Default::default()
                },
                meta_dest::DestinationType::Sns(aws_sns) => Self {
                    name: value.name,
                    template: Some(template),
                    sns_topic_arn: Some(aws_sns.sns_topic_arn),
                    aws_region: Some(aws_sns.aws_region),
                    destination_type: DestinationType::Sns,
                    ..Default::default()
                },
            },
            meta_dest::Module::Pipeline {
                pipeline_id,
                endpoint,
            } => Self {
                name: value.name,
                url: endpoint.url,
                method: endpoint.method,
                skip_tls_verify: endpoint.skip_tls_verify,
                headers: endpoint.headers,
                pipeline_id: Some(pipeline_id),
                destination_type: DestinationType::Http,
                ..Default::default()
            },
        }
    }
}

impl Destination {
    pub fn into(self, org_id: &str) -> Result<meta_dest::Destination, DestinationError> {
        let id = String::new(); // placeholder, set at db layer
        if let Some(template) = self.template {
            let destination_type = match self.destination_type {
                DestinationType::Email => meta_dest::DestinationType::Email(meta_dest::Email {
                    recipients: self.emails,
                }),
                DestinationType::Http => meta_dest::DestinationType::Http(meta_dest::Endpoint {
                    url: self.url,
                    method: meta_dest::HTTPType::from(self.method),
                    skip_tls_verify: self.skip_tls_verify,
                    headers: self.headers,
                }),
                DestinationType::Sns => meta_dest::DestinationType::Sns(meta_dest::AwsSns {
                    sns_topic_arn: self.sns_topic_arn.ok_or(DestinationError::InvalidSns)?,
                    aws_region: self.aws_region.ok_or(DestinationError::InvalidSns)?,
                }),
            };
            Ok(meta_dest::Destination {
                id,
                org_id: org_id.to_string(),
                name: self.name,
                module: meta_dest::Module::Alert {
                    template,
                    destination_type,
                },
            })
        } else if let Some(pipeline_id) = self.pipeline_id {
            let endpoint = meta_dest::Endpoint {
                url: self.url,
                method: self.method,
                skip_tls_verify: self.skip_tls_verify,
                headers: self.headers,
            };
            Ok(meta_dest::Destination {
                id,
                org_id: org_id.to_string(),
                name: self.name,
                module: meta_dest::Module::Pipeline {
                    pipeline_id,
                    endpoint,
                },
            })
        } else {
            Err(DestinationError::UnsupportedType)
        }
    }
}

impl From<meta_dest::Template> for Template {
    fn from(value: meta_dest::Template) -> Self {
        let (title, template_type) = match value.template_type {
            meta_dest::TemplateType::Email { title } => (title, DestinationType::Email),
            meta_dest::TemplateType::Http => (String::new(), DestinationType::Http),
            meta_dest::TemplateType::Sns => (String::new(), DestinationType::Sns),
        };

        Self {
            name: value.name,
            body: value.body,
            is_default: value.is_default.then_some(true),
            template_type,
            title,
        }
    }
}

impl Template {
    pub fn into(self, org_id: &str) -> meta_dest::Template {
        let template_type = match self.template_type {
            DestinationType::Email => meta_dest::TemplateType::Email { title: self.title },
            DestinationType::Sns => meta_dest::TemplateType::Sns,
            DestinationType::Http => meta_dest::TemplateType::Http,
        };
        meta_dest::Template {
            id: ider::uuid(),
            org_id: org_id.to_string(),
            name: self.name,
            is_default: self.is_default.unwrap_or_default(),
            template_type,
            body: self.body,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct Destination {
    #[serde(default)]
    pub name: String,
    /// Required for `Http` destination_type
    #[serde(default)]
    pub url: String,
    /// Required for `Http` destination_type
    #[serde(default)]
    pub method: meta_dest::HTTPType,
    #[serde(default)]
    pub skip_tls_verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_id: Option<String>,
    /// Required when `destination_type` is `Email`
    #[serde(default)]
    pub emails: Vec<String>,
    // New SNS-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sns_topic_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub destination_type: DestinationType,
}

#[derive(Serialize, Debug, Default, PartialEq, Eq, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DestinationType {
    #[default]
    Http,
    Email,
    Sns,
}

impl From<&str> for DestinationType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "email" => DestinationType::Email,
            "http" => DestinationType::Http,
            _ => DestinationType::Sns,
        }
    }
}

impl fmt::Display for DestinationType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DestinationType::Email => write!(f, "email"),
            DestinationType::Http => write!(f, "http"),
            DestinationType::Sns => write!(f, "sns"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Template {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub body: String,
    #[serde(rename = "isDefault")]
    #[serde(default)]
    pub is_default: Option<bool>,
    /// Indicates whether the body is
    /// http or email body
    #[serde(rename = "type")]
    #[serde(default)]
    pub template_type: DestinationType,
    #[serde(default)]
    pub title: String,
}
