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

use arrow::array::StringArray;
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

/// The name of the arrzip UDF given to DataFusion.
pub const ARR_ZIP_UDF_NAME: &str = "arrzip";

/// Implementation of arrzip UDF
pub(crate) static ARR_ZIP_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        ARR_ZIP_UDF_NAME,
        // expects three string - field1, field2 and the delim
        vec![DataType::Utf8, DataType::Utf8, DataType::Utf8],
        // returns string
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(arr_zip_impl),
    )
});

/// arrzip function for datafusion
pub fn arr_zip_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    log::debug!("Inside arrzip");
    if args.len() != 3 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError(
                "UDF params should be: arrzip(field1, field2, delim)".to_string(),
            ),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;
    // log::debug!("Got the args: {:#?}", args);

    // 1. cast both arguments to Union. These casts MUST be aligned with the signature or this
    //    function panics!
    let arr_field1 = as_string_array(&args[0]).expect("cast failed");
    let arr_field2 = as_string_array(&args[1]).expect("cast failed");
    let delim = as_string_array(&args[2]).expect("cast failed");

    // 2. perform the computation
    let array = zip(arr_field1.iter(), zip(arr_field2.iter(), delim.iter()))
        .map(|(arr_field1, val)| {
            match (arr_field1, val) {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return null when either argument is null.
                (Some(arr_field1), (Some(arr_field2), Some(delim))) => {
                    let arr_field1: json::Value =
                        json::from_str(arr_field1).expect("Failed to deserialize arrzip field1");
                    let arr_field2: json::Value =
                        json::from_str(arr_field2).expect("Failed to deserialize arrzip field2");
                    let mut zipped_arrs = vec![];
                    if let (json::Value::Array(field1), json::Value::Array(field2)) =
                        (arr_field1, arr_field2)
                    {
                        // Field1 and field2 can be of different types
                        zip(field1.iter(), field2.iter()).for_each(|(field1, field2)| {
                            let field1 = super::stringify_json_value(field1);
                            let field2 = super::stringify_json_value(field2);

                            zipped_arrs.push(format!("{field1}{delim}{field2}"));
                        });
                    }
                    let zipped_arrs =
                        json::to_string(&zipped_arrs).expect("Failed to stringify zipped arrs");
                    Some(zipped_arrs)
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
    async fn test_arr_zip_udf() {
        let sqls = [
            (
                // Should include values at index 0 to index 1 (inclusive)
                "select arrzip(bools, names, ',') as ret from t",
                vec![
                    "+-----------------------------------------+",
                    "| ret                                     |",
                    "+-----------------------------------------+",
                    "| [\"true,hello2\",\"false,hi2\",\"true,bye2\"] |",
                    "+-----------------------------------------+",
                ],
            ),
            (
                // Should include all the elements
                "select arrzip(nums, floats, ',') as ret from t",
                vec![
                    "+-----------------------------------------+",
                    "| ret                                     |",
                    "+-----------------------------------------+",
                    "| [\"12,1.9\",\"345,34.5\",\"23,2.6\",\"45,4.5\"] |",
                    "+-----------------------------------------+",
                ],
            ),
            (
                "select arrzip(nums, mixed, ',') as ret from t",
                vec![
                    "+--------------------------------------------+",
                    "| ret                                        |",
                    "+--------------------------------------------+",
                    "| [\"12,hello2\",\"345,hi2\",\"23,123\",\"45,23.9\"] |",
                    "+--------------------------------------------+",
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
        ctx.register_udf(ARR_ZIP_UDF.clone());

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
