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

use std::{hash::Hash, sync::Arc};

use arrow_schema::Schema;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::try_join;

use crate::service::metadata::{
    distinct_values::{DistinctValues, DvItem},
    trace_list_index::{TraceListIndex, TraceListItem},
};

pub mod distinct_values;
pub mod trace_list_index;

static METADATA_MANAGER: Lazy<MetadataManager> = Lazy::new(MetadataManager::new);

#[derive(Debug, Eq, Hash, PartialEq, Clone, Serialize, Deserialize)]
pub enum MetadataItem {
    TraceListIndexer(TraceListItem),
    DistinctValues(DvItem),
}

pub enum MetadataType {
    TraceListIndexer,
    DistinctValues,
}

pub struct MetadataManager {
    trace_list_indexer: TraceListIndex,
    distinct_values: DistinctValues,
}

pub trait Metadata {
    fn generate_schema(&self) -> Arc<Schema>;
    fn write(
        &self,
        org_id: &str,
        data: Vec<MetadataItem>,
    ) -> impl std::future::Future<Output = infra::errors::Result<()>> + Send;
    fn flush(&self) -> impl std::future::Future<Output = infra::errors::Result<()>> + Send;
    fn stop(&self) -> impl std::future::Future<Output = infra::errors::Result<()>> + Send;
}

impl Default for MetadataManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataManager {
    pub fn new() -> Self {
        Self {
            trace_list_indexer: TraceListIndex::new(),
            distinct_values: DistinctValues::new(),
        }
    }

    pub async fn close(&self) -> infra::errors::Result<()> {
        match try_join!(self.trace_list_indexer.stop(), self.distinct_values.stop()) {
            Ok(_) => {}
            Err(e) => {
                log::error!("[METADATA] error while closing: {}", e);
            }
        }

        Ok(())
    }
}

pub async fn write(
    org_id: &str,
    mt: MetadataType,
    data: Vec<MetadataItem>,
) -> infra::errors::Result<()> {
    match mt {
        MetadataType::TraceListIndexer => {
            METADATA_MANAGER
                .trace_list_indexer
                .write(org_id, data)
                .await
        }
        MetadataType::DistinctValues => METADATA_MANAGER.distinct_values.write(org_id, data).await,
    }
}

pub async fn close() -> infra::errors::Result<()> {
    // flush metadata
    METADATA_MANAGER.close().await
}
