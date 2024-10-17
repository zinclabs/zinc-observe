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

use datafusion::error::{DataFusionError, Result};

use crate::service::promql::value::{InstantValue, Labels, Sample, Value};

pub(crate) fn vector(data: &Value, eval_ts: i64) -> Result<Value> {
    let value = match data {
        Value::Float(f) => *f,
        _ => {
            return Err(DataFusionError::NotImplemented(
                "Unexpected input. Expected: \"vector(s scalar)\"".into(),
            ));
        }
    };

    let instant = InstantValue {
        labels: Labels::default(),
        sample: Sample {
            timestamp: eval_ts,
            value,
        },
    };
    Ok(Value::Vector(vec![instant]))
}
