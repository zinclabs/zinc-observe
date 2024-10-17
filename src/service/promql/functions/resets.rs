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

use crate::service::promql::value::{RangeValue, Value};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#resets
pub(crate) fn resets(data: &Value) -> Result<Value> {
    super::eval_idelta(data, "resets", exec, false)
}

fn exec(data: &RangeValue) -> Option<f64> {
    let resets = data
        .samples
        .iter()
        .zip(data.samples.iter().skip(1))
        .map(|(current, next)| (current.value.gt(&next.value) as u32) as f64)
        .sum();
    Some(resets)
}
