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
use strum::EnumIter;

use crate::service::promql::value::{InstantValue, LabelsExt, Sample, Value};

#[derive(Debug, EnumIter)]
pub enum MathOperationsType {
    Abs,
    Ceil,
    Exp,
    Floor,
    Ln,
    Log10,
    Log2,
    Round,
    Sgn,
    Sqrt,
}

impl MathOperationsType {
    /// Apply a given simple match function to a float type
    pub fn apply(&self, input: f64) -> f64 {
        match self {
            Self::Abs => input.abs(),
            Self::Ceil => input.ceil(),
            Self::Exp => input.exp(),
            Self::Floor => input.floor(),
            Self::Ln => input.ln(),
            Self::Log2 => input.log2(),
            Self::Log10 => input.log10(),
            Self::Sgn => input.signum(),
            Self::Sqrt => input.sqrt(),
            Self::Round => input.round(),
        }
    }
}

pub(crate) fn abs(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Abs)
}

pub(crate) fn ceil(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Ceil)
}

pub(crate) fn floor(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Floor)
}

pub(crate) fn exp(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Exp)
}

pub(crate) fn ln(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Ln)
}

pub(crate) fn log2(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Log2)
}

pub(crate) fn log10(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Log10)
}

pub(crate) fn sqrt(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Sqrt)
}

pub(crate) fn round(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Round)
}

pub(crate) fn sgn(data: &Value) -> Result<Value> {
    exec(data, &MathOperationsType::Sgn)
}

fn exec(data: &Value, op: &MathOperationsType) -> Result<Value> {
    match &data {
        Value::Vector(v) => {
            let out = v
                .iter()
                .map(|instant| {
                    let val = op.apply(instant.sample.value);
                    InstantValue {
                        labels: instant.labels.without_metric_name(),
                        sample: Sample::new(instant.sample.timestamp, val),
                    }
                })
                .collect();
            Ok(Value::Vector(out))
        }
        Value::None => Ok(Value::None),
        _ => Err(DataFusionError::NotImplemented(format!(
            "Invalid input for minute value: {:?}",
            data
        ))),
    }
}
