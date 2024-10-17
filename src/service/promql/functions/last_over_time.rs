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

pub(crate) fn last_over_time(data: &Value) -> Result<Value> {
    // NOTE: Comment taken from prometheus golang source.
    // The last_over_time function acts like offset; thus, it
    // should keep the metric name.  For all the other range
    // vector functions, the only change needed is to drop the
    // metric name in the output.
    // i.e. keep_name_label = true
    super::eval_idelta(data, "last_over_time", exec, true)
}

fn exec(data: &RangeValue) -> Option<f64> {
    if data.samples.is_empty() {
        return None;
    }
    Some(data.samples.last().unwrap().value)
}
