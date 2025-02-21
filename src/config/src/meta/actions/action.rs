// Copyright (c) 2024.

use std::{collections::HashMap, fmt::Display};

use chrono::{DateTime, Utc};
use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use svix_ksuid::Ksuid;
use utoipa::ToSchema;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionDetailsType {
    #[default]
    Once,
    Repeat,
    Service,
}

impl std::fmt::Display for ExecutionDetailsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ExecutionDetailsType::Once => "once",
            ExecutionDetailsType::Repeat => "repeat",
            ExecutionDetailsType::Service => "service",
        };
        write!(f, "{}", s)
    }
}

impl TryFrom<&str> for ExecutionDetailsType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "once" => Ok(ExecutionDetailsType::Once),
            "repeat" => Ok(ExecutionDetailsType::Repeat),
            "service" => Ok(ExecutionDetailsType::Service),
            _ => Err(anyhow::anyhow!("Invalid ExecutionDetailsType")),
        }
    }
}

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum,
)]
#[sea_orm(db_type = "Enum", rs_type = "String", enum_name = "status")]
pub enum ActionStatus {
    #[default]
    #[sea_orm(string_value = "ready")]
    Ready,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "errored")]
    Errored,
    #[sea_orm(string_value = "completed")]
    Completed,
}

impl TryFrom<&str> for ActionStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ready" => Ok(ActionStatus::Ready),
            "running" => Ok(ActionStatus::Running),
            "errored" => Ok(ActionStatus::Errored),
            "completed" => Ok(ActionStatus::Completed),
            _ => Err(anyhow::anyhow!("Invalid ActionStatus")),
        }
    }
}

impl Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ActionStatus::Ready => "ready",
            ActionStatus::Running => "running",
            ActionStatus::Errored => "errored",
            ActionStatus::Completed => "completed",
        };
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Action {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Ksuid>,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub zip_file_path: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub execution_details: ExecutionDetailsType,
    #[serde(default)]
    pub cron_expr: Option<String>,
    pub environment_variables: HashMap<String, String>,
    #[serde(default)]
    pub created_by: String,
    pub status: ActionStatus,
    pub created_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
    // User Set variable
    pub zip_file_name: String,
    // default to created_at
    #[serde(default)]
    pub last_modified_at: DateTime<Utc>,
    #[serde(default)]
    pub last_successful_at: Option<DateTime<Utc>>,
    pub origin_cluster_url: String,
    pub service_account: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateActionDetailsRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub environment_variables: Option<HashMap<String, String>>,
    #[serde(default)]
    pub execution_details: Option<ExecutionDetailsType>,
    #[serde(default)]
    pub cron_expr: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub service_account: Option<String>,
}

/// Request send to Action Deployer, to deploy an action
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateActionRequest {
    #[serde(default)]
    pub job_name: Option<String>,
    #[serde(default)]
    pub label_name: Option<String>,
    #[serde(default)]
    pub cron_expr: Option<String>,
    #[serde(default)]
    pub environment_variables: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionType {
    #[default]
    Job,
    CronJob,
    Service,
}

impl Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ActionType::Job => "job".to_string(),
            ActionType::CronJob => "cronjob".to_string(),
            ActionType::Service => "service".to_string(),
        };
        write!(f, "{}", str)
    }
}
impl TryFrom<&str> for ActionType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "job" => Ok(ActionType::Job),
            "cronjob" => Ok(ActionType::CronJob),
            "service" => Ok(ActionType::Service),
            _ => Err(anyhow::anyhow!("Invalid action type: {}", s)),
        }
    }
}

impl From<ExecutionDetailsType> for ActionType {
    fn from(e: ExecutionDetailsType) -> Self {
        match e {
            ExecutionDetailsType::Once => ActionType::Job,
            ExecutionDetailsType::Repeat => ActionType::CronJob,
            ExecutionDetailsType::Service => ActionType::Service,
        }
    }
}

/// Response from Action Deployer, to get the status of an action
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionRunningStatusResponse {
    #[serde(default)]
    pub id: String,
    pub action_status: ActionStatus,
    pub created_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
    pub last_successful_at: Option<DateTime<Utc>>,
}
