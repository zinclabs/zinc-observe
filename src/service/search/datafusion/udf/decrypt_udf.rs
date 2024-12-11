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

use std::sync::Arc;

use arrow::array::{ArrayRef, StringArray};
use config::utils::str;
use datafusion::{
    arrow::datatypes::DataType,
    common::cast::as_string_array,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarFunctionImplementation, ScalarUDF, Volatility},
    prelude::create_udf,
    scalar::ScalarValue,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// The name of the decrypt UDF given to DataFusion.
pub const DECRYPT_UDF_NAME: &str = "decrypt";

/// Dummy implementation of decrypt
pub(crate) static DECRYPT_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        DECRYPT_UDF_NAME,
        // expects two arguments : field and key_name
        vec![DataType::Utf8, DataType::Utf8],
        // returns string
        DataType::Utf8,
        // Volatile is needed, as it is needed for our optimizer to be used, check if Stable can be
        // used instead as volatile is much less performant
        Volatility::Stable,
        decrypt(),
    )
});

/// decrypt function
fn decrypt() -> ScalarFunctionImplementation {
    Arc::new(move |args: &[ColumnarValue]| {
        if args.len() != 2 {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "decrypt requires tow params : decrypt(field_name, key_name)".to_string(),
                ),
                None,
            ));
        }

        let key = match &args[1] {
            ColumnarValue::Scalar(ScalarValue::Utf8(Some(s))) => s.to_owned(),
            _ => {
                return Err(DataFusionError::SQL(
                    ParserError::ParserError(
                        "second argument to decrypt must be a key-name string".to_string(),
                    ),
                    None,
                ))
            }
        };

        let args = ColumnarValue::values_to_arrays(args)?;

        let values = as_string_array(&args[0]).map_err(|_| {
            DataFusionError::SQL(
                ParserError::ParserError(
                    "first argument to decrypt must be a string type column".to_string(),
                ),
                None,
            )
        })?;

        // TODO: here we will eventually do the decryption with key
        let ret = values.iter().map(|v| v).collect::<StringArray>();

        Ok(ColumnarValue::from(Arc::new(ret) as ArrayRef))
    })
}