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

use std::any::Any;

use arrow::datatypes::{DataType, DataType::Timestamp, TimeUnit::Microsecond};
use datafusion::{
    common::Result,
    logical_expr::{ColumnarValue, ScalarUDF, ScalarUDFImpl, Signature, Volatility},
};
use once_cell::sync::Lazy;

pub const HISTOGRAM_UDF_NAME: &str = "histogram";

pub(crate) static HISTOGRAM_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(HistogramUdf::new()));

#[derive(Debug, Clone)]
struct HistogramUdf {
    signature: Signature,
}

impl HistogramUdf {
    fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for HistogramUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        HISTOGRAM_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(Timestamp(Microsecond, None))
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> Result<ColumnarValue> {
        unreachable!()
    }
}
