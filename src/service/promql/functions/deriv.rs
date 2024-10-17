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

use crate::service::promql::{
    common::linear_regression,
    value::{RangeValue, Value},
};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#deriv
pub(crate) fn deriv(data: &Value) -> Result<Value> {
    super::eval_idelta(data, "deriv", exec, false)
}

fn exec(data: &RangeValue) -> Option<f64> {
    if data.samples.len() < 2 {
        return None;
    }
    // https://github.com/prometheus/prometheus/issues/2674
    let value = linear_regression(&data.samples, data.samples[0].timestamp / 1000);
    match value {
        Some((slope, _)) => Some(slope),
        _ => None,
    }
}
