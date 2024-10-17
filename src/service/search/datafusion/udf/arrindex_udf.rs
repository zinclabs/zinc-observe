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

use arrow::datatypes::ArrowNativeType;
use config::utils::json;
use datafusion::{
    arrow::{
        array::{ArrayRef, StringArray},
        datatypes::DataType,
    },
    common::cast::{as_int64_array, as_string_array},
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// The name of the arrindex UDF given to DataFusion.
pub const ARR_INDEX_UDF_NAME: &str = "arrindex";

/// Implementation of arrindex
pub(crate) static ARR_INDEX_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        ARR_INDEX_UDF_NAME,
        // expects a string (the array field) and two integers - start index and end index
        vec![DataType::Utf8, DataType::Int64, DataType::Int64],
        // returns string
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(arr_index_impl),
    )
});

/// arrindex function for datafusion
pub fn arr_index_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    log::debug!("Inside arrindex");
    if args.len() < 2 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError(
                "UDF params should be: arrindex(field1, start, end)".to_string(),
            ),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;
    // log::debug!("Got the args: {:#?}", args);

    // 1. cast both arguments to Union. These casts MUST be aligned with the signature or this
    //    function panics!
    let arr_field1 = as_string_array(&args[0]).expect("cast failed");
    let start = as_int64_array(&args[1]).expect("cast failed");
    let end = if args.len() == 3 {
        let end = as_int64_array(&args[2]).expect("cast failed");
        if !end.is_empty() {
            Some(end.value(0))
        } else {
            None
        }
    } else {
        None
    };

    // 2. perform the computation
    let array = zip(arr_field1.iter(), start.iter())
        .map(|(arr_field1, start)| {
            let mut index_arrs = vec![];

            match (arr_field1, start) {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return null when either argument is null.
                (Some(arr_field1), Some(start)) => {
                    let start = start as usize;
                    let arr_field1: json::Value =
                        json::from_str(arr_field1).expect("Failed to deserialize arrzip field1");
                    // This field is assumed to be an array field
                    if let json::Value::Array(field1) = arr_field1 {
                        if field1.len() > start {
                            if let Some(end) = end {
                                // end is min(end, field1.len)
                                let end = if end.as_usize() < field1.len() {
                                    end as usize
                                } else {
                                    field1.len() - 1
                                };
                                if start <= end && end < field1.len() {
                                    for item in field1.into_iter().take(end + 1).skip(start) {
                                        index_arrs.push(item);
                                    }
                                }
                            } else {
                                index_arrs.push(field1[start].clone());
                            }
                        }
                    }
                    let index_arrs =
                        json::to_string(&index_arrs).expect("Failed to stringify arrs");
                    Some(index_arrs)
                }
                _ => None,
            }
        })
        .collect::<StringArray>();

    // `Ok` because no error occurred during the calculation
    // `Arc` because arrays are immutable, thread-safe, trait objects.
    Ok(ColumnarValue::from(Arc::new(array) as ArrayRef))
}

#[cfg(test)]
mod tests {
    use datafusion::{
        arrow::{
            datatypes::{Field, Schema},
            record_batch::RecordBatch,
        },
        assert_batches_eq,
        datasource::MemTable,
        prelude::SessionContext,
    };

    use super::*;

    #[tokio::test]
    async fn test_arr_index_udf() {
        let sqls = [
            (
                // Should include values at index 0 to index 1 (inclusive)
                "select arrindex(bools, 0, 1) as ret from t",
                vec![
                    "+--------------+",
                    "| ret          |",
                    "+--------------+",
                    "| [true,false] |",
                    "+--------------+",
                ],
            ),
            (
                // Should include all the elements
                "select arrindex(names, 0, 6) as ret from t",
                vec![
                    "+-------------------------+",
                    "| ret                     |",
                    "+-------------------------+",
                    "| [\"hello2\",\"hi2\",\"bye2\"] |",
                    "+-------------------------+",
                ],
            ),
            (
                "select arrindex(nums, 1, 1) as ret from t",
                vec![
                    "+-------+",
                    "| ret   |",
                    "+-------+",
                    "| [345] |",
                    "+-------+",
                ],
            ),
            (
                "select arrindex(floats, 1, 3) as ret from t",
                vec![
                    "+----------------+",
                    "| ret            |",
                    "+----------------+",
                    "| [34.5,2.6,4.5] |",
                    "+----------------+",
                ],
            ),
            (
                "select arrindex(floats, 7, 9) as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| []  |", "+-----+"],
            ),
            (
                "select arrindex(mixed, 2, 4) as ret from t",
                vec![
                    "+------------------+",
                    "| ret              |",
                    "+------------------+",
                    "| [123,23.9,false] |",
                    "+------------------+",
                ],
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("bools", DataType::Utf8, false),
            Field::new("names", DataType::Utf8, false),
            Field::new("nums", DataType::Utf8, false),
            Field::new("floats", DataType::Utf8, false),
            Field::new("mixed", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["a"])),
                Arc::new(StringArray::from(vec!["[true,false,true]"])),
                Arc::new(StringArray::from(vec!["[\"hello2\",\"hi2\",\"bye2\"]"])),
                Arc::new(StringArray::from(vec!["[12, 345, 23, 45]"])),
                Arc::new(StringArray::from(vec!["[1.9, 34.5, 2.6, 4.5]"])),
                Arc::new(StringArray::from(vec![
                    "[\"hello2\",\"hi2\",123, 23.9, false]",
                ])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(ARR_INDEX_UDF.clone());

        // declare a table in memory. In spark API, this corresponds to
        // createDataFrame(...).
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        for item in sqls {
            let df = ctx.sql(item.0).await.unwrap();
            let data = df.collect().await.unwrap();
            assert_batches_eq!(item.1, &data);
        }
    }
}
