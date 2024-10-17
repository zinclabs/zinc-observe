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
    common::std_variance,
    value::{RangeValue, Value},
};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#stdvar_over_time
pub(crate) fn stdvar_over_time(data: &Value) -> Result<Value> {
    super::eval_idelta(data, "stdvar_over_time", exec, false)
}

fn exec(data: &RangeValue) -> Option<f64> {
    if data.samples.is_empty() {
        return None;
    }
    let samples: Vec<f64> = data.samples.iter().map(|s| s.value).collect();
    std_variance(&samples)
}
