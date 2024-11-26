// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

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

//! Helper functions for the table implementation

use std::sync::Arc;

use arrow_schema::{DataType, Schema, SchemaRef};
use config::{INDEX_SEGMENT_LENGTH, PARQUET_MAX_ROW_GROUP_SIZE};
use datafusion::{
    common::{
        project_schema,
        stats::Precision,
        tree_node::{TreeNode, TreeNodeRecursion},
        Column, DataFusionError, Result,
    },
    datasource::{
        listing::{ListingTableUrl, PartitionedFile},
        physical_plan::parquet::ParquetAccessPlan,
    },
    logical_expr::{Expr, Volatility},
    parquet::arrow::arrow_reader::{RowSelection, RowSelector},
    physical_plan::{
        expressions::CastExpr, filter::FilterExec, projection::ProjectionExec, ExecutionPlan,
        PhysicalExpr,
    },
};
use futures::{stream::BoxStream, TryStreamExt};
use hashbrown::HashMap;
use object_store::ObjectStore;

use crate::service::search::{datafusion::storage, index::IndexCondition};

/// Check whether the given expression can be resolved using only the columns `col_names`.
/// This means that if this function returns true:
/// - the table provider can filter the table partition values with this expression
/// - the expression can be marked as `TableProviderFilterPushDown::Exact` once this filtering was
///   performed
pub fn expr_applicable_for_cols(col_names: &[String], expr: &Expr) -> bool {
    let mut is_applicable = true;
    expr.apply(|expr| {
        match expr {
            Expr::Column(Column { ref name, .. }) => {
                is_applicable &= col_names.contains(name);
                if is_applicable {
                    Ok(TreeNodeRecursion::Jump)
                } else {
                    Ok(TreeNodeRecursion::Stop)
                }
            }
            Expr::Literal(_)
            | Expr::Alias(_)
            | Expr::OuterReferenceColumn(..)
            | Expr::ScalarVariable(..)
            | Expr::Not(_)
            | Expr::IsNotNull(_)
            | Expr::IsNull(_)
            | Expr::IsTrue(_)
            | Expr::IsFalse(_)
            | Expr::IsUnknown(_)
            | Expr::IsNotTrue(_)
            | Expr::IsNotFalse(_)
            | Expr::IsNotUnknown(_)
            | Expr::Negative(_)
            | Expr::Cast { .. }
            | Expr::TryCast { .. }
            | Expr::BinaryExpr { .. }
            | Expr::Between { .. }
            | Expr::Like { .. }
            | Expr::SimilarTo { .. }
            | Expr::InList { .. }
            | Expr::Exists { .. }
            | Expr::InSubquery(_)
            | Expr::ScalarSubquery(_)
            | Expr::GroupingSet(_)
            | Expr::Case { .. } => Ok(TreeNodeRecursion::Continue),

            Expr::ScalarFunction(scalar_function) => {
                match scalar_function.func.signature().volatility {
                    Volatility::Immutable => Ok(TreeNodeRecursion::Continue),
                    // TODO: Stable functions could be `applicable`, but that would require access
                    // to the context
                    Volatility::Stable | Volatility::Volatile => {
                        is_applicable = false;
                        Ok(TreeNodeRecursion::Stop)
                    }
                }
            }

            // TODO other expressions are not handled yet:
            // - AGGREGATE, WINDOW and SORT should not end up in filter conditions, except maybe in
            //   some edge cases
            // - Can `Wildcard` be considered as a `Literal`?
            // - ScalarVariable could be `applicable`, but that would require access to the context
            Expr::AggregateFunction { .. }
            | Expr::WindowFunction { .. }
            | Expr::Wildcard { .. }
            | Expr::Unnest { .. }
            | Expr::Placeholder(_) => {
                is_applicable = false;
                Ok(TreeNodeRecursion::Stop)
            }
        }
    })
    .unwrap();
    is_applicable
}

/// List all files in the table path
pub async fn list_files<'a>(
    store: &'a dyn ObjectStore,
    table_path: &'a ListingTableUrl,
) -> Result<BoxStream<'a, Result<PartitionedFile>>> {
    Ok(Box::pin(
        store
            .list(Some(table_path.prefix()))
            .map_err(DataFusionError::ObjectStore)
            .map_ok(|object_meta| object_meta.into()),
    ))
}

/// Partition the list of files into `n` groups
pub fn split_files(
    mut partitioned_files: Vec<PartitionedFile>,
    n: usize,
) -> Vec<Vec<PartitionedFile>> {
    if partitioned_files.is_empty() {
        return vec![];
    }

    // ObjectStore::list does not guarantee any consistent order and for some
    // implementations such as LocalFileSystem, it may be inconsistent. Thus
    // Sort files by path to ensure consistent plans when run more than once.
    partitioned_files.sort_by(|a, b| a.path().cmp(b.path()));

    // effectively this is div with rounding up instead of truncating
    let chunk_size = partitioned_files.len().div_ceil(n);
    partitioned_files
        .chunks(chunk_size)
        .map(|c| c.to_vec())
        .collect()
}

pub fn generate_access_plan(file: &PartitionedFile) -> Option<Arc<ParquetAccessPlan>> {
    if config::get_config()
        .common
        .inverted_index_search_format
        .eq("tantivy")
    {
        return generate_access_plan_row_level(file);
    };
    let index_segment_length = INDEX_SEGMENT_LENGTH;
    let segment_ids = storage::file_list::get_segment_ids(file.path().as_ref())?;
    let stats = file.statistics.as_ref()?;
    let Precision::Exact(num_rows) = stats.num_rows else {
        return None;
    };
    let row_group_count = num_rows.div_ceil(PARQUET_MAX_ROW_GROUP_SIZE);
    let segment_count = num_rows.div_ceil(index_segment_length);
    let mut access_plan = ParquetAccessPlan::new_none(row_group_count);
    let mut selection = Vec::with_capacity(segment_ids.len());
    let mut last_group_id = 0;
    for (segment_id, val) in segment_ids.iter().enumerate() {
        if segment_id >= segment_count {
            break;
        }
        let row_group_id = (segment_id * index_segment_length) / PARQUET_MAX_ROW_GROUP_SIZE;
        if row_group_id != last_group_id && !selection.is_empty() {
            if selection.iter().any(|s: &RowSelector| !s.skip) {
                access_plan.scan(last_group_id);
                access_plan.scan_selection(last_group_id, RowSelection::from(selection.clone()));
            }
            selection.clear();
            last_group_id = row_group_id;
        }
        let length = if (segment_id + 1) * index_segment_length > num_rows {
            num_rows % index_segment_length
        } else {
            index_segment_length
        };
        if *val {
            selection.push(RowSelector::select(length));
        } else {
            selection.push(RowSelector::skip(length));
        }
    }
    if !selection.is_empty() && selection.iter().any(|s: &RowSelector| !s.skip) {
        access_plan.scan(last_group_id);
        access_plan.scan_selection(last_group_id, RowSelection::from(selection));
    }
    log::debug!(
        "file path: {:?}, row_group_count: {}, access_plan: {:?}",
        file.path().as_ref(),
        row_group_count,
        access_plan
    );
    Some(Arc::new(access_plan))
}

pub fn generate_access_plan_row_level(file: &PartitionedFile) -> Option<Arc<ParquetAccessPlan>> {
    let row_ids = storage::file_list::get_segment_ids(file.path().as_ref())?;
    let stats = file.statistics.as_ref()?;
    let Precision::Exact(num_rows) = stats.num_rows else {
        return None;
    };
    let row_group_count = num_rows.div_ceil(PARQUET_MAX_ROW_GROUP_SIZE);
    let mut access_plan = ParquetAccessPlan::new_none(row_group_count);

    for (row_group_id, chunk) in row_ids.chunks(PARQUET_MAX_ROW_GROUP_SIZE).enumerate() {
        let mut selection = Vec::new();
        let mut current_count = 0;
        let mut current_select = false;

        for val in chunk
            .iter()
            .take(num_rows - row_group_id * PARQUET_MAX_ROW_GROUP_SIZE)
        {
            if *val == current_select {
                current_count += 1;
            } else {
                if current_count > 0 {
                    if current_select {
                        selection.push(RowSelector::select(current_count));
                    } else {
                        selection.push(RowSelector::skip(current_count));
                    }
                }
                current_select = *val;
                current_count = 1;
            }
        }

        // handle the last batch
        if current_count > 0 {
            if current_select {
                selection.push(RowSelector::select(current_count));
            } else {
                selection.push(RowSelector::skip(current_count));
            }
        }

        if selection.iter().any(|s| !s.skip) {
            access_plan.scan(row_group_id);
            access_plan.scan_selection(row_group_id, RowSelection::from(selection));
        }
    }

    log::debug!(
        "file path: {:?}, row_group_count: {}, access_plan: {:?}",
        file.path().as_ref(),
        row_group_count,
        access_plan
    );
    Some(Arc::new(access_plan))
}

fn wrap_filter(
    index_condition: &IndexCondition,
    schema: &Schema,
    fst_fields: &[String],
    exec: Arc<dyn ExecutionPlan>,
    projection: Option<&Vec<usize>>,
) -> Result<Arc<dyn ExecutionPlan>> {
    let expr = index_condition
        .to_physical_expr(schema, fst_fields)
        .map_err(|e| DataFusionError::External(e.into()))?;

    Ok(Arc::new(
        FilterExec::try_new(expr, exec)?.with_projection(projection.cloned())?,
    ))
}

pub fn apply_projection(
    schema: &SchemaRef,
    diff_rules: &HashMap<String, DataType>,
    projection: Option<&Vec<usize>>,
    memory_exec: Arc<dyn ExecutionPlan>,
) -> Result<Arc<dyn ExecutionPlan>> {
    if diff_rules.is_empty() {
        return Ok(memory_exec);
    }
    let projected_schema = project_schema(schema, projection)?;
    let mut exprs: Vec<(Arc<dyn PhysicalExpr>, String)> =
        Vec::with_capacity(projected_schema.fields().len());
    for (idx, field) in projected_schema.fields().iter().enumerate() {
        let name = field.name().to_string();
        let col = Arc::new(datafusion::physical_expr::expressions::Column::new(
            &name, idx,
        ));
        if let Some(data_type) = diff_rules.get(&name) {
            exprs.push((Arc::new(CastExpr::new(col, data_type.clone(), None)), name));
        } else {
            exprs.push((col, name));
        }
    }
    Ok(Arc::new(ProjectionExec::try_new(exprs, memory_exec)?))
}

pub fn apply_filter(
    index_condition: Option<&IndexCondition>,
    schema: &Schema,
    fst_fields: &[String],
    exec_plan: Arc<dyn ExecutionPlan>,
    filter_projection: Option<&Vec<usize>>,
) -> Result<Arc<dyn ExecutionPlan>> {
    if let Some(condition) = index_condition {
        wrap_filter(condition, schema, fst_fields, exec_plan, filter_projection)
    } else {
        Ok(exec_plan)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use datafusion::logical_expr::{case, col, lit};

    use super::*;

    #[test]
    fn test_split_files() {
        let new_partitioned_file = |path: &str| PartitionedFile::new(path.to_owned(), 10);
        let files = vec![
            new_partitioned_file("a"),
            new_partitioned_file("b"),
            new_partitioned_file("c"),
            new_partitioned_file("d"),
            new_partitioned_file("e"),
        ];

        let chunks = split_files(files.clone(), 1);
        assert_eq!(1, chunks.len());
        assert_eq!(5, chunks[0].len());

        let chunks = split_files(files.clone(), 2);
        assert_eq!(2, chunks.len());
        assert_eq!(3, chunks[0].len());
        assert_eq!(2, chunks[1].len());

        let chunks = split_files(files.clone(), 5);
        assert_eq!(5, chunks.len());
        assert_eq!(1, chunks[0].len());
        assert_eq!(1, chunks[1].len());
        assert_eq!(1, chunks[2].len());
        assert_eq!(1, chunks[3].len());
        assert_eq!(1, chunks[4].len());

        let chunks = split_files(files, 123);
        assert_eq!(5, chunks.len());
        assert_eq!(1, chunks[0].len());
        assert_eq!(1, chunks[1].len());
        assert_eq!(1, chunks[2].len());
        assert_eq!(1, chunks[3].len());
        assert_eq!(1, chunks[4].len());

        let chunks = split_files(vec![], 2);
        assert_eq!(0, chunks.len());
    }

    #[test]
    fn test_expr_applicable_for_cols() {
        assert!(expr_applicable_for_cols(
            &[String::from("c1")],
            &Expr::eq(col("c1"), lit("value"))
        ));
        assert!(!expr_applicable_for_cols(
            &[String::from("c1")],
            &Expr::eq(col("c2"), lit("value"))
        ));
        assert!(!expr_applicable_for_cols(
            &[String::from("c1")],
            &Expr::eq(col("c1"), col("c2"))
        ));
        assert!(expr_applicable_for_cols(
            &[String::from("c1"), String::from("c2")],
            &Expr::eq(col("c1"), col("c2"))
        ));
        assert!(expr_applicable_for_cols(
            &[String::from("c1"), String::from("c2")],
            &(Expr::eq(col("c1"), col("c2").alias("c2_alias"))).not()
        ));
        assert!(expr_applicable_for_cols(
            &[String::from("c1"), String::from("c2")],
            &(case(col("c1"))
                .when(lit("v1"), lit(true))
                .otherwise(lit(false))
                .expect("valid case expr"))
        ));
        // static expression not relevant in this context but we
        // test it as an edge case anyway in case we want to generalize
        // this helper function
        assert!(expr_applicable_for_cols(&[], &lit(true)));
    }
}
