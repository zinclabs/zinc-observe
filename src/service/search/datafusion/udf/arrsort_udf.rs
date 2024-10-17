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

use std::{cmp::Ordering, sync::Arc};

use config::utils::json;
use datafusion::{
    arrow::{
        array::{ArrayRef, StringArray},
        datatypes::DataType,
    },
    common::cast::as_string_array,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// The name of the arrsort UDF given to DataFusion.
pub const ARR_SORT_UDF_NAME: &str = "arrsort";

/// Implementation of arrsort
pub(crate) static ARR_SORT_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        ARR_SORT_UDF_NAME,
        // expects one string - the array field
        vec![DataType::Utf8],
        // returns string
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(arr_sort_impl),
    )
});

/// arrsort function for datafusion
pub fn arr_sort_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    log::debug!("Inside arrsort");
    if args.len() != 1 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError("UDF params should be: arrsort(field)".to_string()),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;
    // log::debug!("Got the args: {:#?}", args);

    // 1. cast argument to a string array. These casts MUST be aligned with the signature or this
    //    function panics!
    let arr_field = as_string_array(&args[0]).expect("cast failed");

    // 2. perform the computation
    let array = arr_field
        .iter()
        .map(|arr_field| {
            let mut arr_sorted = vec![];

            if let Some(arr_field) = arr_field {
                let arr_field: json::Value =
                    json::from_str(arr_field).expect("Failed to deserialize arrzip field1");
                // This field is assumed to be a multivalue field
                if let json::Value::Array(mut field) = arr_field {
                    field.sort_by(|a, b| {
                        // Assuming the array having elements of same type
                        if a.is_f64() {
                            a.as_f64().unwrap().total_cmp(b.as_f64().as_ref().unwrap())
                        } else if a.is_i64() {
                            a.as_i64().unwrap().cmp(b.as_i64().as_ref().unwrap())
                        } else if a.is_u64() {
                            a.as_u64().unwrap().cmp(b.as_u64().as_ref().unwrap())
                        } else if a.is_string() {
                            a.as_str().unwrap().cmp(b.as_str().unwrap())
                        } else if a.is_boolean() {
                            a.as_bool().unwrap().cmp(b.as_bool().as_ref().unwrap())
                        } else {
                            Ordering::Less
                        }
                    });
                    arr_sorted.append(&mut field);
                }
                let arr_sorted =
                    json::to_string(&arr_sorted).expect("Failed to stringify sorted arrs");
                Some(arr_sorted)
            } else {
                None
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
    async fn test_arr_sort_udf() {
        let sqls = [
            (
                "select arrsort(bools) as ret from t",
                vec![
                    "+-------------------+",
                    "| ret               |",
                    "+-------------------+",
                    "| [false,true,true] |",
                    "+-------------------+",
                ],
            ),
            (
                "select arrsort(names) as ret from t",
                vec![
                    "+-------------------------+",
                    "| ret                     |",
                    "+-------------------------+",
                    "| [\"bye2\",\"hello2\",\"hi2\"] |",
                    "+-------------------------+",
                ],
            ),
            (
                "select arrsort(nums) as ret from t",
                vec![
                    "+----------------+",
                    "| ret            |",
                    "+----------------+",
                    "| [12,23,45,345] |",
                    "+----------------+",
                ],
            ),
            (
                "select arrsort(floats) as ret from t",
                vec![
                    "+--------------------+",
                    "| ret                |",
                    "+--------------------+",
                    "| [1.9,2.6,4.5,34.5] |",
                    "+--------------------+",
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
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(ARR_SORT_UDF.clone());

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
