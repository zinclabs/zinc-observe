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

use std::str::FromStr;

use serde::Serialize;
use svix_ksuid::Ksuid;
use utoipa::ToSchema;

/// HTTP response body for `ListReports` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListReportsResponseBody {
    pub list: Vec<ListReportsResponseBodyItem>,
}

/// An item in the list returned by the `ListReports` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListReportsResponseBodyItem {
    pub report_id: Ksuid,
    pub folder_id: String,
    pub folder_name: String,
    pub name: String,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub last_triggered_at: Option<i64>,
}

impl TryFrom<Vec<infra::table::reports::ListReportsQueryResult>> for ListReportsResponseBody {
    type Error = svix_ksuid::Error;

    fn try_from(
        value: Vec<infra::table::reports::ListReportsQueryResult>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            list: value
                .into_iter()
                .map(ListReportsResponseBodyItem::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<infra::table::reports::ListReportsQueryResult> for ListReportsResponseBodyItem {
    type Error = svix_ksuid::Error;

    fn try_from(value: infra::table::reports::ListReportsQueryResult) -> Result<Self, Self::Error> {
        Ok(Self {
            report_id: Ksuid::from_str(&value.report_id)?,
            folder_id: value.folder_id,
            folder_name: value.folder_name,
            name: value.report_name,
            owner: value.report_owner,
            description: value.report_description,
            last_triggered_at: value.report_last_triggered_at,
        })
    }
}
