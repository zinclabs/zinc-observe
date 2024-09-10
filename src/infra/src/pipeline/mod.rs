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

use std::sync::Arc;

use async_trait::async_trait;
use config::meta::{meta_store::MetaStore, pipeline::Pipeline, stream::StreamParams};
use once_cell::sync::Lazy;
use tokio::sync::mpsc;

use crate::{db::Event, errors::Result};

pub mod mysql;
pub mod postgres;
pub mod sqlite;

static CLIENT: Lazy<Box<dyn PipelineTable>> = Lazy::new(connect);

pub fn connect() -> Box<dyn PipelineTable> {
    match config::get_config().common.meta_store.as_str().into() {
        MetaStore::MySQL => Box::<mysql::MySqlPipelineTable>::default(),
        MetaStore::PostgreSQL => Box::<postgres::PostgresPipelineTable>::default(),
        _ => Box::<sqlite::SqlitePipelineTable>::default(),
    }
}

#[async_trait]
pub trait PipelineTable: Sync + Send + 'static {
    async fn create_table(&self) -> Result<()>;
    async fn create_table_index(&self) -> Result<()>;
    async fn put(&self, org_id: &str, pipeline: Pipeline) -> Result<()>;
    async fn get_by_stream(
        &self,
        org_id: &str,
        stream_params: &StreamParams,
    ) -> Result<Vec<Pipeline>>;
    async fn get_by_id(&self, org_id: &str, pipeline_id: &str) -> Result<Pipeline>;
    async fn list(&self, org_id: &str) -> Result<Vec<Pipeline>>;
    async fn delete(&self, org_id: &str, pipeline_id: &str) -> Result<()>;
    async fn watch(&self, prefix: &str) -> Result<Arc<mpsc::Receiver<Event>>>;
}

/// Initializes the cached PipelinedTable - creates table and index
pub async fn init() -> Result<()> {
    CLIENT.create_table().await?;
    CLIENT.create_table_index().await?;
    Ok(())
}

/// Creates a pipeline entry in the table
#[inline]
pub async fn put(org_id: &str, pipeline: Pipeline) -> Result<()> {
    CLIENT.put(org_id, pipeline).await
}

/// Returns all pipelines associated with StreamParams within an organization
#[inline]
pub async fn get_by_stream(org_id: &str, stream_params: &StreamParams) -> Result<Vec<Pipeline>> {
    CLIENT.get_by_stream(org_id, stream_params).await
}

/// Returns all pipelines by id within an organization
#[inline]
pub async fn get(org_id: &str, pipeline_id: &str) -> Result<Pipeline> {
    CLIENT.get_by_id(org_id, pipeline_id).await
}

/// List all the pipelines within an organization
#[inline]
pub async fn list(org_id: &str) -> Result<Vec<Pipeline>> {
    CLIENT.list(org_id).await
}

/// Deletes the Pipeline identified by org_id and pipeline_id
#[inline]
pub async fn delete(org_id: &str, pipeline_id: &str) -> Result<()> {
    CLIENT.delete(org_id, pipeline_id).await
}

// impl<R: sqlx::Row> sqlx::FromRow<'_, R> for Pipeline<R> {
//     fn from_row(row: &'_ R) -> std::result::Result<Self, sqlx::Error> {
//         Ok(Pipeline::default())
//     }
// }
