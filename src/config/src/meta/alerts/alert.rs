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

use crate::meta::{
    alerts::{QueryCondition, TriggerCondition},
    stream::StreamType,
};

#[derive(Clone, Debug)]
pub struct Alert {
    pub name: String,
    pub org_id: String,
    pub stream_type: StreamType,
    pub stream_name: String,
    pub is_real_time: bool,
    pub query_condition: QueryCondition,
    pub trigger_condition: TriggerCondition,
    pub destinations: Vec<String>,
    pub context_attributes: Option<HashMap<String, String>>,
    pub row_template: String,
    pub description: String,
    pub enabled: bool,
    /// Timezone offset in minutes.
    /// The negative secs means the Western Hemisphere
    pub tz_offset: i32,
    pub last_triggered_at: Option<i64>,
    pub last_satisfied_at: Option<i64>,
    pub owner: Option<String>,
    pub updated_at: Option<DateTime<FixedOffset>>,
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
