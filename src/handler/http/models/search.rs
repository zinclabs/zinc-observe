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

use config::meta::search as meta_search;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Hash, Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchEventType {
    UI,
    Dashboards,
    Reports,
    Alerts,
    Values,
    Other,
    RUM,
    DerivedStream,
}

impl From<meta_search::SearchEventType> for SearchEventType {
    fn from(value: meta_search::SearchEventType) -> Self {
        match value {
            meta_search::SearchEventType::UI => Self::UI,
            meta_search::SearchEventType::Dashboards => Self::Dashboards,
            meta_search::SearchEventType::Reports => Self::Reports,
            meta_search::SearchEventType::Alerts => Self::Alerts,
            meta_search::SearchEventType::Values => Self::Values,
            meta_search::SearchEventType::Other => Self::Other,
            meta_search::SearchEventType::RUM => Self::RUM,
            meta_search::SearchEventType::DerivedStream => Self::DerivedStream,
        }
    }
}

impl From<SearchEventType> for meta_search::SearchEventType {
    fn from(value: SearchEventType) -> Self {
        match value {
            SearchEventType::UI => Self::UI,
            SearchEventType::Dashboards => Self::Dashboards,
            SearchEventType::Reports => Self::Reports,
            SearchEventType::Alerts => Self::Alerts,
            SearchEventType::Values => Self::Values,
            SearchEventType::Other => Self::Other,
            SearchEventType::RUM => Self::RUM,
            SearchEventType::DerivedStream => Self::DerivedStream,
        }
    }
}
