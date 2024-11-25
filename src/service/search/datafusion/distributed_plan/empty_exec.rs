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

use arrow_schema::SortOptions;
use config::get_config;
use datafusion::{
    arrow::{array::RecordBatch, datatypes::SchemaRef},
    common::{internal_err, Result, Statistics},
    execution::{SendableRecordBatchStream, TaskContext},
    physical_expr::{EquivalenceProperties, LexOrdering, Partitioning, PhysicalSortExpr},
    physical_plan::{
        common, expressions::Column, memory::MemoryStream, DisplayAs, DisplayFormatType,
        ExecutionMode, ExecutionPlan, PlanProperties,
    },
    prelude::Expr,
};

/// Execution plan for empty relation with produce_one_row=false
#[derive(Debug)]
pub struct NewEmptyExec {
    name: String,      // table name
    schema: SchemaRef, // The schema for the produced row
    partitions: usize, // Number of partitions
    cache: PlanProperties,
    projection: Option<Vec<usize>>,
    filters: Vec<Expr>,
    limit: Option<usize>,
    sorted_by_time: bool,
    full_schema: SchemaRef, // The schema use for remove filter feature
}

impl NewEmptyExec {
    /// Create a new NewEmptyExec
    pub fn new(
        name: &str,
        schema: SchemaRef,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
        sorted_by_time: bool,
        full_schema: SchemaRef,
    ) -> Self {
        let cache = Self::compute_properties(Arc::clone(&schema), 1, sorted_by_time);
        NewEmptyExec {
            name: name.to_string(),
            schema,
            partitions: 1,
            cache,
            projection: projection.cloned(),
            filters: filters.to_owned(),
            limit,
            sorted_by_time,
            full_schema,
        }
    }

    /// Create a new NewEmptyExec with specified partition number
    pub fn with_partitions(mut self, partitions: usize) -> Self {
        self.partitions = partitions;
        // Changing partitions may invalidate output partitioning, so update it:
        let output_partitioning = Self::output_partitioning_helper(self.partitions);
        self.cache = self.cache.with_partitioning(output_partitioning);
        self
    }

    fn data(&self) -> Result<Vec<RecordBatch>> {
        Ok(vec![])
    }

    fn output_partitioning_helper(n_partitions: usize) -> Partitioning {
        Partitioning::UnknownPartitioning(n_partitions)
    }

    /// This function creates the cache object that stores the plan properties such as schema,
    /// equivalence properties, ordering, partitioning, etc.
    fn compute_properties(
        schema: SchemaRef,
        n_partitions: usize,
        sorted_by_time: bool,
    ) -> PlanProperties {
        let index = schema.index_of(&get_config().common.column_timestamp);
        let eq_properties = if !sorted_by_time {
            EquivalenceProperties::new(schema)
        } else {
            match index {
                Ok(index) => {
                    let ordering = vec![LexOrdering::new(vec![PhysicalSortExpr {
                        expr: Arc::new(Column::new(&get_config().common.column_timestamp, index)),
                        options: SortOptions {
                            descending: true,
                            nulls_first: false,
                        },
                    }])];
                    EquivalenceProperties::new_with_orderings(schema, &ordering)
                }
                Err(_) => EquivalenceProperties::new(schema),
            }
        };
        let output_partitioning = Self::output_partitioning_helper(n_partitions);
        PlanProperties::new(
            eq_properties,
            // Output Partitioning
            output_partitioning,
            // Execution Mode
            ExecutionMode::Bounded,
        )
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn projection(&self) -> Option<&Vec<usize>> {
        self.projection.as_ref()
    }

    pub fn filters(&self) -> &[Expr] {
        &self.filters
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    pub fn sorted_by_time(&self) -> bool {
        self.sorted_by_time
    }

    pub fn full_schema(&self) -> SchemaRef {
        Arc::clone(&self.full_schema)
    }
}

impl DisplayAs for NewEmptyExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                let name_string = format!("name={:?}", self.name);
                let projection_string = format!(
                    ", projection={:?}",
                    self.schema
                        .fields()
                        .iter()
                        .map(|f| f.name())
                        .collect::<Vec<_>>()
                );
                let filters_string = format!(
                    ", filters={:?}",
                    self.filters
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                );
                let limit_string = self
                    .limit
                    .map_or_else(|| "".to_string(), |l| format!(", limit={}", l));
                let sorted_by_time_string = if self.sorted_by_time {
                    ", sorted_by_time=true"
                } else {
                    ""
                };

                write!(f, "NewEmptyExec: ")?;
                write!(
                    f,
                    "{}{}{}{}{}",
                    name_string,
                    projection_string,
                    filters_string,
                    limit_string,
                    sorted_by_time_string
                )
            }
        }
    }
}

impl ExecutionPlan for NewEmptyExec {
    fn name(&self) -> &'static str {
        "NewEmptyExec"
    }

    /// Return a reference to Any that can be used for downcasting
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
        if partition >= self.partitions {
            return internal_err!(
                "NewEmptyExec invalid partition {} (expected less than {})",
                partition,
                self.partitions
            );
        }

        Ok(Box::pin(MemoryStream::try_new(
            self.data()?,
            Arc::clone(&self.schema),
            None,
        )?))
    }

    fn statistics(&self) -> Result<Statistics> {
        let batch = self
            .data()
            .expect("Create empty RecordBatch should not fail");
        Ok(common::compute_record_batch_statistics(
            &[batch],
            &self.schema,
            None,
        ))
    }
}

// add some unit tests here
#[cfg(test)]
mod tests {
    use arrow::datatypes::{DataType, Field, Schema};

    use super::*;

    #[test]
    fn test_new_empty_exec() {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        let exec = NewEmptyExec::new("test", schema.clone(), None, &[], None, false, schema);
        assert_eq!(exec.name(), "test");
        assert_eq!(exec.projection(), None);
        assert_eq!(exec.filters().len(), 0);
        assert_eq!(exec.limit(), None);
    }
}
