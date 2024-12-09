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

pub mod reports;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

pub fn datetime_now() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(0).expect(
        "BUG", // This can't possibly fail. Can it?
    ))
}

/// Parameters for listing dashboards.
#[derive(Debug, Clone)]
pub struct ListDashboardsParams {
    /// The org ID surrogate key with which to filter dashboards.
    pub org_id: String,

    /// The optional folder ID surrogate key with which to filter dashboards.
    pub folder_id: Option<String>,

    /// The optional case-insensitive title substring with which to filter
    /// dashboards.
    pub title_pat: Option<String>,

    /// The optional page size and page index of results to retrieve.
    pub page_size_and_idx: Option<(u64, u64)>,
}

impl ListDashboardsParams {
    /// Returns new parameters to list dashboards for the given org ID surrogate
    /// key.
    pub fn new(org_id: &str) -> Self {
        Self {
            org_id: org_id.to_string(),
            folder_id: None,
            title_pat: None,
            page_size_and_idx: None,
        }
    }

    /// Filter dashboards by the given folder ID surrogate key.
    pub fn with_folder_id(mut self, folder_id: &str) -> Self {
        self.folder_id = Some(folder_id.to_string());
        self
    }

    /// Filter dashboards by the case-insensitive title pattern.
    ///
    /// Listed dashboards will only include dashboards with a title that
    /// contains the case-insitive title pattern.
    pub fn where_title_contains(mut self, title_pat: &str) -> Self {
        self.title_pat = Some(title_pat.to_string());
        self
    }

    /// Paginate the results by the given page size and page index.
    pub fn paginate(mut self, page_size: u64, page_idx: u64) -> Self {
        self.page_size_and_idx = Some((page_size, page_idx));
        self
    }
}
