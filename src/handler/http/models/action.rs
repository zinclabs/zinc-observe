// Copyright (c) 2024. OpenObserve Inc.
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

use std::collections::HashMap;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use config::meta::actions::action::{Action, ActionStatus, ExecutionDetailsType};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetActionInfoResponse {
    pub id: String,
    pub name: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub last_run_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_option_datetime")]
    pub last_successful_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub status: ActionStatus,
    pub execution_details_type: ExecutionDetailsType,
}

impl TryFrom<Action> for GetActionInfoResponse {
    type Error = anyhow::Error;

    fn try_from(value: Action) -> Result<Self, Self::Error> {
        Ok(GetActionInfoResponse {
            id: value.id.ok_or(anyhow!("No Id for action"))?.to_string(),
            name: value.name,
            created_at: value.created_at,
            last_run_at: value.last_executed_at.unwrap_or(value.created_at),
            last_successful_at: value.last_successful_at,
            created_by: value.created_by,
            status: value.status,
            execution_details_type: value.execution_details,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetActionListResponse {
    pub actions: Vec<GetActionInfoResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetActionDetailsResponse {
    pub id: String,
    pub name: String,
    pub environment_variables: HashMap<String, String>,
    pub execution_details: ExecutionDetailsType,
    pub cron_expr: Option<String>,
    pub zip_file_name: String,
    pub description: Option<String>,
    pub service_account: String,
}

impl TryFrom<Action> for GetActionDetailsResponse {
    type Error = anyhow::Error;

    fn try_from(value: Action) -> Result<Self, Self::Error> {
        Ok(GetActionDetailsResponse {
            id: value.id.ok_or(anyhow!("No Id for action"))?.to_string(),
            name: value.name,
            environment_variables: value.environment_variables,
            execution_details: value.execution_details,
            cron_expr: value.cron_expr,
            zip_file_name: value.zip_file_name,
            description: value.description,
            service_account: value.service_account,
        })
    }
}

// Function to serialize DateTime<Utc> to Unix microseconds
fn serialize_datetime<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i64(dt.timestamp_micros())
}

// Function to serialize Option<DateTime<Utc>> to Unix microseconds
fn serialize_option_datetime<S>(
    dt: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => serializer.serialize_some(&dt.timestamp_micros()),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestActionRequest {
    pub inputs: Vec<serde_json::Value>,
}
