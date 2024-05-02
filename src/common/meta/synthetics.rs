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

use chrono::{DateTime, FixedOffset};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{
    alerts::{destinations::HTTPType, Operator},
    dashboards::datetime_now,
};

#[derive(Serialize, Debug, Default, Deserialize, PartialEq, Clone, ToSchema)]
#[serde(rename = "snake_case")]
pub enum SyntheticsAlertType {
    #[default]
    Email,
}

#[derive(Serialize, Debug, Default, Deserialize, PartialEq, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticsFrequencyType {
    Once,
    Seconds,
    Minutes,
    #[default]
    Hours,
    Days,
    Cron,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SyntheticsFrequency {
    /// Frequency interval in the `frequency_type` unit
    #[serde(default)]
    pub interval: i64,
    /// Cron expression
    #[serde(default)]
    pub cron: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub frequency_type: SyntheticsFrequencyType,
    /// Start time of report generation in UNIX microseconds.
    pub start: i64,
    pub timezone: String,
    #[serde(default)]
    #[serde(rename = "timezoneOffset")]
    pub timezone_offset: i64,
}

impl Default for SyntheticsFrequency {
    fn default() -> Self {
        Self {
            interval: 1,
            cron: "".to_string(),
            frequency_type: Default::default(),
            start: 0,
            timezone: "".to_string(),
            timezone_offset: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyntheticsType {
    #[default]
    Http,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HttpSyntheticsBodyType {
    #[default]
    Raw,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HttpSyntheticsAuthType {
    #[default]
    Basic,
    Bearer,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpSyntheticsAuth {
    #[serde(rename = "type")]
    pub auth_type: HttpSyntheticsAuthType,
    pub basic: String,
    pub bearer: String,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
pub struct HttpSynthetics {
    pub url: String,
    pub method: HTTPType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_params: Option<HashMap<String, String>>,
    pub body_type: HttpSyntheticsBodyType,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    pub cookies: String,
    pub auth: HttpSyntheticsAuth,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
pub struct SyntheticsRetry {
    pub count: u32,
    pub delay: u32,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
pub struct SyntheticsAlert {
    #[serde(rename = "type")]
    pub alert_type: SyntheticsAlertType,
    pub emails: Vec<String>,
    pub message: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Synthetics {
    pub name: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub synthetics_type: SyntheticsType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<HttpSynthetics>,
    #[serde(default)]
    pub retry: SyntheticsRetry,
    /// Frequency of synthetics test. E.g. - Hourly.
    #[serde(default)]
    pub schedule: SyntheticsFrequency,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub alert: SyntheticsAlert,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_triggered_at: Option<i64>,
    #[serde(default = "datetime_now")]
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTime<FixedOffset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: Option<DateTime<FixedOffset>>,
    pub owner: String,
    pub last_edited_by: String,
}

impl Default for Synthetics {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            org_id: "".to_string(),
            description: "".to_string(),
            synthetics_type: SyntheticsType::Http,
            request: Some(Default::default()),
            retry: Default::default(),
            schedule: SyntheticsFrequency::default(),
            assertions: Default::default(),
            alert: SyntheticsAlert::default(),
            enabled: false,
            last_triggered_at: None,
            created_at: datetime_now(),
            updated_at: None,
            owner: "".to_string(),
            last_edited_by: "".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    #[default]
    Body,
    Headers,
    ResponseTime,
    StatusCode,
    BodyHash,
}

#[derive(Clone, Debug, Serialize, Default, Deserialize, ToSchema)]
pub struct Assertion {
    #[serde(rename = "type")]
    pub assert_type: AssertionType,
    pub operator: Operator,
    pub key: String,
    pub timing_scope: bool,
    pub value: config::utils::json::Value,
}
