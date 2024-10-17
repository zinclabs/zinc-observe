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
use promql_parser::parser::{Expr as PromExpr, LabelModifier};

use crate::service::promql::{
    value::{InstantValue, Label, Sample, Value},
    Engine,
};

pub async fn count_values(
    ctx: &mut Engine,
    timestamp: i64,
    param: Box<PromExpr>,
    modifier: &Option<LabelModifier>,
    data: &Value,
) -> Result<Value> {
    let param = ctx.exec_expr(&param).await?;
    let label_name = match param {
        Value::String(v) => v,
        _ => {
            return Err(DataFusionError::Plan(
                "[label_name] param must be a String".to_string(),
            ));
        }
    };
    if !Label::is_valid_label_name(&label_name) {
        return Err(DataFusionError::Plan(
            "[label_name] param invalid. Check https://prometheus.io/docs/concepts/data_model/#metric-names-and-labels".to_string(),
        ));
    }
    let score_values = super::eval_count_values(modifier, data, "count_values", &label_name)?;
    if score_values.is_none() {
        return Ok(Value::None);
    }

    let values = score_values
        .unwrap()
        .values()
        .map(|it| InstantValue {
            labels: it.labels.clone(),
            sample: Sample::new(timestamp, it.count as f64),
        })
        .collect();

    Ok(Value::Vector(values))
}
