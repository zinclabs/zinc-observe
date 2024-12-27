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

use chrono::{DateTime, FixedOffset};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use svix_ksuid::Ksuid;
use utoipa::ToSchema;

use crate::meta::{
    alerts::{QueryCondition, TriggerCondition},
    stream::StreamType,
};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Alert {
    #[serde(default)]
    pub id: Option<Ksuid>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub stream_type: StreamType,
    #[serde(default)]
    pub stream_name: String,
    #[serde(default)]
    pub is_real_time: bool,
    #[serde(default)]
    pub query_condition: QueryCondition,
    #[serde(default)]
    pub trigger_condition: TriggerCondition,
    pub destinations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_attributes: Option<HashMap<String, String>>,
    #[serde(default)]
    pub row_template: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    /// Timezone offset in minutes.
    /// The negative secs means the Western Hemisphere
    pub tz_offset: i32,
    #[serde(default)]
    pub last_triggered_at: Option<i64>,
    #[serde(default)]
    pub last_satisfied_at: Option<i64>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: Option<DateTime<FixedOffset>>,
    #[serde(default)]
    pub last_edited_by: Option<String>,
}

impl PartialEq for Alert {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.stream_type == other.stream_type
            && self.stream_name == other.stream_name
    }
}

impl Default for Alert {
    fn default() -> Self {
        Self {
            id: None,
            name: "".to_string(),
            org_id: "".to_string(),
            stream_type: StreamType::default(),
            stream_name: "".to_string(),
            is_real_time: false,
            query_condition: QueryCondition::default(),
            trigger_condition: TriggerCondition::default(),
            destinations: vec![],
            context_attributes: None,
            row_template: "".to_string(),
            description: "".to_string(),
            enabled: false,
            tz_offset: 0, // UTC
            last_triggered_at: None,
            owner: None,
            updated_at: None,
            last_edited_by: None,
            last_satisfied_at: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AlertListFilter {
    pub enabled: Option<bool>,
    pub owner: Option<String>,
}

/// Parameters for listing alerts.
#[derive(Debug, Clone)]
pub struct ListAlertsParams {
    /// The optional org ID surrogate key with which to filter alerts.
    pub org_id: Option<String>,

    /// The optional folder ID surrogate key with which to filter alerts.
    pub folder_id: Option<String>,

    /// The optional stream type and name with which to filter alerts.
    pub stream_type_and_name: Option<(StreamType, String)>,

    /// The optional filter on the enabled field. `Some(true)` indicates that
    /// only enabled alerts should be returned, while `Some(false)` indicates
    /// that only disabled alerts should be returned.
    pub enabled: Option<bool>,

    /// The optional owner with which to filter alerts.
    pub owner: Option<String>,

    /// The optional page size and page index of results to retrieve.
    pub page_size_and_idx: Option<(u64, u64)>,
}

impl ListAlertsParams {
    /// Returns new parameters to list dashboards for the given org ID surrogate
    /// key.
    pub fn new(org_id: &str) -> Self {
        Self {
            org_id: Some(org_id.to_owned()),
            folder_id: None,
            stream_type_and_name: None,
            enabled: None,
            owner: None,
            page_size_and_idx: None,
        }
    }

    /// Returns new parameters to list all dashboards.
    pub fn all() -> Self {
        Self {
            org_id: None,
            folder_id: None,
            stream_type_and_name: None,
            enabled: None,
            owner: None,
            page_size_and_idx: None,
        }
    }

    /// Filter dashboards by the given folder ID surrogate key.
    pub fn in_folder(mut self, folder_id: &str) -> Self {
        self.folder_id = Some(folder_id.to_string());
        self
    }

    /// Filter alerts by the given stream type and name.
    pub fn for_stream(mut self, stream_type: StreamType, stream_name: &str) -> Self {
        self.stream_type_and_name = Some((stream_type, stream_name.to_string()));
        self
    }

    /// Paginate the results by the given page size and page index.
    pub fn paginate(mut self, page_size: u64, page_idx: u64) -> Self {
        self.page_size_and_idx = Some((page_size, page_idx));
        self
    }
}
