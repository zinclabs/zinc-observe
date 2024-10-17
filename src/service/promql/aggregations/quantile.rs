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

use datafusion::error::{DataFusionError, Result};
use promql_parser::parser::Expr as PromExpr;
use rayon::prelude::*;

use crate::service::promql::{
    aggregations::prepare_vector, common::quantile as calculate_quantile, value::Value, Engine,
};

pub async fn quantile(
    ctx: &mut Engine,
    timestamp: i64,
    param: Box<PromExpr>,
    data: &Value,
) -> Result<Value> {
    let param = ctx.exec_expr(&param).await?;
    let qtile = match param {
        Value::Float(v) => v,
        _ => {
            return Err(DataFusionError::Plan(
                "[quantile] param must be a NumberLiteral".to_string(),
            ));
        }
    };
    let data = match data {
        Value::Vector(v) => v,
        Value::None => return Ok(Value::None),
        _ => {
            return Err(DataFusionError::Plan(
                "[quantile] function only accept vector values".to_string(),
            ));
        }
    };

    if qtile < 0 as f64 || qtile > 1_f64 || qtile.is_nan() {
        let value = match qtile.signum() as i32 {
            1 => f64::INFINITY,
            -1 => f64::NEG_INFINITY,
            _ => f64::NAN,
        };
        return prepare_vector(timestamp, value);
    }

    if data.is_empty() {
        return prepare_vector(timestamp, f64::NAN);
    }

    let values: Vec<f64> = data.par_iter().map(|item| item.sample.value).collect();

    let quantile_value = calculate_quantile(&values, qtile).unwrap();
    prepare_vector(timestamp, quantile_value)
}
