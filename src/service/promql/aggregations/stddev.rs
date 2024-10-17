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
use rayon::prelude::*;

use crate::service::promql::{
    common::std_deviation2,
    value::{InstantValue, Sample, Value},
};

pub fn stddev(timestamp: i64, param: &Option<LabelModifier>, data: &Value) -> Result<Value> {
    let score_values = super::eval_std_dev_var(param, data, "stddev")?;
    if score_values.is_none() {
        return Ok(Value::None);
    }
    let values = score_values
        .unwrap()
        .par_iter()
        .map(|it| {
            let std_var =
                std_deviation2(&it.1.values, it.1.current_mean, it.1.current_count).unwrap();
            InstantValue {
                labels: it.1.labels.clone(),
                sample: Sample::new(timestamp, std_var),
            }
        })
        .collect();
    Ok(Value::Vector(values))
}
