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

use async_trait::async_trait;
use config::meta::meta_store::MetaStore;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::errors::Result;

pub mod mysql;
pub mod postgres;
pub mod sqlite;

static CLIENT: Lazy<Box<dyn Scheduler>> = Lazy::new(connect);
pub const TRIGGERS_KEY: &str = "/triggers/";

pub fn connect() -> Box<dyn Scheduler> {
    match config::get_config().common.meta_store.as_str().into() {
        MetaStore::MySQL => Box::<mysql::MySqlScheduler>::default(),
        MetaStore::PostgreSQL => Box::<postgres::PostgresScheduler>::default(),
        _ => Box::<sqlite::SqliteScheduler>::default(),
    }
}

#[async_trait]
pub trait Scheduler: Sync + Send + 'static {
    async fn create_table(&self) -> Result<()>;
    async fn create_table_index(&self) -> Result<()>;
    async fn push(&self, trigger: Trigger) -> Result<()>;
    async fn delete(&self, org: &str, module: TriggerModule, key: &str) -> Result<()>;
    async fn update_status(
        &self,
        org: &str,
        module: TriggerModule,
        key: &str,
        status: TriggerStatus,
        retries: i32,
    ) -> Result<()>;
    async fn update_trigger(&self, trigger: Trigger) -> Result<()>;
    async fn pull(
        &self,
        concurrency: i64,
        alert_timeout: i64,
        report_timeout: i64,
    ) -> Result<Vec<Trigger>>;
    async fn get(&self, org: &str, module: TriggerModule, key: &str) -> Result<Trigger>;
    async fn list(&self, module: Option<TriggerModule>) -> Result<Vec<Trigger>>;
    async fn clean_complete(&self) -> Result<()>;
    async fn watch_timeout(&self) -> Result<()>;
    async fn len_module(&self, module: TriggerModule) -> usize;
    async fn len(&self) -> usize;
    async fn is_empty(&self) -> bool;
    async fn clear(&self) -> Result<()>;
}

#[derive(Debug, Clone, sqlx::Type, PartialEq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum TriggerStatus {
    #[default]
    Waiting,
    Processing,
    Completed,
}

#[derive(Debug, Clone, sqlx::Type, PartialEq, Serialize, Deserialize, Default)]
#[repr(i32)]
pub enum TriggerModule {
    Report,
    #[default]
    Alert,
    Synthetics,
}

impl std::fmt::Display for TriggerModule {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TriggerModule::Alert => write!(f, "alert"),
            TriggerModule::Report => write!(f, "report"),
            TriggerModule::Synthetics => write!(f, "synthetics"),
        }
    }
}

#[derive(sqlx::FromRow, Debug, Clone, Default)]
pub struct TriggerId {
    pub id: i64,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize, Deserialize, Default)]
pub struct Trigger {
    pub org: String,
    pub module: TriggerModule,
    pub module_key: String,
    pub next_run_at: i64,
    pub is_realtime: bool,
    pub is_silenced: bool,
    pub status: TriggerStatus,
    // #[sqlx(default)] only works when the column itself is missing.
    // For NULL value it does not work.
    // TODO: See https://github.com/launchbadge/sqlx/issues/1106
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    pub retries: i32,
}

/// Initializes the scheduler - creates table and index
pub async fn init() -> Result<()> {
    CLIENT.create_table().await?;
    CLIENT.create_table_index().await?;
    Ok(())
}

/// Pushes a Trigger job into the queue
#[inline]
pub async fn push(trigger: Trigger) -> Result<()> {
    CLIENT.push(trigger).await
}

/// Deletes the Trigger job matching the given parameters
#[inline]
pub async fn delete(org: &str, module: TriggerModule, key: &str) -> Result<()> {
    CLIENT.delete(org, module, key).await
}

/// Updates the status of the Trigger job. This method is supposed
/// to be used only by the node that is currently processing the trigger.
#[inline]
pub async fn update_status(
    org: &str,
    module: TriggerModule,
    key: &str,
    status: TriggerStatus,
    retries: i32,
) -> Result<()> {
    CLIENT
        .update_status(org, module, key, status, retries)
        .await
}

/// Updates the triggers with given identifiers.
/// Does not update start_time, end_time, org, module and key. Must
/// only be used by the node that is currently processing the trigger.
/// Use `pull()` method to set the status of the job from `Waiting` to `Processing`.
#[inline]
pub async fn update_trigger(trigger: Trigger) -> Result<()> {
    CLIENT.update_trigger(trigger).await
}

/// Scheduler pulls only those triggers that match the conditions-
/// - trigger.next_run_at <= now
/// - !(trigger.is_realtime && !trigger.is_silenced)
/// - trigger.status == "Waiting"
///
/// `concurrency` - Defines the maximum number of jobs to pull at a time.
/// `timeout` - Used to set the maximum time duration the job executation can take.
///     This is used to calculate the `end_time` of the trigger.
#[inline]
pub async fn pull(
    concurrency: i64,
    alert_timeout: i64,
    report_timeout: i64,
) -> Result<Vec<Trigger>> {
    CLIENT
        .pull(concurrency, alert_timeout, report_timeout)
        .await
}

/// Returns the scheduled job associated with the given id in read-only fashion
#[inline]
pub async fn get(org: &str, module: TriggerModule, key: &str) -> Result<Trigger> {
    CLIENT.get(org, module, key).await
}

/// Background job that frequently (with the given interval) cleans "Completed" jobs
/// or jobs with retries >= scheduler_max_retries set through environment config
#[inline]
pub async fn clean_complete() -> Result<()> {
    CLIENT.clean_complete().await
}

/// Background job that watches for timeout of a job
/// Steps:
/// - Select all the records with status = "Processing"
/// - calculate the current timestamp and difference from `start_time` of each record
/// - Get the record ids with difference more than the given timeout
/// - Update their status back to "Waiting" and increase their "retries" by 1
#[inline]
pub async fn watch_timeout() -> Result<()> {
    CLIENT.watch_timeout().await
}

/// The count of jobs for the given module (Report/Alert etc.)
#[inline]
pub async fn len_module(module: TriggerModule) -> usize {
    CLIENT.len_module(module).await
}

#[inline]
pub async fn len() -> usize {
    CLIENT.len().await
}

/// List the jobs for the given module
#[inline]
pub async fn list(module: Option<TriggerModule>) -> Result<Vec<Trigger>> {
    CLIENT.list(module).await
}

#[inline]
pub async fn is_empty() -> bool {
    CLIENT.is_empty().await
}

#[inline]
pub async fn clear() -> Result<()> {
    CLIENT.clear().await
}
