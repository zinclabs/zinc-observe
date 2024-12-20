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

use std::{iter::zip, sync::Arc};

use config::utils::tantivy::tokenizer::o2_collect_tokens;
use datafusion::{
    arrow::{
        array::{ArrayRef, BooleanArray},
        datatypes::DataType,
    },
    common::{
        cast::{as_int64_array, as_string_array},
        utils::datafusion_strsim::levenshtein,
    },
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarFunctionImplementation, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// Implementation of fuzzy_match
pub(crate) static FUZZY_MATCH_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        super::FUZZY_MATCH_UDF_NAME,
        // expects two string
        vec![DataType::Utf8, DataType::Utf8, DataType::Int64],
        // returns boolean
        DataType::Boolean,
        Volatility::Stable,
        fuzzy_match_expr_impl(),
    )
});

/// fuzzy_match function for datafusion
pub fn fuzzy_match_expr_impl() -> ScalarFunctionImplementation {
    Arc::new(move |args: &[ColumnarValue]| {
        if args.len() != 3 {
            return Err(DataFusionError::SQL(
                ParserError::ParserError("fuzzy_match UDF expects two string".to_string()),
                None,
            ));
        }
        let args = ColumnarValue::values_to_arrays(args)?;

        // 1. cast both arguments to be aligned with the signature
        let haystack = as_string_array(&args[0])?;
        let needle = as_string_array(&args[1])?;
        let distance = as_int64_array(&args[2])?;

        // 2. perform the computation
        let array = zip(haystack.iter(), zip(needle.iter(), distance.iter()))
            .map(|(haystack, (needle, distance))| {
                match (haystack, needle, distance) {
                    // in arrow, any value can be null.
                    // Here we decide to make our UDF to return null when either haystack or needle
                    // is null.
                    (Some(haystack), Some(needle), Some(dis)) => Some({
                        let terms = o2_collect_tokens(haystack);
                        terms
                            .iter()
                            .any(|term| levenshtein(term, needle) as i64 <= dis)
                    }),
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
    async fn test_fuzzy_match_udf() {
        let sql = "select * from t where fuzzy_match(log, 'data', 1)";

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("log", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(StringArray::from(vec![
                    "here has data",
                    "here has date",
                    "here no datt",
                    "here no daxx",
                    "",
                ])),
            ],
        )
        .unwrap();

        let ctx = SessionContext::new();
        ctx.register_udf(FUZZY_MATCH_UDF.clone());

        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        let df = ctx.sql(sql).await.unwrap();
        let result = df.collect().await.unwrap();
        let count = result.iter().map(|batch| batch.num_rows()).sum::<usize>();
        assert_eq!(count, 3);
    }
}
