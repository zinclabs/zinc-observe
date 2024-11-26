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

use datafusion::error::Result;
use promql_parser::parser::LabelModifier;

use crate::service::promql::{aggregations::score_to_instant_value, value::Value};

pub fn min(timestamp: i64, param: &Option<LabelModifier>, data: &Value) -> Result<Value> {
    let score_values = super::eval_arithmetic(param, data, "min", |prev, val| {
        if prev > 0.0 && prev <= val {
            prev
        } else {
            val
        }
    })?;
    if score_values.is_none() {
        return Ok(Value::None);
    }
    Ok(Value::Vector(score_to_instant_value(
        timestamp,
        score_values,
    )))
}
