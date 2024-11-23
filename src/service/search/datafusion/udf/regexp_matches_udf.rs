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

use std::{any::Any, sync::Arc};

use arrow::array::{Array, ArrayRef, GenericStringBuilder, ListBuilder, OffsetSizeTrait};
use arrow_schema::Field;
use datafusion::{
    arrow::datatypes::DataType::{self, *},
    common::{cast::as_generic_string_array, exec_err, ScalarValue},
    error::{DataFusionError, Result},
    logical_expr::{ColumnarValue, ScalarUDF, ScalarUDFImpl, Signature, TypeSignature, Volatility},
};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::service::search::datafusion::udf::REGEX_MATCHES_UDF_NAME;

/// Implementation of regexp_matches
pub(crate) static REGEX_MATCHES_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(RegexpMatchesFunc::default()));

/// # `re_matches` User-Defined Function (UDF)
///
/// This UDF extracts all substrings from a string column that match a given regular expression
/// pattern. It is designed to work with the DataFusion query engine and supports both scalar and
/// array inputs.
///
/// ## Purpose
///
/// The `re_matches` UDF allows users to extract all matches of a regular expression from a string
/// column.
///
/// ## Function Signature
///
/// The UDF supports the following signatures:
///
/// - `re_matches(string_column, regex_pattern)`
///
/// - `string_column`: The input column containing strings to search.
/// - `regex_pattern`: The regular expression pattern to match.
///
/// ## Return Type
///
/// - The function returns a `List` of strings for each input row. If no matches are found, an empty
///   list is returned.
/// - If the input string or pattern is null, the function returns null for that row.
#[derive(Debug)]
pub struct RegexpMatchesFunc {
    signature: Signature,
}

impl Default for RegexpMatchesFunc {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexpMatchesFunc {
    pub fn new() -> Self {
        Self {
            signature: Signature::one_of(
                vec![
                    TypeSignature::Exact(vec![Utf8, Utf8]),
                    TypeSignature::Exact(vec![LargeUtf8, LargeUtf8]),
                    TypeSignature::Exact(vec![Utf8, Utf8, Utf8]),
                    TypeSignature::Exact(vec![LargeUtf8, LargeUtf8, LargeUtf8]),
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl ScalarUDFImpl for RegexpMatchesFunc {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        REGEX_MATCHES_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> datafusion::common::Result<DataType> {
        Ok(match &arg_types[0] {
            Null => Null,
            other => List(Arc::new(Field::new("item", other.clone(), true))),
        })
    }

    fn invoke(&self, args: &[ColumnarValue]) -> datafusion::common::Result<ColumnarValue> {
        let len = args
            .iter()
            .fold(Option::<usize>::None, |acc, arg| match arg {
                ColumnarValue::Scalar(_) => acc,
                ColumnarValue::Array(a) => Some(a.len()),
            });

        let is_scalar = len.is_none();
        let inferred_length = len.unwrap_or(1);
        let args = args
            .iter()
            .map(|arg| arg.clone().into_array(inferred_length))
            .collect::<Result<Vec<_>>>()?;

        let result = regexp_matches_func(&args)?;
        if is_scalar {
            let scaler = ScalarValue::try_from_array(&result, 0)?;
            Ok(ColumnarValue::Scalar(scaler))
        } else {
            Ok(ColumnarValue::Array(result))
        }
    }
}

fn regexp_matches_func(args: &[ArrayRef]) -> Result<ArrayRef> {
    match args[0].data_type() {
        Utf8 => regexp_matches::<i32>(args),
        LargeUtf8 => regexp_matches::<i64>(args),
        other => {
            exec_err!("unsupported data type {other:?} for function regexp_matches")
        }
    }
}

/// Extracts all matches of a regular expression from a string column.
/// The function supports both scalar and array inputs.
pub fn regexp_matches<T: OffsetSizeTrait>(args: &[ArrayRef]) -> Result<ArrayRef> {
    let values = as_generic_string_array::<T>(&args[0])?;
    let regex = as_generic_string_array::<T>(&args[1])?;

    // Scalar is a single regex pattern
    // Check if the regex pattern is a scalar
    let is_scalar_pattern = regex.len() == 1;

    // Precompile the regex if it's scalar
    let scalar_regex = if is_scalar_pattern {
        Some(
            Regex::new(regex.value(0))
                .map_err(|e| DataFusionError::Execution(format!("Invalid regex pattern: {}", e)))?,
        )
    } else {
        None
    };

    let mut list_builder = ListBuilder::new(GenericStringBuilder::<T>::new());

    for i in 0..values.len() {
        if values.is_null(i) || (!is_scalar_pattern && regex.is_null(i)) {
            // Append NULL for this row
            list_builder.append(false);
            continue;
        }

        let value = values.value(i);

        let re = if is_scalar_pattern {
            scalar_regex.clone().unwrap() // Use precompiled regex
        } else {
            Regex::new(regex.value(i)) // Compile regex for this row
                .map_err(|e| {
                DataFusionError::Execution(format!("Invalid regex pattern: {}", e))
            })?
        };

        // Extract matches
        let mut has_match = false;
        for mat in re.find_iter(value) {
            list_builder.values().append_value(mat.as_str());
            has_match = true;
        }

        if has_match {
            // If matches were found, finalize the list for this row
            list_builder.append(true);
        } else {
            // If no matches were found, append NULL
            list_builder.append(false);
        }
    }

    Ok(Arc::new(list_builder.finish()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{RecordBatch, StringArray};
    use arrow_schema::Schema;
    use datafusion::{assert_batches_eq, datasource::MemTable, prelude::SessionContext};

    use super::*;

    #[tokio::test]
    async fn test_re_matches_extract_all_numbers_as_scalar() {
        let sql = "SELECT re_matches(log, '(\\d+)') AS matches FROM t";

        // Define schema
        let schema = Arc::new(Schema::new(vec![Field::new("log", DataType::Utf8, false)]));

        // Define data
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(StringArray::from(vec![
                "abc123def456",
                "no match here",
                "789ghi123",
            ]))],
        )
        .unwrap();

        // Create a session context
        let ctx = SessionContext::new();
        ctx.register_udf(REGEX_MATCHES_UDF.clone());

        // Register the table
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        // Execute SQL
        let df = ctx.sql(sql).await.unwrap();
        let results = df.collect().await.unwrap();

        // Expected output
        let expected = vec![
            "+------------+",
            "| matches    |",
            "+------------+",
            "| [123, 456] |",
            "|            |",
            "| [789, 123] |",
            "+------------+",
        ];

        assert_batches_eq!(expected, &results);
    }

    #[tokio::test]
    async fn test_re_matches_regex_as_array() {
        let sql = "SELECT re_matches(log, regex) AS matches FROM t";

        // Define schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, true), // Allow NULLs in `log`
            Field::new("regex", DataType::Utf8, true), // Allow NULLs in `regex`
        ]));

        // Define data
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec![
                    Some("abc123def456"),  // Normal case: numbers in the string
                    Some("no match here"), // No match case
                    Some("789ghi123"),     // Multiple matches
                    None,                  // NULL log
                    Some(""),              // Empty string
                ])),
                Arc::new(StringArray::from(vec![
                    Some("\\d+"),     // Match any digits
                    Some("no match"), // Match literal "no match"
                    Some("ghi"),      // Match substring "ghi"
                    Some("\\w+"),     // Match any word (NULL log should return NULL)
                    None,             // NULL regex
                ])),
            ],
        )
        .unwrap();

        // Create a session context
        let ctx = SessionContext::new();
        ctx.register_udf(REGEX_MATCHES_UDF.clone());

        // Register the table
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        // Execute SQL
        let df = ctx.sql(sql).await.unwrap();
        let results = df.collect().await.unwrap();

        // Expected output
        let expected = vec![
            "+------------+",
            "| matches    |",
            "+------------+",
            "| [123, 456] |", // Matches "\\d+" for "abc123def456"
            "| [no match] |", // Matches "no match" for "no match here"
            "| [ghi]      |", // Matches "ghi" for "789ghi123"
            "|            |", // NULL log should return NULL
            "|            |", // Empty string, no match
            "+------------+",
        ];

        assert_batches_eq!(expected, &results);
    }
}
