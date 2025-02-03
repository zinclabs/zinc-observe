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

use std::fmt;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::templates::Template;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Destination {
    #[serde(rename = "type")]
    #[serde(default)]
    pub destination_type: DestinationType,
    #[serde(default)]
    pub name: String,
    /// Required for `Http` or `RemotePipeline` destination_type
    #[serde(default)]
    pub url: String,
    /// Required for `Http` or `RemotePipeline` destination_type
    #[serde(default)]
    pub method: HTTPType,
    /// Required for `RemotePipeline` destination_type
    #[serde(default)]
    pub skip_tls_verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub template: String,
    /// Required when `destination_type` is `Email`
    #[serde(default)]
    pub emails: Vec<String>,
    // New SNS-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sns_topic_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
}

#[derive(Serialize, Debug, Default, PartialEq, Eq, Deserialize, Clone, ToSchema)]
pub enum DestinationType {
    #[default]
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "sns")]
    Sns,
    #[serde(rename = "remote_pipeline")]
    RemotePipeline,
}

impl fmt::Display for DestinationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DestinationType::Http => "http",
            DestinationType::Email => "email",
            DestinationType::Sns => "sns",
            DestinationType::RemotePipeline => "remote_pipeline",
        };
        write!(f, "{}", s)
    }
}

impl From<&str> for DestinationType {
    fn from(s: &str) -> Self {
        match s {
            "http" => DestinationType::Http,
            "email" => DestinationType::Email,
            "sns" => DestinationType::Sns,
            "remote_pipeline" => DestinationType::RemotePipeline,
            _ => DestinationType::Http,
        }
    }
}

impl Destination {
    pub fn with_template(&self, template: Template) -> DestinationWithTemplate {
        DestinationWithTemplate {
            name: self.name.clone(),
            url: self.url.clone(),
            method: self.method.clone(),
            skip_tls_verify: self.skip_tls_verify,
            headers: self.headers.clone(),
            template,
            emails: self.emails.clone(),
            destination_type: self.destination_type.clone(),
            sns_topic_arn: self.sns_topic_arn.clone(),
            aws_region: self.aws_region.clone(),
        }
    }

    pub fn is_remote_pipeline(&self) -> bool {
        self.destination_type == DestinationType::RemotePipeline
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct DestinationWithTemplate {
    pub name: String,
    pub url: String,
    pub method: HTTPType,
    #[serde(default)]
    pub skip_tls_verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    pub template: Template,
    pub emails: Vec<String>,
    pub destination_type: DestinationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sns_topic_arn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub enum HTTPType {
    #[default]
    #[serde(rename = "post")]
    POST,
    #[serde(rename = "put")]
    PUT,
    #[serde(rename = "get")]
    GET,
}

impl fmt::Display for HTTPType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HTTPType::POST => write!(f, "post"),
            HTTPType::PUT => write!(f, "put"),
            HTTPType::GET => write!(f, "get"),
        }
    }
}
