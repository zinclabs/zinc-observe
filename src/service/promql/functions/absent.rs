// Copyright 2023 Zinc Labs Inc.
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

use datafusion::error::{DataFusionError, Result};

use crate::service::promql::value::{InstantValue, Labels, Sample, Value};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#absent
pub(crate) fn absent(data: &Value, eval_ts: i64) -> Result<Value> {
    let _data = match data {
        Value::Vector(v) => v,
        Value::None => {
            let rate_values = vec![InstantValue {
                labels: Labels::default(),
                sample: Sample::new(eval_ts, 1.0),
            }];
            return Ok(Value::Vector(rate_values));
        }
        _ => {
            return Err(DataFusionError::Plan(
                "Unexpected input for absent func".into(),
            ))
        }
    };
    Ok(Value::None)
}
