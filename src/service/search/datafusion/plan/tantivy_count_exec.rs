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

use std::{any::Any, sync::Arc};

use arrow::array::{Array, Int64Array};
use config::meta::{inverted_index::InvertedIndexOptimizeMode, stream::FileKey};
use datafusion::{
    arrow::{array::RecordBatch, datatypes::SchemaRef},
    common::{internal_err, Result, Statistics},
    error::DataFusionError,
    execution::{SendableRecordBatchStream, TaskContext},
    physical_expr::{EquivalenceProperties, Partitioning},
    physical_plan::{
        stream::RecordBatchStreamAdapter, DisplayAs, DisplayFormatType, ExecutionMode,
        ExecutionPlan, PlanProperties,
    },
};

use crate::service::search::{
    grpc::{storage::filter_file_list_by_tantivy_index, QueryParams},
    index::IndexCondition,
};

#[derive(Debug)]
pub struct TantivyCountExec {
    query: Arc<QueryParams>,
    schema: SchemaRef,               // The schema for the produced row
    file_list: Vec<FileKey>,         // The list of files to read
    index_condition: IndexCondition, // The condition to filter the rows
    cache: PlanProperties,           // Cached properties of this plan
}

impl TantivyCountExec {
    /// Create a new TantivyCountExec
    pub fn new(
        query: Arc<QueryParams>,
        schema: SchemaRef,
        file_list: Vec<FileKey>,
        index_condition: IndexCondition,
    ) -> Self {
        let cache = Self::compute_properties(Arc::clone(&schema));
        TantivyCountExec {
            query,
            schema,
            file_list,
            index_condition,
            cache,
        }
    }

    fn compute_properties(schema: SchemaRef) -> PlanProperties {
        PlanProperties::new(
            EquivalenceProperties::new(schema.clone()),
            // Output Partitioning
            Partitioning::UnknownPartitioning(1),
            // Execution Mode
            ExecutionMode::Bounded,
        )
    }
}

impl DisplayAs for TantivyCountExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // display up to 5 file keys,
        let file_keys = self
            .file_list
            .iter()
            .take(5)
            .map(|file_key| file_key.key.clone())
            .collect::<Vec<String>>()
            .join(", ");
        write!(
            f,
            "TantivyCountExec: file num: {}, file_list: [{file_keys}]",
            self.file_list.len()
        )
    }
}

impl ExecutionPlan for TantivyCountExec {
    fn name(&self) -> &'static str {
        "TantivyCountExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.cache
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if partition >= 1 {
            return internal_err!(
                "TantivyCountExec invalid partition {partition} (expected partition: 0)"
            );
        }

        let fut = adapt_tantivy_result(
            self.query.clone(),
            self.file_list.clone(),
            Some(self.index_condition.clone()),
            self.schema.clone(),
        );
        let stream = futures::stream::once(fut);
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema.clone(),
            stream,
        )))
    }

    fn statistics(&self) -> Result<Statistics> {
        Ok(Statistics::new_unknown(&self.schema))
    }
}

async fn adapt_tantivy_result(
    query: Arc<QueryParams>,
    mut file_list: Vec<FileKey>,
    index_condition: Option<IndexCondition>,
    schema: SchemaRef,
) -> Result<RecordBatch> {
    let (idx_took, error, total_hits) = filter_file_list_by_tantivy_index(
        query.clone(),
        &mut file_list,
        index_condition,
        Some(InvertedIndexOptimizeMode::SimpleCount),
    )
    .await
    .map_err(|e| DataFusionError::External(Box::new(e)))?;

    if error {
        return internal_err!("Error while filtering file list by Tantivy index");
    }

    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, TantivyCountExec execution time {} ms",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        idx_took
    );

    let array = vec![Arc::new(Int64Array::from(vec![total_hits as i64])) as Arc<dyn Array>];

    RecordBatch::try_new(schema, array).map_err(|e| {
        DataFusionError::Internal(format!("TantivyCountExec create record batch error: {e}",))
    })
}
