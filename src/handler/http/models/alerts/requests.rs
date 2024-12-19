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

use config::meta::alerts::alert as meta_alerts;
use serde::Deserialize;
use utoipa::ToSchema;

use super::Alert;

/// HTTP request body for `SaveAlert` endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct SaveAlertRequestBody(pub Alert);

/// HTTP request body for `UpdateAlert` endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateAlertRequestBody(pub Alert);

impl From<SaveAlertRequestBody> for meta_alerts::Alert {
    fn from(value: SaveAlertRequestBody) -> Self {
        value.0.into()
    }
}

impl From<UpdateAlertRequestBody> for meta_alerts::Alert {
    fn from(value: UpdateAlertRequestBody) -> Self {
        value.0.into()
    }
}
