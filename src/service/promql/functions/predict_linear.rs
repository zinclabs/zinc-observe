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

use datafusion::error::{DataFusionError, Result};

use crate::service::promql::{
    common::linear_regression,
    value::{InstantValue, Sample, Value},
};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#predict_linear
pub(crate) fn predict_linear(data: Value, duration: f64) -> Result<Value> {
    exec(data, duration)
}

fn exec(data: Value, duration: f64) -> Result<Value> {
    let data = match data {
        Value::Matrix(v) => v,
        Value::None => return Ok(Value::None),
        v => {
            return Err(DataFusionError::Plan(format!(
                "predict_linear: matrix argument expected, got {}",
                v.get_type()
            )));
        }
    };

    let mut rate_values = Vec::with_capacity(data.len());
    for mut metric in data {
        let labels = std::mem::take(&mut metric.labels);
        let eval_ts = metric.time_window.as_ref().unwrap().eval_ts;
        if let Some((slope, intercept)) = linear_regression(&metric.samples, eval_ts / 1000) {
            let value = slope * duration + intercept;
            rate_values.push(InstantValue {
                labels,
                sample: Sample::new(eval_ts, value),
            });
        }
    }
    Ok(Value::Vector(rate_values))
}
