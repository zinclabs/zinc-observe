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

use config::utils::str;
use datafusion::{
    arrow::datatypes::DataType,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarFunctionImplementation, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

// NOTE: This is not the actual implementation fo decrypt, because
// decrypt needs async function, and udf do not support async functions.
// Hence a method shown https://github.com/apache/datafusion/pull/6713/commits/127cc9313882aa321da8cd175fddc70eaec94e23
// is used as guidance.

// Thus this is only the "dummy" implementation, and actual handler is in the optimizer
// The way it works is that we register a dummy UDF here so datafusion doesn't complain
//  about the function not existing while parsing sql,
// and then in optimizer passes we replcae that call with our custom physical executor
// which actually does the work.

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
        Volatility::Volatile,
        decrypt_placeholder(),
    )
});

/// decrypt function placeholder for datafusion
fn decrypt_placeholder() -> ScalarFunctionImplementation {
    Arc::new(move |args: &[ColumnarValue]| {
        if args.len() != 2 {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "decrypt requires tow params : decrypt(field_name, key_name)".to_string(),
                ),
                None,
            ));
        }
        Err(DataFusionError::NotImplemented(
            "BUG: decrypt should have optimized away and never reached here".to_string(),
        ))
    })
}
