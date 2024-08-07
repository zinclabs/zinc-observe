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

use arrow::array::RecordBatch;
use datafusion::{
    common::tree_node::{Transformed, TreeNodeRecursion, TreeNodeRewriter},
    physical_plan::{
        coalesce_batches::CoalesceBatchesExec, empty::EmptyExec, limit::GlobalLimitExec,
        memory::MemoryExec, repartition::RepartitionExec, ExecutionPlan, Partitioning,
    },
};

const DISTRIBUTED_PLAN_NAMES: [&str; 3] = [
    "CoalescePartitionsExec",
    "RepartitionExec",
    "SortPreservingMergeExec",
];

pub fn get_partial_plan(
    plan: &Arc<dyn ExecutionPlan>,
) -> Result<Option<Arc<dyn ExecutionPlan>>, datafusion::error::DataFusionError> {
    let children = plan.children();
    if children.is_empty() {
        return Ok(None);
    }
    if children.len() > 1 {
        return Err(datafusion::error::DataFusionError::NotImplemented(
            "ExecutionPlan with multiple children".to_string(),
        ));
    }

    let cplan = children.first().unwrap();
    for name in ["HashJoinExec", "CrossJoinExec"] {
        if cplan.name() == name {
            return Err(datafusion::error::DataFusionError::NotImplemented(
                name.to_string(),
            ));
        }
    }

    if cplan.name() == "RepartitionExec" {
        let plan = cplan.as_any().downcast_ref::<RepartitionExec>().unwrap();
        if let Partitioning::RoundRobinBatch(_) = plan.partitioning() {
            return Ok(None);
        }
    }
    if DISTRIBUTED_PLAN_NAMES.contains(&cplan.name()) {
        let child = *cplan.children().first().unwrap();
        if child.name() == "ParquetExec" {
            return Ok(None);
        }
        if let Some(v) = get_partial_plan(child)? {
            return Ok(Some(v));
        } else {
            if cplan.name() == "SortPreservingMergeExec" {
                return Ok(Some((*cplan).clone()));
            }
            return Ok(Some(child.clone()));
        }
    }
    get_partial_plan(cplan)
}

pub fn get_final_plan(
    plan: &Arc<dyn ExecutionPlan>,
    data: &[Vec<RecordBatch>],
) -> Result<(Option<Arc<dyn ExecutionPlan>>, bool), datafusion::error::DataFusionError> {
    let children = plan.children();
    if children.is_empty() {
        return Ok((Some(plan.clone()), false));
    }
    if children.len() > 1 {
        return Err(datafusion::error::DataFusionError::NotImplemented(
            "ExecutionPlan with multiple children".to_string(),
        ));
    }
    let mut found_dist = false;
    let mut cplan = (*children.first().unwrap()).clone();
    for name in ["HashJoinExec", "CrossJoinExec"] {
        if cplan.name() == name {
            return Err(datafusion::error::DataFusionError::NotImplemented(
                name.to_string(),
            ));
        }
    }

    if DISTRIBUTED_PLAN_NAMES.contains(&cplan.name()) {
        let child = *cplan.children().first().unwrap();
        if get_partial_plan(child)?.is_some() {
            if let (Some(v), dist) = get_final_plan(&cplan, data)? {
                cplan = v;
                if dist {
                    found_dist = true;
                }
            }
        } else {
            cplan = cplan
                .clone()
                .with_new_children(vec![Arc::new(
                    MemoryExec::try_new(data, cplan.schema(), None).unwrap(),
                )])
                .unwrap();
            found_dist = true;
        }
    } else if let (Some(v), dist) = get_final_plan(&cplan, data)? {
        cplan = v;
        if dist {
            found_dist = true;
        }
    }
    Ok((
        Some(plan.clone().with_new_children(vec![cplan]).unwrap()),
        found_dist,
    ))
}

pub fn get_without_dist_plan(
    plan: &Arc<dyn ExecutionPlan>,
    data: &[Vec<RecordBatch>],
    batch_size: usize,
) -> Arc<dyn ExecutionPlan> {
    if data.is_empty() {
        return Arc::new(EmptyExec::new(plan.schema()));
    }
    let schema = if plan.name() == "ProjectionExec" {
        data.first().unwrap().first().unwrap().schema()
    } else {
        plan.schema()
    };
    let memory_exec = MemoryExec::try_new(data, schema, None).unwrap();
    let coalesce_exec = CoalesceBatchesExec::new(Arc::new(memory_exec), batch_size);
    let coalesce_exec = Arc::new(coalesce_exec);
    if plan.name() == "ProjectionExec" {
        coalesce_exec
    } else {
        plan.clone().with_new_children(vec![coalesce_exec]).unwrap()
    }
}

/// Rewriter to update the offset and limit of a GlobalLimitExec node
pub(crate) struct UpdateOffsetExec {
    offset: usize,
    limit: usize,
}

impl UpdateOffsetExec {
    pub fn new(offset: usize, limit: usize) -> Self {
        Self { offset, limit }
    }
}

impl TreeNodeRewriter for UpdateOffsetExec {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_down(
        &mut self,
        node: Self::Node,
    ) -> Result<Transformed<Self::Node>, datafusion::error::DataFusionError> {
        if let Some(node) = node.as_any().downcast_ref::<GlobalLimitExec>() {
            if let Some(fetch) = node.fetch() {
                if fetch > self.limit {
                    let node =
                        GlobalLimitExec::new(node.input().clone(), self.offset, Some(self.limit));
                    return Ok(Transformed::new(
                        Arc::new(node),
                        true,
                        TreeNodeRecursion::Stop,
                    ));
                }
            }
            let node = GlobalLimitExec::new(node.input().clone(), node.skip(), node.fetch());
            Ok(Transformed::new(
                Arc::new(node),
                false,
                TreeNodeRecursion::Stop,
            ))
        } else if node.name() == "ProjectionExec" {
            Ok(Transformed::new(node, false, TreeNodeRecursion::Continue))
        } else {
            Ok(Transformed::new(node, false, TreeNodeRecursion::Stop))
        }
    }
}
