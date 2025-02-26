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

use datafusion::{
    arrow::{
        array::{ArrayRef, BooleanArray},
        datatypes::DataType,
    },
    common::cast::as_string_array,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarFunctionImplementation, ScalarUDF, Volatility},
    prelude::create_udf,
    scalar::ScalarValue,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// Implementation of str_match
pub(crate) static STR_MATCH_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        super::STR_MATCH_UDF_NAME,
        // expects two string
        vec![DataType::Utf8, DataType::Utf8],
        // returns boolean
        DataType::Boolean,
        Volatility::Stable,
        str_match_expr_impl(false),
    )
});

/// Implementation of str_match_ignore_case
pub(crate) static STR_MATCH_IGNORE_CASE_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        super::STR_MATCH_UDF_IGNORE_CASE_NAME,
        // expects two string
        vec![DataType::Utf8, DataType::Utf8],
        // returns boolean
        DataType::Boolean,
        Volatility::Stable,
        str_match_expr_impl(true),
    )
});

/// str_match function for datafusion
pub fn str_match_expr_impl(case_insensitive: bool) -> ScalarFunctionImplementation {
    Arc::new(move |args: &[ColumnarValue]| {
        if args.len() != 2 {
            return Err(DataFusionError::SQL(
                ParserError::ParserError("str_match UDF expects two string".to_string()),
                None,
            ));
        }

        // 1. cast both arguments to be aligned with the signature
        let ColumnarValue::Array(haystack) = &args[0] else {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "Invalid argument types[haystack] to str_match function".to_string(),
                ),
                None,
            ));
        };
        let haystack = as_string_array(&haystack)?;
        let ColumnarValue::Scalar(needle) = &args[1] else {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "Invalid argument types[needle] to str_match function".to_string(),
                ),
                None,
            ));
        };
        let needle = match needle {
            ScalarValue::Utf8(v) => v,
            ScalarValue::Utf8View(v) => v,
            ScalarValue::LargeUtf8(v) => v,
            _ => {
                return Err(DataFusionError::SQL(
                    ParserError::ParserError(
                        "Invalid argument types[needle] to str_match function".to_string(),
                    ),
                    None,
                ))
            }
        };
        if needle.is_none() || needle.as_ref().unwrap().is_empty() {
            return Err(DataFusionError::SQL(
                ParserError::ParserError(
                    "Invalid argument types[needle] to str_match function".to_string(),
                ),
                None,
            ));
        }
        // pre-compute the needle
        let needle = if case_insensitive {
            needle.as_ref().unwrap().to_lowercase()
        } else {
            needle.as_ref().unwrap().to_string()
        };
        let mem_finder = memchr::memmem::Finder::new(needle.as_bytes());

        // 2. perform the computation
        let array = haystack
            .iter()
            .map(|haystack| {
                match haystack {
                    // in arrow, any value can be null.
                    // Here we decide to make our UDF to return null when haystack is null.
                    Some(haystack) => match case_insensitive {
                        true => Some(
                            mem_finder
                                .find(haystack.to_lowercase().as_bytes())
                                .is_some(),
                        ),
                        false => Some(mem_finder.find(haystack.as_bytes()).is_some()),
                    },
                    _ => None,
                }
            })
            .collect::<BooleanArray>();

        // `Ok` because no error occurred during the calculation
        // `Arc` because arrays are immutable, thread-safe, trait objects.
        Ok(ColumnarValue::from(Arc::new(array) as ArrayRef))
    })
}

#[cfg(test)]
mod tests {
    use arrow::array::StringArray;
    use datafusion::{
        arrow::{
            array::Int64Array,
            datatypes::{Field, Schema},
            record_batch::RecordBatch,
        },
        datasource::MemTable,
        prelude::SessionContext,
    };

    use super::*;

    #[tokio::test]
    async fn test_str_match_udf() {
        let sql = vec![
            "select * from t where str_match(log, 'es') and str_match_ignore_case(city, 'be')",
            "select * from t where str_match(log, 'es') and str_match_ignore_case(city, 'BE')",
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("id", DataType::Int64, false),
            Field::new("city", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["this", "is", "a", "test"])),
                Arc::new(Int64Array::from(vec![1, 2, 3, 4])),
                Arc::new(StringArray::from(vec![
                    "New York",
                    "Pune",
                    "San Francisco",
                    "Beijing",
                ])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(STR_MATCH_UDF.clone());
        ctx.register_udf(STR_MATCH_IGNORE_CASE_UDF.clone());

        // declare a table in memory. In spark API, this corresponds to
        // createDataFrame(...).
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        for sql in sql {
            let df = ctx.sql(sql).await.unwrap();
            let result = df.collect().await.unwrap();
            let count = result.iter().map(|batch| batch.num_rows()).sum::<usize>();
            assert_eq!(count, 1);
        }
    }
}
