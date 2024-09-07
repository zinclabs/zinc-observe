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

use super::destinations::DestinationType;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Template {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub body: String,
    #[serde(rename = "isDefault")]
    #[serde(default)]
    pub is_default: Option<bool>,
    /// Indicates whether the body is
    /// http or email body
    #[serde(rename = "type")]
    #[serde(default)]
    pub template_type: DestinationType,
}
