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

type OrdF64 = ordered_float::OrderedFloat<f64>;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DashboardVersion {
    #[serde(default = "default_version")]
    pub version: i32,
}

fn default_version() -> i32 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema, Default)]
pub struct Dashboard {
    pub v1: Option<v1::Dashboard>,
    pub v2: Option<v2::Dashboard>,
    pub v3: Option<v3::Dashboard>,
    pub v4: Option<v4::Dashboard>,
    pub v5: Option<v5::Dashboard>,
    pub version: i32,
    pub hash: String,
}

impl Dashboard {
    pub fn dashboard_id(&self) -> Option<&str> {
        match self.version {
            1 => self.v1.as_ref().map(|inner| inner.dashboard_id.as_str()),
            2 => self.v2.as_ref().map(|inner| inner.dashboard_id.as_str()),
            3 => self.v3.as_ref().map(|inner| inner.dashboard_id.as_str()),
            4 => self.v4.as_ref().map(|inner| inner.dashboard_id.as_str()),
            5 => self.v5.as_ref().map(|inner| inner.dashboard_id.as_str()),
            _ => None,
        }
    }

    pub fn set_dashboard_id(&mut self, dashboard_id: String) {
        match self {
            Self {
                version: 1,
                v1: Some(inner),
                ..
            } => inner.dashboard_id = dashboard_id,
            Self {
                version: 2,
                v2: Some(inner),
                ..
            } => inner.dashboard_id = dashboard_id,
            Self {
                version: 3,
                v3: Some(inner),
                ..
            } => inner.dashboard_id = dashboard_id,
            Self {
                version: 4,
                v4: Some(inner),
                ..
            } => inner.dashboard_id = dashboard_id,
            Self {
                version: 5,
                v5: Some(inner),
                ..
            } => inner.dashboard_id = dashboard_id,
            _ => {}
        };
    }

    pub fn owner(&self) -> Option<&str> {
        match self.version {
            1 => self.v1.as_ref().map(|inner| inner.owner.as_str()),
            2 => self.v2.as_ref().map(|inner| inner.owner.as_str()),
            3 => self.v3.as_ref().map(|inner| inner.owner.as_str()),
            4 => self.v4.as_ref().map(|inner| inner.owner.as_str()),
            5 => self.v5.as_ref().map(|inner| inner.owner.as_str()),
            _ => None,
        }
    }

    pub fn title(&self) -> Option<&str> {
        match self.version {
            1 => self.v1.as_ref().map(|inner| inner.title.as_str()),
            2 => self.v2.as_ref().map(|inner| inner.title.as_str()),
            3 => self.v3.as_ref().map(|inner| inner.title.as_str()),
            4 => self.v4.as_ref().map(|inner| inner.title.as_str()),
            5 => self.v5.as_ref().map(|inner| inner.title.as_str()),
            _ => None,
        }
    }

    pub fn set_title(&mut self, title: String) {
        match self {
            Self {
                version: 1,
                v1: Some(inner),
                ..
            } => inner.title = title,
            Self {
                version: 2,
                v2: Some(inner),
                ..
            } => inner.title = title,
            Self {
                version: 3,
                v3: Some(inner),
                ..
            } => inner.title = title,
            Self {
                version: 4,
                v4: Some(inner),
                ..
            } => inner.title = title,
            Self {
                version: 5,
                v5: Some(inner),
                ..
            } => inner.title = title,
            _ => {}
        };
    }

    pub fn description(&self) -> Option<&str> {
        match self.version {
            1 => self.v1.as_ref().map(|inner| inner.description.as_str()),
            2 => self.v2.as_ref().map(|inner| inner.description.as_str()),
            3 => self.v3.as_ref().map(|inner| inner.description.as_str()),
            4 => self.v4.as_ref().map(|inner| inner.description.as_str()),
            5 => self.v5.as_ref().map(|inner| inner.description.as_str()),
            _ => None,
        }
    }

    pub fn role(&self) -> Option<&str> {
        match self.version {
            1 => self.v1.as_ref().map(|inner| inner.role.as_str()),
            2 => self.v2.as_ref().map(|inner| inner.role.as_str()),
            3 => self.v3.as_ref().map(|inner| inner.role.as_str()),
            4 => self.v4.as_ref().map(|inner| inner.role.as_str()),
            5 => self.v5.as_ref().map(|inner| inner.role.as_str()),
            _ => None,
        }
    }

    /// Returns the timestamp with timezone of the time at which the dashboard
    /// was created.
    ///
    /// This value is stored in JSON for versions 1-5 of the dashboard. However
    /// future versions of the dashboard should utilize the `created_at` Unix
    /// timestamp field in the database to represent the creation timestamp.
    pub fn created_at_deprecated(&self) -> Option<DateTime<FixedOffset>> {
        match self.version {
            1 => self.v1.as_ref().map(|inner| inner.created),
            2 => self.v2.as_ref().map(|inner| inner.created),
            3 => self.v3.as_ref().map(|inner| inner.created),
            4 => self.v4.as_ref().map(|inner| inner.created),
            5 => self.v5.as_ref().map(|inner| inner.created),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Dashboards {
    pub dashboards: Vec<Dashboard>,
}

pub mod reports;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoveDashboard {
    pub from: String,
    pub to: String,
}

pub fn datetime_now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).expect(
        "BUG", // This can't possibly fail. Can it?
    ))
}
