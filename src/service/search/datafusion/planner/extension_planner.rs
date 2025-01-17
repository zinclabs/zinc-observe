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

use std::sync::Arc;

use async_trait::async_trait;
use datafusion::{
    common::Result as DataFusionResult,
    error::DataFusionError,
    execution::{context::QueryPlanner, SessionState},
    logical_expr::{LogicalPlan, UserDefinedLogicalNode},
    physical_plan::{expressions::Column, ExecutionPlan},
    physical_planner::{DefaultPhysicalPlanner, ExtensionPlanner, PhysicalPlanner},
};

use crate::service::search::datafusion::plan::{
    deduplication::DeduplicationLogicalNode, deduplication_exec::DeduplicationExec,
};

// A query planner that wrap datafusion's default planner with extension planner
#[derive(Debug)]
pub struct OpenobserveQueryPlanner {}

impl OpenobserveQueryPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryPlanner for OpenobserveQueryPlanner {
    async fn create_physical_plan(
        &self,
        logical_plan: &LogicalPlan,
        session_state: &SessionState,
    ) -> DataFusionResult<Arc<dyn ExecutionPlan>> {
        let planners: Vec<Arc<dyn ExtensionPlanner + Send + Sync>> =
            vec![Arc::new(DeduplicationExecPlanner::new())];

        DefaultPhysicalPlanner::with_extension_planners(planners)
            .create_physical_plan(logical_plan, session_state)
            .await
    }
}

/// A physical planner that convert a `DeduplicationLogicalNode` into `DeduplicationExec`,
#[derive(Debug)]
pub(crate) struct DeduplicationExecPlanner {}

impl DeduplicationExecPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ExtensionPlanner for DeduplicationExecPlanner {
    async fn plan_extension(
        &self,
        _planner: &dyn PhysicalPlanner,
        node: &dyn UserDefinedLogicalNode,
        _logical_inputs: &[&LogicalPlan],
        physical_inputs: &[Arc<dyn ExecutionPlan>],
        _session_state: &SessionState,
    ) -> DataFusionResult<Option<Arc<dyn ExecutionPlan>>> {
        let Some(deduplication_node) = node.as_any().downcast_ref::<DeduplicationLogicalNode>()
        else {
            return Ok(None);
        };

        if physical_inputs.len() != 1 {
            return Err(DataFusionError::Plan(
                "DeduplicationExecPlanner expects only one input".to_string(),
            ));
        }

        let input = physical_inputs.first().unwrap();
        let schema = input.schema();

        let deduplication_columns = deduplication_node
            .deduplication_columns
            .iter()
            .map(|c| Column::new_with_schema(c.name(), &schema).unwrap())
            .collect();

        let max_rows = deduplication_node.max_rows;
        let deduplication_exec =
            DeduplicationExec::new(input.clone(), deduplication_columns, max_rows);
        Ok(Some(Arc::new(deduplication_exec)))
    }
}
