// Copyright 2025 OpenObserve Inc.
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

use config::meta::sql::OrderBy;
use datafusion::{
    common::{
        tree_node::{Transformed, TreeNode, TreeNodeRecursion},
        Result,
    },
    logical_expr::LogicalPlan,
    optimizer::{optimizer::ApplyOrder, OptimizerConfig, OptimizerRule},
};

use super::utils::AddSortAndLimit;

#[derive(Debug)]
pub struct AddSortAndLimitRule {
    limit: usize,
    offset: usize,
    order_by: Option<(String, OrderBy)>,
}

impl AddSortAndLimitRule {
    pub fn new(limit: usize, offset: usize, order_by: Option<(String, OrderBy)>) -> Self {
        Self {
            limit,
            offset,
            order_by,
        }
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
        let mut plan = plan.rewrite(&mut AddSortAndLimit::new(
            self.limit,
            self.offset,
            self.order_by.clone(),
        ))?;
        plan.tnr = TreeNodeRecursion::Stop;
        Ok(plan)
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
        ctx.add_optimizer_rule(Arc::new(AddSortAndLimitRule::new(2, 0, None)));

        for item in sqls {
            let df = ctx.sql(item.0).await.unwrap();
            let data = df.collect().await.unwrap();
            assert_batches_eq!(item.1, &data);
        }
    }
}
