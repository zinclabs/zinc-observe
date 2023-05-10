// Copyright 2022 Zinc Labs Inc. and Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use datafusion::error::Result;

use crate::service::promql::value::{RangeValue, Value};

pub(crate) fn delta(data: &Value) -> Result<Value> {
    super::eval_idelta(data, "delta", exec)
}

fn exec(range: &RangeValue) -> Option<f64> {
    range
        .extrapolate()
        .map_or(None, |(first, last)| Some(last.value - first.value))
}
