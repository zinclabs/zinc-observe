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

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::datetime_now;

#[derive(Serialize, Debug, Deserialize, Clone, ToSchema)]
pub enum ReportDestination {
    #[serde(rename = "email")]
    Email(String), // Supports email only
}

#[derive(Serialize, Debug, Default, Deserialize, Clone, ToSchema)]
pub enum ReportMediaType {
    #[default]
    #[serde(rename = "pdf")]
    Pdf, // Supports Pdf only
}

#[derive(Serialize, Debug, Default, Deserialize, Clone, ToSchema)]
pub struct ReportDashboardVariable {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Serialize, Debug, Deserialize, Clone, ToSchema)]
pub struct ReportDashboard {
    pub dashboard: String,
    pub folder: String,
    pub tabs: Vec<String>,
    #[serde(default)]
    pub variables: Vec<ReportDashboardVariable>,
    /// The timerange of dashboard data.
    #[serde(default)]
    pub timerange: ReportTimerange,
}

#[derive(Serialize, Debug, Default, Deserialize, Clone, ToSchema)]
pub enum ReportTimerangeType {
    #[default]
    #[serde(rename = "relative")]
    Relative,
    #[serde(rename = "absolute")]
    Absolute,
}

#[derive(Serialize, Debug, Deserialize, Clone, ToSchema)]
pub struct ReportTimerange {
    #[serde(rename = "type")]
    pub range_type: ReportTimerangeType,
    pub period: String, // 15m, 4M etc. For relative.
    pub from: i64,      // For absolute, in microseconds
    pub to: i64,        // For absolute, in microseconds
}

impl Default for ReportTimerange {
    fn default() -> Self {
        Self {
            range_type: ReportTimerangeType::default(),
            period: "1w".to_string(),
            from: 0,
            to: 0,
        }
    }
}

#[derive(Serialize, Debug, Default, Deserialize, PartialEq, Clone, ToSchema)]
pub enum ReportFrequencyType {
    #[serde(rename = "once")]
    Once,
    #[serde(rename = "hours")]
    Hours,
    #[serde(rename = "days")]
    Days,
    #[serde(rename = "weeks")]
    #[default]
    Weeks,
    #[serde(rename = "months")]
    Months,
    #[serde(rename = "cron")]
    Cron,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ReportFrequency {
    /// Frequency interval in the `frequency_type` unit
    #[serde(default)]
    pub interval: i64,
    /// Cron expression
    #[serde(default)]
    pub cron: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub frequency_type: ReportFrequencyType,
}

impl Default for ReportFrequency {
    fn default() -> Self {
        Self {
            interval: 1,
            cron: "".to_string(),
            frequency_type: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub title: String,
    pub org_id: String,
    /// Frequency of report generation. E.g. - Weekly.
    #[serde(default)]
    pub frequency: ReportFrequency,
    /// Start time of report generation in UNIX microseconds.
    #[serde(default)]
    pub start: i64,
    pub dashboards: Vec<ReportDashboard>,
    pub destinations: Vec<ReportDestination>,
    #[serde(default)]
    pub description: String,
    /// Message to include in the email
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub media_type: ReportMediaType,
    /// User email for chromedriver login
    #[serde(default)]
    pub user: String,
    /// User password for chromedriver login
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub timezone: String,
    /// Fixed timezone offset in minutes
    #[serde(default)]
    #[serde(rename = "timezoneOffset")]
    pub tz_offset: i32,
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

impl Default for Report {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            title: "".to_string(),
            org_id: "".to_string(),
            frequency: ReportFrequency::default(),
            start: Utc::now().timestamp_micros(), // Now
            destinations: vec![],
            dashboards: vec![],
            description: "".to_string(),
            message: "".to_string(),
            enabled: false,
            media_type: ReportMediaType::default(),
            user: "".to_string(),
            password: "".to_string(),
            timezone: "".to_string(),
            tz_offset: 0, // UTC
            last_triggered_at: None,
            created_at: datetime_now(),
            updated_at: None,
            owner: "".to_string(),
            last_edited_by: "".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub struct ReportEmailDetails {
    #[serde(alias = "recepients")]
    pub recipients: Vec<String>,
    pub title: String,
    pub name: String,
    pub message: String,
    pub dashb_url: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct HttpReportPayload {
    pub dashboards: Vec<ReportDashboard>,
    pub email_details: ReportEmailDetails,
}

#[derive(Debug, Clone)]
pub struct ReportListFilters {
    pub dashboard: Option<String>,
    pub folder: Option<String>,
    pub destination_less: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipients_backwards_compatibility() {
        // The serde alias allows loading data created before the typo was fixed.
        let email_details = ReportEmailDetails {
            recipients: vec!["foo@example.com".to_string()],
            title: "title".to_string(),
            name: "name".to_string(),
            message: "message".to_string(),
            dashb_url: "https://example.com/dashb_url".to_string(),
        };
        let json = serde_json::to_string(&email_details).unwrap();
        let json_using_alias = json.replace("recipients", "recepients");
        let email_details_from_alias: ReportEmailDetails =
            serde_json::from_str(&json_using_alias).unwrap();
        assert_eq!(email_details, email_details_from_alias);
    }
}
