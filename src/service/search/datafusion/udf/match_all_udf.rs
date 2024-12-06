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

use arrow::datatypes::DataType;
use datafusion::{
    common::{DataFusionError, Result},
    logical_expr::{ColumnarValue, ScalarUDF, ScalarUDFImpl, Signature, Volatility},
};
use once_cell::sync::Lazy;

pub const MATCH_ALL_UDF_NAME: &str = "match_all";
pub const MATCH_ALL_RAW_UDF_NAME: &str = "match_all_raw";
pub const MATCH_ALL_RAW_IGNORE_CASE_UDF_NAME: &str = "match_all_raw_ignore_case";
pub const FUZZY_MATCH_ALL_UDF_NAME: &str = "fuzzy_match_all";

pub(crate) static MATCH_ALL_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(MatchAllUdf::new()));

pub(crate) static MATCH_ALL_RAW_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(MatchAllRawUdf::new()));

pub(crate) static MATCH_ALL_RAW_IGNORE_CASE_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(MatchAllRawIgnoreCaseUdf::new()));

pub(crate) static FUZZY_MATCH_ALL_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(FuzzyMatchAllUdf::new()));

#[derive(Debug, Clone)]
struct MatchAllUdf {
    signature: Signature,
}

impl MatchAllUdf {
    fn new() -> Self {
        Self {
            signature: Signature::exact(vec![DataType::Utf8], Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for MatchAllUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        MATCH_ALL_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> Result<ColumnarValue> {
        Err(DataFusionError::Internal(
            "match_all function don't support sql with multiple streams".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
struct MatchAllRawUdf {
    signature: Signature,
}

impl MatchAllRawUdf {
    fn new() -> Self {
        Self {
            signature: Signature::exact(vec![DataType::Utf8], Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for MatchAllRawUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        MATCH_ALL_RAW_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> Result<ColumnarValue> {
        Err(DataFusionError::Internal(
            "match_all_raw function don't support sql with multiple streams".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
struct MatchAllRawIgnoreCaseUdf {
    signature: Signature,
}

impl MatchAllRawIgnoreCaseUdf {
    fn new() -> Self {
        Self {
            signature: Signature::exact(vec![DataType::Utf8], Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for MatchAllRawIgnoreCaseUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        MATCH_ALL_RAW_IGNORE_CASE_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> Result<ColumnarValue> {
        Err(DataFusionError::Internal(
            "match_all_raw_ignore_case function don't support sql with multiple streams"
                .to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
struct FuzzyMatchAllUdf {
    signature: Signature,
}

impl FuzzyMatchAllUdf {
    fn new() -> Self {
        Self {
            signature: Signature::exact(
                vec![DataType::Utf8, DataType::Int64],
                Volatility::Immutable,
            ),
        }
    }
}

impl ScalarUDFImpl for FuzzyMatchAllUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        FUZZY_MATCH_ALL_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> Result<ColumnarValue> {
        Err(DataFusionError::Internal(
            "fuzzy_match_all function don't support sql with multiple streams".to_string(),
        ))
    }
}
