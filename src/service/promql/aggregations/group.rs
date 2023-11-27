// Copyright 2023 Zinc Labs Inc.
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

use crate::service::promql::value::{InstantValue, Sample, Value};
use datafusion::error::Result;
use promql_parser::parser::LabelModifier;
use rayon::prelude::*;

/// https://prometheus.io/docs/prometheus/latest/querying/operators/#aggregation-operators
pub fn group(timestamp: i64, param: &Option<LabelModifier>, data: &Value) -> Result<Value> {
    let score_values = super::eval_arithmetic(param, data, "group", |_total, _val| 1.0)?;
    if score_values.is_none() {
        return Ok(Value::None);
    }
    let values = score_values
        .unwrap()
        .par_iter()
        .map(|v| InstantValue {
            labels: v.1.labels.clone(),
            sample: Sample::new(timestamp, v.1.value),
        })
        .collect();
    Ok(Value::Vector(values))
}
