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

use arrow::array::{RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use async_trait::async_trait;
use datafusion::{
    common::{
        cast::as_string_array,
        tree_node::{Transformed, TreeNode},
        Column, DFSchemaRef, Result, Statistics, ToDFSchema,
    },
    error::DataFusionError,
    execution::{context::QueryPlanner, SendableRecordBatchStream, SessionState, TaskContext},
    logical_expr::{
        expr::ScalarFunction, Extension, LogicalPlan, Projection, Subquery, UserDefinedLogicalNode,
        UserDefinedLogicalNodeCore,
    },
    optimizer::{optimizer::ApplyOrder, utils::NamePreserver, OptimizerConfig, OptimizerRule},
    physical_expr::EquivalenceProperties,
    physical_plan::{
        stream::RecordBatchStreamAdapter, DisplayAs, DisplayFormatType, Distribution,
        ExecutionMode, ExecutionPlan, ExecutionPlanProperties, Partitioning, PlanProperties,
    },
    physical_planner::{DefaultPhysicalPlanner, ExtensionPlanner, PhysicalPlanner},
    prelude::Expr,
    scalar::ScalarValue,
};
use futures::{FutureExt, StreamExt};

// NOTE: This is the actual handler for decrypt function, implemented based on
// https://github.com/apache/datafusion/pull/6713/files and
// https://github.com/apache/datafusion/blob/main/datafusion/core/tests/user_defined/user_defined_plan.rs

async fn decrypt(batch: Result<RecordBatch>, _key_name: String) -> Result<RecordBatch> {
    println!("decrypt-start");
    let t = batch.map(|b| {
        // 1. get columns from batch and cast them
        let args = b.columns();
        // TODO handle the error case instead of expect
        let encrypted = match as_string_array(&args[0]) {
            Ok(v) => v,
            Err(_) => return b,
        };

        // 2. perform the computation
        let array = encrypted
            .iter()
            .map(|val| {
                match val {
                    // in arrow, any value can be null.
                    // Here we decide to make our UDF to return null when either base or exponent is
                    // null.
                    Some(value) => {
                        // log::warn!("IN DECRYPT EXEC, value {value}, key {key_name}");
                        Some(value)
                    }
                    _ => None,
                }
            })
            .collect::<StringArray>();

        // 3. Define a new schema and construct a new RecordBatch with the computed data
        let schema = Arc::new(Schema::new(vec![Field::new(
            "decrypted",
            DataType::Utf8,
            false,
        )]));
        RecordBatch::try_new(schema, vec![Arc::new(array)]).unwrap()
    });
    println!("decrypt-end");
    t
}

#[derive(Debug)]
pub struct DecryptQueryPlanner {}

impl DecryptQueryPlanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl QueryPlanner for DecryptQueryPlanner {
    // Given a `LogicalPlan` created from above, create an `ExecutionPlan` suitable for execution
    async fn create_physical_plan(
        &self,
        logical_plan: &LogicalPlan,
        session_state: &SessionState,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Teach the default physical planner how to plan Pow nodes.
        let physical_planner =
            DefaultPhysicalPlanner::with_extension_planners(vec![Arc::new(DecryptPlanner {})]);
        // Delegate most work of physical planning to the default physical planner
        physical_planner
            .create_physical_plan(logical_plan, session_state)
            .await
    }
}

// Physical planner for Pow nodes
struct DecryptPlanner {}

#[async_trait]
impl ExtensionPlanner for DecryptPlanner {
    // Create a physical plan for an extension node
    async fn plan_extension(
        &self,
        _planner: &dyn PhysicalPlanner,
        node: &dyn UserDefinedLogicalNode,
        _logical_inputs: &[&LogicalPlan],
        physical_inputs: &[Arc<dyn ExecutionPlan>],
        _session_state: &SessionState,
    ) -> Result<Option<Arc<dyn ExecutionPlan>>> {
        Ok(
            if let Some(n) = node.as_any().downcast_ref::<DecryptNode>() {
                Some(Arc::new(DecryptExec::new(
                    physical_inputs.to_vec(),
                    &n.key_name,
                )))
            } else {
                None
            },
        )
    }
}

fn rewrite_expr(expr: Expr, original_plan: Arc<LogicalPlan>) -> Result<Transformed<Expr>> {
    expr.transform(&|expr| {
        // closure is invoked for all sub expressions
        Ok(match &expr {
            Expr::ScalarFunction(ScalarFunction { ref func, ref args }) => {
                if func.name() == "decrypt" {
                    let arg0 = if let Expr::Column(Column { name, .. }) = &args[0] {
                        name
                    } else {
                        // TODO: fix this with proper error
                        panic!("panicking over here");
                    };
                    let arg1 = if let Expr::Literal(ScalarValue::Utf8(val)) = &args[1] {
                        val.as_ref().unwrap()
                    } else {
                        // TODO: fix this with proper error
                        panic!("over here too");
                    };
                    let schema = Schema::new(vec![Field::new(arg0, DataType::Utf8, false)])
                        .to_dfschema_ref()?;

                    // the Extension is how datafusion knows it is an user-defined plan
                    let subplan = LogicalPlan::Extension(Extension {
                        node: Arc::new(DecryptNode {
                            expr: args.to_vec(),
                            key_name: arg1.clone(),
                            input: (*original_plan).clone(),
                        }),
                    });

                    // we need to wrap it in projection because if we don't,
                    // 1. ScalarSubqueryToJoin optimizer fails
                    // 2. if somehow that passes, datafusion errors with schema mismatch error
                    let subplan = LogicalPlan::Projection(Projection::new_from_schema(
                        Arc::new(subplan),
                        schema.clone(),
                    ));

                    Transformed::yes(Expr::ScalarSubquery(Subquery {
                        subquery: Arc::new(subplan),
                        outer_ref_columns: vec![expr],
                    }))
                } else {
                    Transformed::no(expr)
                }
            }
            _ => Transformed::no(expr),
        })
    })
}

#[derive(Debug)]
pub struct DecryptOptimizerRule {}

impl DecryptOptimizerRule {
    pub fn new() -> Self {
        Self {}
    }
}

impl OptimizerRule for DecryptOptimizerRule {
    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>, DataFusionError> {
        let new_plan = match &plan {
            // decrypt function call weill only be in projection plans
            LogicalPlan::Projection(p) => {
                let name_preserver = NamePreserver::new(&plan);
                let original_plan = Arc::new((*p.input).clone());

                plan.map_expressions(|expr| {
                    let original_name = name_preserver.save(&expr);
                    rewrite_expr(expr, original_plan.clone()).map(|transformed| {
                        transformed.update_data(|data| original_name.restore(data))
                    })
                })?
            }
            _ => Transformed::no(plan),
        };
        Ok(new_plan)
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::BottomUp)
    }

    fn name(&self) -> &str {
        "decrypt"
    }
}

#[derive(PartialEq, Eq, PartialOrd, Hash)]
struct DecryptNode {
    input: LogicalPlan,
    key_name: String,
    expr: Vec<Expr>,
}

impl std::fmt::Debug for DecryptNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "decrypt expr.len({})", self.expr.len())
    }
}

impl UserDefinedLogicalNodeCore for DecryptNode {
    fn name(&self) -> &str {
        "decrypt"
    }

    fn with_exprs_and_inputs(
        &self,
        exprs: Vec<Expr>,
        mut inputs: Vec<LogicalPlan>,
    ) -> Result<Self> {
        Ok(Self {
            input: inputs.swap_remove(0),
            key_name: self.key_name.clone(),
            expr: exprs,
        })
    }

    fn inputs(&self) -> Vec<&LogicalPlan> {
        vec![&self.input]
    }

    fn schema(&self) -> &DFSchemaRef {
        &self.input.schema()
    }

    fn expressions(&self) -> Vec<Expr> {
        self.expr.clone()
    }

    fn fmt_for_explain(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "decrypt expr.len({})", self.expr.len())
    }

    fn from_template(&self, exprs: &[Expr], inputs: &[LogicalPlan]) -> Self {
        Self {
            input: inputs[0].clone(),
            key_name: self.key_name.to_string(),
            expr: exprs.to_vec(),
        }
    }
}

struct DecryptExec {
    prop_cache: PlanProperties,
    key_name: String,
    inputs: Vec<Arc<dyn ExecutionPlan>>,
}

impl DecryptExec {
    fn new(inputs: Vec<Arc<dyn ExecutionPlan>>, key: &str) -> Self {
        let partitioning = inputs[0].output_partitioning();

        let prop_cache = Self::compute_properties(partitioning, inputs[0].schema());
        Self {
            inputs,
            prop_cache,
            key_name: key.to_string(),
        }
    }

    /// This function creates the cache object that stores the plan properties such as schema,
    /// equivalence properties, ordering, partitioning, etc.
    // I tried using just Unknown partitioning, that does not give any benefit
    fn compute_properties(partition: &Partitioning, schema: SchemaRef) -> PlanProperties {
        let eq_properties = EquivalenceProperties::new(schema);
        let partitions = match partition {
            Partitioning::RoundRobinBatch(n) => Partitioning::RoundRobinBatch(*n),
            Partitioning::Hash(_, n) => Partitioning::UnknownPartitioning(*n),
            Partitioning::UnknownPartitioning(n) => Partitioning::UnknownPartitioning(*n),
        };

        PlanProperties::new(eq_properties, partitions, ExecutionMode::Bounded)
    }
}

impl std::fmt::Debug for DecryptExec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DecryptExec")
    }
}

impl DisplayAs for DecryptExec {
    fn fmt_as(&self, _: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DecryptExec")
    }
}

#[async_trait]
impl ExecutionPlan for DecryptExec {
    // Return a reference to Any that can be used for downcasting
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "DecryptExec"
    }
    fn properties(&self) -> &PlanProperties {
        &self.prop_cache
    }

    fn required_input_distribution(&self) -> Vec<Distribution> {
        vec![Distribution::SinglePartition]
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        self.inputs.iter().collect()
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(DecryptExec::new(children, &self.key_name)))
    }

    // Execute one partition and return an iterator over `RecordBatch`.
    // The iterator calls the user defined async function `pow`.
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        if self.properties().output_partitioning().partition_count() <= partition {
            return Err(DataFusionError::Internal(format!(
                "DecryptExec invalid partition {partition}"
            )));
        }
        let key_name = self.key_name.to_owned();
        let s = self.inputs[0]
            .execute(partition, context)?
            .map(move |b| {
                let t = decrypt(b, key_name.clone()).into_stream();

                t
            })
            .flatten();
        let s = RecordBatchStreamAdapter::new(self.schema(), s);
        Ok(Box::pin(s))
    }

    fn statistics(&self) -> Result<Statistics, DataFusionError> {
        // to improve the optimizability of this plan
        // better statistics inference could be provided
        Ok(Statistics::new_unknown(&self.schema()))
    }
}
