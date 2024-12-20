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

use super::{Alert, StreamType};

/// HTTP request body for `CreateAlert` endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct CreateAlertRequestBody(pub Alert);

/// HTTP request body for `UpdateAlert` endpoint.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdateAlertRequestBody(pub Alert);

/// HTTP URL query component that contains parameters for listing alerts.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
#[serde(rename_all = "snake_case")]
pub struct ListAlertsQuery {
    /// Optional folder ID filter parameter.
    pub folder: Option<String>,

    #[serde(flatten)]
    pub stream: Option<ListAlertsQueryStreamParams>,

    /// Optional owner user filter parameter.
    pub owner: Option<String>,

    /// Optional enabled filter parameter.
    pub enabled: Option<bool>,

    /// The optional number of alerts to retrieve. If not set then all alerts
    /// that match the query parameters will be returned.
    pub page_size: Option<u64>,

    /// The optional page index. If not set then defaults to `0`.
    pub page_idx: Option<u64>,
}

/// Parameters for filtering by stream in the HTTP URL query component for
/// listing alerts.
///
/// This structure is flattened inside [ListAlertsQuery] and is used to enforce
/// the constraint that `stream_name` can only be provided when `stream_type` is
/// also provided.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "snake_case")]
pub struct ListAlertsQueryStreamParams {
    /// Stream type filter parameter.
    pub stream_type: StreamType,

    /// Optional stream name filter parameter.
    pub stream_name: Option<String>,
}

impl From<CreateAlertRequestBody> for meta_alerts::Alert {
    fn from(value: CreateAlertRequestBody) -> Self {
        value.0.into()
    }
}

impl From<UpdateAlertRequestBody> for meta_alerts::Alert {
    fn from(value: UpdateAlertRequestBody) -> Self {
        value.0.into()
    }
}
