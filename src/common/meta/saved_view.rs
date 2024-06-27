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

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateViewRequest {
    /// Base64 encoded string, containing all the data for a given view.
    /// This data is expected to be versioned so that the frontend can
    /// deserialize as required.
    pub data: serde_json::Value,

    /// User-readable name of the view, doesn't need to be unique.
    pub view_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateViewRequest {
    /// Base64 encoded string, containing all the data for a given view.
    /// This data is expected to be versioned so that the frontend can
    /// deserialize as required.
    pub data: serde_json::Value,

    /// User-readable name of the view, doesn't need to be unique.
    pub view_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct View {
    pub org_id: String,
    pub data: serde_json::Value,
    pub view_id: String,
    pub view_name: String,
}

/// Save the bandwidth for a given view, without sending the actual data
/// This is expected to be used for listing views.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ViewWithoutData {
    pub org_id: String,
    pub view_id: String,
    pub view_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ViewsWithoutData {
    pub views: Vec<ViewWithoutData>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DeleteViewResponse {
    pub org_id: String,
    pub view_id: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateViewResponse {
    pub org_id: String,
    pub view_id: String,
    pub view_name: String,
}
