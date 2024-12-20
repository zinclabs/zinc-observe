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

use arrow::array::UInt64Array;
use config::utils::json;
use datafusion::{
    arrow::{array::ArrayRef, datatypes::DataType},
    common::cast::as_string_array,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// The name of the arrcount UDF given to DataFusion.
pub const ARR_COUNT_UDF_NAME: &str = "arrcount";

/// Implementation of arrcount
pub(crate) static ARR_COUNT_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        ARR_COUNT_UDF_NAME,
        // expects two string - the field and the delimiter
        vec![DataType::Utf8],
        // returns string
        DataType::UInt64,
        Volatility::Immutable,
        Arc::new(arr_count_impl),
    )
});

/// arrcount function for datafusion
pub fn arr_count_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    log::debug!("Inside arrcount");
    if args.len() != 1 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError("UDF params should be: arrcount(arr_field)".to_string()),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;
    // log::debug!("Got the args: {:#?}", args);

    // 1. cast both arguments to be aligned with the signature
    let arr_field = as_string_array(&args[0])?;

    // 2. perform the computation
    let array = arr_field
        .iter()
        .map(|arr_field| {
            match arr_field {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return 0 when the argument is null.
                Some(arr_field) => {
                    let arr_field: json::Value =
                        json::from_str(arr_field).expect("Failed to deserialize arrzip field1");
                    match arr_field {
                        json::Value::Array(field) => Some(field.len() as u64),
                        _ => Some(0),
                    }
                }
                _ => Some(0),
            }
        })
        .collect::<UInt64Array>();

    // `Ok` because no error occurred during the calculation
    // `Arc` because arrays are immutable, thread-safe, trait objects.
    Ok(ColumnarValue::from(Arc::new(array) as ArrayRef))
}

#[cfg(test)]
mod tests {
    use arrow::array::StringArray;
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
    async fn test_arr_count_udf() {
        let sqls = [
            (
                "select arrcount(bools) as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| 3   |", "+-----+"],
            ),
            (
                "select arrcount(names) as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| 3   |", "+-----+"],
            ),
            (
                "select arrcount(nums) as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| 4   |", "+-----+"],
            ),
            (
                "select arrcount(floats) as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| 4   |", "+-----+"],
            ),
            (
                "select arrcount(mixed) as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| 5   |", "+-----+"],
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
                    "[\"hello2\",\"hi2\",123, 23.90, false]",
                ])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(ARR_COUNT_UDF.clone());

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
