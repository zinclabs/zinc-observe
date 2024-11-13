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

use config::get_config;
use datafusion::{
    common::{
        tree_node::{
            Transformed, TransformedResult, TreeNode, TreeNodeRecursion, TreeNodeRewriter,
        },
        DFSchema, Result,
    },
    logical_expr::{col, Limit, LogicalPlan, Projection, Sort, SortExpr, TableScan},
    optimizer::{optimizer::ApplyOrder, OptimizerConfig, OptimizerRule},
};

/// Optimization rule that add sort and limit to table scan
#[derive(Default)]
pub struct AddSortAndLimitRule {
    limit: usize,
    offset: usize,
}

impl AddSortAndLimitRule {
    #[allow(missing_docs)]
    pub fn new(limit: usize, offset: usize) -> Self {
        Self { limit, offset }
    }
}

impl OptimizerRule for AddSortAndLimitRule {
    fn name(&self) -> &str {
        "add_sort_and_limit"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::TopDown)
    }

    fn supports_rewrite(&self) -> bool {
        true
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        if self.limit == 0 {
            return Ok(Transformed::new(plan, false, TreeNodeRecursion::Stop));
        }

        let is_complex = plan.exists(|plan| Ok(is_complex_query(plan)))?;
        let mut is_stop = true;
        let (mut transformed, schema) = match plan {
            LogicalPlan::Projection(_) => {
                is_stop = false;
                (Transformed::no(plan), None)
            }
            LogicalPlan::Limit(mut limit) => match limit.input.as_ref() {
                LogicalPlan::Sort(_) => (Transformed::no(LogicalPlan::Limit(limit)), None),
                _ => {
                    if is_complex {
                        (Transformed::no(LogicalPlan::Limit(limit)), None)
                    } else {
                        // the add sort plan should reflect the limit
                        let fetch = limit.fetch.unwrap() + limit.skip;
                        let (sort, schema) = generate_sort_plan(limit.input.clone(), fetch);
                        limit.input = Arc::new(sort);
                        (Transformed::yes(LogicalPlan::Limit(limit)), schema)
                    }
                }
            },
            LogicalPlan::Sort(sort) => {
                if sort.fetch.is_some() {
                    (Transformed::no(LogicalPlan::Sort(sort)), None)
                } else {
                    let plan = generate_limit_plan(
                        Arc::new(LogicalPlan::Sort(sort)),
                        self.limit,
                        self.offset,
                    );
                    (Transformed::yes(plan), None)
                }
            }
            _ => {
                if is_complex {
                    (
                        Transformed::yes(generate_limit_plan(
                            Arc::new(plan),
                            self.limit,
                            self.offset,
                        )),
                        None,
                    )
                } else {
                    let (plan, schema) =
                        generate_limit_and_sort_plan(Arc::new(plan), self.limit, self.offset);
                    (Transformed::yes(plan), schema)
                }
            }
        };
        if is_stop {
            transformed.tnr = TreeNodeRecursion::Stop;
        }
        if let Some(schema) = schema {
            let plan = transformed.data;
            let proj = LogicalPlan::Projection(Projection::new_from_schema(Arc::new(plan), schema));
            transformed.data = proj;
        }
        Ok(transformed)
    }
}

// check if the plan is a complex query that we can't add sort _timestamp
fn is_complex_query(plan: &LogicalPlan) -> bool {
    matches!(
        plan,
        LogicalPlan::Aggregate(_)
            | LogicalPlan::Join(_)
            | LogicalPlan::CrossJoin(_)
            | LogicalPlan::Distinct(_)
            | LogicalPlan::RecursiveQuery(_)
            | LogicalPlan::SubqueryAlias(_)
            | LogicalPlan::Subquery(_)
            | LogicalPlan::Window(_)
    )
}

fn generate_limit_plan(input: Arc<LogicalPlan>, limit: usize, skip: usize) -> LogicalPlan {
    LogicalPlan::Limit(Limit {
        skip,
        fetch: Some(limit),
        input,
    })
}

fn generate_sort_plan(
    input: Arc<LogicalPlan>,
    limit: usize,
) -> (LogicalPlan, Option<Arc<DFSchema>>) {
    let config = get_config();
    let timestamp = SortExpr {
        expr: col(config.common.column_timestamp.clone()),
        asc: false,
        nulls_first: false,
    };
    let schema = input.schema().clone();
    if schema
        .field_with_name(None, config.common.column_timestamp.as_str())
        .is_err()
    {
        let mut input = input.as_ref().clone();
        input = input
            .rewrite(&mut ChangeTableScanSchema::new())
            .data()
            .unwrap();
        return (
            LogicalPlan::Sort(Sort {
                expr: vec![timestamp],
                input: Arc::new(input),
                fetch: Some(limit),
            }),
            Some(schema),
        );
    }
    (
        LogicalPlan::Sort(Sort {
            expr: vec![timestamp],
            input,
            fetch: Some(limit),
        }),
        None,
    )
}

fn generate_limit_and_sort_plan(
    input: Arc<LogicalPlan>,
    limit: usize,
    skip: usize,
) -> (LogicalPlan, Option<Arc<DFSchema>>) {
    let (sort, schema) = generate_sort_plan(input, limit + skip);
    (
        LogicalPlan::Limit(Limit {
            skip,
            fetch: Some(limit),
            input: Arc::new(sort),
        }),
        schema,
    )
}

struct ChangeTableScanSchema {}

impl ChangeTableScanSchema {
    fn new() -> Self {
        Self {}
    }
}

impl TreeNodeRewriter for ChangeTableScanSchema {
    type Node = LogicalPlan;

    fn f_up(&mut self, node: LogicalPlan) -> Result<Transformed<LogicalPlan>> {
        let mut transformed = match node {
            LogicalPlan::TableScan(scan) => {
                let schema = scan.source.schema();
                let timestamp_idx =
                    schema.index_of(get_config().common.column_timestamp.as_str())?;
                let mut projection = scan.projection.clone().unwrap();
                projection.push(timestamp_idx);
                let table_scan = TableScan::try_new(
                    scan.table_name,
                    scan.source,
                    Some(projection),
                    scan.filters,
                    scan.fetch,
                )?;
                Transformed::yes(LogicalPlan::TableScan(table_scan))
            }
            _ => Transformed::no(node),
        };
        transformed.tnr = TreeNodeRecursion::Stop;
        Ok(transformed)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{Int64Array, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::{
        arrow::record_batch::RecordBatch, assert_batches_eq, datasource::MemTable,
        prelude::SessionContext,
    };

    use super::AddSortAndLimitRule;

    #[tokio::test]
    async fn test_real_sql_for_timestamp() {
        let sqls = [
            (
                "select name from t order by _timestamp ASC",
                vec![
                    "+-------------+",
                    "| name        |",
                    "+-------------+",
                    "| openobserve |",
                    "| observe     |",
                    "+-------------+",
                ],
            ),
            (
                "select * from t",
                vec![
                    "+------------+------+",
                    "| _timestamp | name |",
                    "+------------+------+",
                    "| 5          | o2   |",
                    "| 4          | oo   |",
                    "+------------+------+",
                ],
            ),
            (
                "select * from t limit 3",
                vec![
                    "+------------+-------------+",
                    "| _timestamp | name        |",
                    "+------------+-------------+",
                    "| 5          | o2          |",
                    "| 4          | oo          |",
                    "| 3          | openobserve |",
                    "+------------+-------------+",
                ],
            ),
            (
                "select name from t limit 3",
                vec![
                    "+-------------+",
                    "| name        |",
                    "+-------------+",
                    "| o2          |",
                    "| oo          |",
                    "| openobserve |",
                    "+-------------+",
                ],
            ),
            (
                "select name from t where name = 'openobserve' limit 3",
                vec![
                    "+-------------+",
                    "| name        |",
                    "+-------------+",
                    "| openobserve |",
                    "| openobserve |",
                    "+-------------+",
                ],
            ),
            (
                "select * from t where _timestamp > 2 and name != 'oo'",
                vec![
                    "+------------+-------------+",
                    "| _timestamp | name        |",
                    "+------------+-------------+",
                    "| 5          | o2          |",
                    "| 3          | openobserve |",
                    "+------------+-------------+",
                ],
            ),
            (
                "select count(*) from t",
                vec![
                    "+----------+",
                    "| count(*) |",
                    "+----------+",
                    "| 5        |",
                    "+----------+",
                ],
            ),
            (
                "select name, count(*) as cnt from t group by name order by cnt desc, name desc",
                vec![
                    "+-------------+-----+",
                    "| name        | cnt |",
                    "+-------------+-----+",
                    "| openobserve | 2   |",
                    "| oo          | 1   |",
                    "+-------------+-----+",
                ],
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("_timestamp", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(StringArray::from(vec![
                    "openobserve",
                    "observe",
                    "openobserve",
                    "oo",
                    "o2",
                ])),
            ],
        )
        .unwrap();

        let ctx = SessionContext::new();
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();
        ctx.add_optimizer_rule(Arc::new(AddSortAndLimitRule::new(2, 0)));

        for item in sqls {
            let df = ctx.sql(item.0).await.unwrap();
            let data = df.collect().await.unwrap();
            assert_batches_eq!(item.1, &data);
        }
    }
}
