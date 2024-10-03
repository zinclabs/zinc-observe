// Copyright 2024 Zinc Labs Inc.
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

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::meta::{
    alerts::{QueryCondition, TriggerCondition},
    stream::StreamParams,
};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct DerivedStreamMeta {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub source: StreamParams,
    #[serde(default)]
    pub destination: StreamParams,
    #[serde(default)]
    pub is_real_time: bool,
    #[serde(default)]
    pub query_condition: QueryCondition,
    #[serde(default)]
    pub trigger_condition: TriggerCondition, // Frequency type only supports minutes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_attributes: Option<HashMap<String, String>>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    /// Timezone offset in minutes.
    /// The negative secs means the Western Hemisphere
    pub tz_offset: i32,
}

impl Default for DerivedStreamMeta {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            source: StreamParams::default(),
            destination: StreamParams::default(),
            is_real_time: false,
            query_condition: QueryCondition::default(),
            trigger_condition: TriggerCondition::default(),
            context_attributes: None,
            description: "".to_string(),
            enabled: true,
            tz_offset: 0, // UTC
        }
    }
}
