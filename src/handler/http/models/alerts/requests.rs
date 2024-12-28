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
pub struct CreateAlertRequestBody {
    /// Optional folder ID indicating the folder in which to create the alert.
    /// If omitted the alert will be created in the default folder.
    pub folder_id: Option<String>,

    #[serde(flatten)]
    pub alert: Alert,
}

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

    /// The optional page size and page index of results to retrieve.
    #[serde(flatten)]
    pub page: Option<ListAlertsQueryPageParams>,
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

/// Parameters for requesting a specific page of results in the HTTP URL query
/// component for listing alerts.
///
/// This structure is flattened inside [ListAlertsQuery] and is used to enforce
/// the constraint that `page_idx` can only be provided when `page_size` is
/// also provided.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "snake_case")]
pub struct ListAlertsQueryPageParams {
    /// The optional number of alerts to retrieve. If not set then all alerts
    /// that match the query parameters will be returned.
    pub page_size: u64,

    /// The optional page index. If not set then defaults to `0`.
    pub page_idx: Option<u64>,
}

/// HTTP URL query component that contains parameters for enabling alerts.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
#[serde(rename_all = "snake_case")]
pub struct EnableAlertQuery {
    /// Set to `true` to enable the alert or `false` to disable the alert.
    pub value: bool,
}

impl From<CreateAlertRequestBody> for meta_alerts::Alert {
    fn from(value: CreateAlertRequestBody) -> Self {
        value.alert.into()
    }
}

impl From<UpdateAlertRequestBody> for meta_alerts::Alert {
    fn from(value: UpdateAlertRequestBody) -> Self {
        value.0.into()
    }
}

impl ListAlertsQuery {
    pub fn into(self, org_id: &str) -> meta_alerts::ListAlertsParams {
        meta_alerts::ListAlertsParams {
            org_id: org_id.to_string(),
            folder_id: self.folder,
            stream_type_and_name: self.stream.map(
                |ListAlertsQueryStreamParams {
                     stream_type,
                     stream_name,
                 }| { (stream_type.into(), stream_name) },
            ),
            enabled: self.enabled,
            owner: self.owner,
            page_size_and_idx: self.page.map(
                |ListAlertsQueryPageParams {
                     page_size,
                     page_idx,
                 }| { (page_size, page_idx.unwrap_or(0)) },
            ),
        }
    }
}
