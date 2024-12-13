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

use datafusion::{
    common::{
        tree_node::{Transformed, TreeNode, TreeNodeRewriter},
        Result,
    },
    error::DataFusionError,
    logical_expr::{expr::ScalarFunction, BinaryExpr, Expr, LogicalPlan, Operator},
    optimizer::{optimizer::ApplyOrder, utils::NamePreserver, OptimizerConfig, OptimizerRule},
    scalar::ScalarValue,
};

use crate::service::search::datafusion::udf::cipher_udf::{DECRYPT_UDF_NAME, ENCRYPT_UDF};

/// Optimization rule that rewrite decrypt if applicable
/// This will convert
/// - decrypt(field,key_name) = value -> field = encrypt(value,key_name)
/// - decrypt(field,key_name) != value -> field != encrypt(value,key_name)
///
/// The value must be static string, as otherwise there is no benefit to
/// converting from decrypt->encrypt, both will have same const.
///
/// When the value is static string, it is quite beneficial to do it this way,
/// as datafusion will call encrypt at time of physical plan construction and
/// basically inline the encrypted value in the plan, making the comparison
/// a standard string comparison, instead of calling decrypt on each value and
/// then doing a string comparison on it.
///
/// This is assuming both have same cost. If we later find they don't, we'll
/// have to deal with that separately.
#[derive(Default, Debug)]
pub struct RewriteDecrypt {}

impl RewriteDecrypt {
    pub fn new() -> Self {
        Self {}
    }
}

impl OptimizerRule for RewriteDecrypt {
    fn name(&self) -> &str {
        "rewrite_decrypt"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::BottomUp)
    }

    fn supports_rewrite(&self) -> bool {
        true
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        if plan
            .expressions()
            .iter()
            .map(|expr| expr.exists(|expr| Ok(is_rewritable_decrypt(expr))).unwrap())
            .any(|x| x)
        {
            let mut expr_rewriter = DecryptToEncrypt::new();

            let name_preserver = NamePreserver::new(&plan);
            plan.map_expressions(|expr| {
                let original_name = name_preserver.save(&expr);
                expr.rewrite(&mut expr_rewriter)
                    .map(|transformed| transformed.update_data(|e| original_name.restore(e)))
            })
        } else {
            Ok(Transformed::no(plan))
        }
    }
}

fn is_rewritable_decrypt(expr: &Expr) -> bool {
    if let Expr::BinaryExpr(BinaryExpr {
        left,
        op: Operator::Eq | Operator::NotEq,
        right,
    }) = expr
    {
        // we are specifically looking for binary op, where the left side
        // is decrypt invocation and right side  is a constant string

        // check left side is decrypt call
        let left_is_decrypt = match left.as_ref() {
            Expr::ScalarFunction(ScalarFunction { func, .. }) => func.name() == DECRYPT_UDF_NAME,
            _ => false,
        };
        // check right side is const string
        let right_is_literal = matches!(right.as_ref(), Expr::Literal(ScalarValue::Utf8(_)));
        left_is_decrypt && right_is_literal
    } else {
        false
    }
}

// Rewriter for decrypt
#[derive(Debug, Clone)]
pub struct DecryptToEncrypt {}

impl DecryptToEncrypt {
    pub fn new() -> Self {
        Self {}
    }
}
impl TreeNodeRewriter for DecryptToEncrypt {
    type Node = Expr;

    fn f_up(&mut self, expr: Expr) -> Result<Transformed<Expr>, DataFusionError> {
        // here we expect the expr to be is_rewritable_decrypt, and return at any point
        // where it is not. we can do a is_rewritable_decrypt call fist and early return,
        // but then we will have to do the following anyways to extract the values,
        // so we do it this way.

        // start with extracting individual sides of binary op
        let (left, op, right) = match &expr {
            Expr::BinaryExpr(BinaryExpr { left, op, right })
                if matches!(op, Operator::Eq | Operator::NotEq) =>
            {
                (left, op, right)
            }
            _ => return Ok(Transformed::no(expr)),
        };

        // extract args from the decrypt call, so we get the table col and key name
        let decrypt_args = match left.as_ref() {
            Expr::ScalarFunction(ScalarFunction { func, args }) => {
                if func.name() != DECRYPT_UDF_NAME {
                    return Ok(Transformed::no(expr));
                }
                args
            }
            _ => return Ok(Transformed::no(expr)),
        };

        // then get the value we are comparing it to
        let compared_const = match right.as_ref() {
            Expr::Literal(ScalarValue::Utf8(_)) => right.as_ref().to_owned(),
            _ => return Ok(Transformed::no(expr)),
        };

        // sanity check
        if decrypt_args.len() != 2 {
            return Err(DataFusionError::Internal(
                "decrypt function requires 2 arguments".to_string(),
            ));
        }

        let col_expr = decrypt_args[0].clone();
        let key_name = decrypt_args[1].clone();

        // construct the encrypt call over the const value
        let encrypt_call = Expr::ScalarFunction(ScalarFunction {
            func: Arc::new(ENCRYPT_UDF.clone()),
            args: vec![compared_const, key_name],
        });

        // construct the new binary op, where we compare col to the encrypted value
        // instead of decrypting col and comparing it to plain value
        let binary_op = BinaryExpr {
            left: Box::new(col_expr),
            op: *op,
            right: Box::new(encrypt_call),
        };

        Ok(Transformed::yes(Expr::BinaryExpr(binary_op)))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{Int64Array, StringArray};
    use arrow_schema::DataType;
    use datafusion::{
        arrow::{
            datatypes::{Field, Schema},
            record_batch::RecordBatch,
        },
        datasource::MemTable,
        logical_expr::LogicalPlan,
        optimizer::{Optimizer, OptimizerContext, OptimizerRule},
        prelude::SessionContext,
    };

    use super::RewriteDecrypt;
    use crate::service::search::datafusion::udf::{
        cipher_udf::{DECRYPT_UDF, ENCRYPT_UDF},
        match_all_udf::MATCH_ALL_UDF,
    };

    #[tokio::test]
    async fn test_rewrite_decrypt() {
        let sqls = [
            // equal to operator gets re-written
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') = 'test_val'",
                "Projection: t._timestamp\n  Filter: t.name = encrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            // not equal operator gets rewritten
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') != 'test_val'",
                "Projection: t._timestamp\n  Filter: t.name != encrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            // like operator does not get rewritten
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') LIKE '%test%'",
                "Projection: t._timestamp\n  Filter: decrypt(t.name, Utf8(\"test_key\")) LIKE Utf8(\"%test%\")\n    TableScan: t"
            ),
            // match all does not get re-written
            (
                "SELECT _timestamp FROM t where match_all(decrypt(name, 'test_key'), 'test')",
                "Projection: t._timestamp\n  Filter: match_all(decrypt(t.name, Utf8(\"test_key\")), Utf8(\"test\"))\n    TableScan: t"
            ),
            // if comparison is to non-literal, it does not get re-written
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') = other_col",
                "Projection: t._timestamp\n  Filter: decrypt(t.name, Utf8(\"test_key\")) = t.other_col\n    TableScan: t"
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("_timestamp", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("other_col", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(StringArray::from(vec![
                    "open",
                    "observe",
                    "openobserve",
                    "o2",
                    "oo",
                ])),
                Arc::new(StringArray::from(vec![
                    "open",
                    "observe",
                    "openobserve",
                    "o2",
                    "oo",
                ])),
            ],
        )
        .unwrap();

        fn observe(_plan: &LogicalPlan, _rule: &dyn OptimizerRule) {}

        let ctx = SessionContext::new();
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();
        ctx.register_udf(DECRYPT_UDF.clone());
        ctx.register_udf(ENCRYPT_UDF.clone());
        ctx.register_udf(MATCH_ALL_UDF.clone());
        ctx.add_optimizer_rule(Arc::new(RewriteDecrypt::new()));

        for (sql, res) in sqls {
            let plan = ctx.state().create_logical_plan(sql).await.unwrap();
            let optimizer = Optimizer::with_rules(vec![Arc::new(RewriteDecrypt::new())]);
            let optimized_plan = optimizer
                .optimize(plan, &OptimizerContext::new(), observe)
                .unwrap();
            let formatted_plan = format!("{optimized_plan}");
            assert_eq!(res, formatted_plan);
        }
    }
}
