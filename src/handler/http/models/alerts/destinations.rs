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

use config::meta::alerts::destinations as meta_destinations;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Destination {
    #[serde(default)]
    pub name: String,
    /// Required for `Http` destination_type
    #[serde(default)]
    pub url: String,
    /// Required for `Http` destination_type
    #[serde(default)]
    pub method: HTTPType,
    #[serde(default)]
    pub skip_tls_verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    pub template: String,
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
pub enum DestinationType {
    #[default]
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "sns")]
    Sns,
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

impl From<meta_destinations::Destination> for Destination {
    fn from(value: meta_destinations::Destination) -> Self {
        Self {
            name: value.name,
            url: value.url,
            method: value.method.into(),
            skip_tls_verify: value.skip_tls_verify,
            headers: value.headers,
            template: value.template,
            emails: value.emails,
            sns_topic_arn: value.sns_topic_arn,
            aws_region: value.aws_region,
            destination_type: value.destination_type.into(),
        }
    }
}

impl From<meta_destinations::DestinationType> for DestinationType {
    fn from(value: meta_destinations::DestinationType) -> Self {
        match value {
            meta_destinations::DestinationType::Http => Self::Http,
            meta_destinations::DestinationType::Email => Self::Email,
            meta_destinations::DestinationType::Sns => Self::Sns,
        }
    }
}

impl From<meta_destinations::HTTPType> for HTTPType {
    fn from(value: meta_destinations::HTTPType) -> Self {
        match value {
            meta_destinations::HTTPType::POST => Self::POST,
            meta_destinations::HTTPType::PUT => Self::PUT,
            meta_destinations::HTTPType::GET => Self::GET,
        }
    }
}

impl From<Destination> for meta_destinations::Destination {
    fn from(value: Destination) -> Self {
        Self {
            name: value.name,
            url: value.url,
            method: value.method.into(),
            skip_tls_verify: value.skip_tls_verify,
            headers: value.headers,
            template: value.template,
            emails: value.emails,
            sns_topic_arn: value.sns_topic_arn,
            aws_region: value.aws_region,
            destination_type: value.destination_type.into(),
        }
    }
}

impl From<DestinationType> for meta_destinations::DestinationType {
    fn from(value: DestinationType) -> Self {
        match value {
            DestinationType::Http => Self::Http,
            DestinationType::Email => Self::Email,
            DestinationType::Sns => Self::Sns,
        }
    }
}

impl From<HTTPType> for meta_destinations::HTTPType {
    fn from(value: HTTPType) -> Self {
        match value {
            HTTPType::POST => Self::POST,
            HTTPType::PUT => Self::PUT,
            HTTPType::GET => Self::GET,
        }
    }
}
