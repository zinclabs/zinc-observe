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

/// The name of the arr_descending UDF given to DataFusion.
pub const ARR_DESCENDING_UDF_NAME: &str = "arr_descending";

/// Implementation of arr_descending
pub(crate) static ARR_DESCENDING_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        ARR_DESCENDING_UDF_NAME,
        // expects three string
        vec![DataType::Utf8],
        // returns string
        Arc::new(DataType::Utf8),
        Volatility::Immutable,
        Arc::new(arr_descending_impl),
    )
});

/// arr_descending function for datafusion
pub fn arr_descending_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    log::debug!("Inside arr_descending");
    if args.len() != 1 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError("UDF params should be: arr_descending(field1)".to_string()),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;
    // log::debug!("Got the args: {:#?}", args);

    // 1. cast the argument to Union. This cast MUST be aligned with the signature or this function
    //    panics!
    let arr_field = as_string_array(&args[0]).expect("cast failed");

    // 2. perform the computation
    let array = arr_field
        .iter()
        .map(|arr_field| {
            let mut desc_arrs = vec![];

            if let Some(arr_field) = arr_field {
                let arr_field: json::Value =
                    json::from_str(arr_field).expect("Failed to deserialize arrzip field1");
                // This field is assumed to be a multivalue field
                if let json::Value::Array(mut field1) = arr_field {
                    field1.sort_by(|a, b| {
                        // Assuming the array having elements of same type
                        if a.is_f64() {
                            b.as_f64().unwrap().total_cmp(a.as_f64().as_ref().unwrap())
                        } else if a.is_i64() {
                            b.as_i64().unwrap().cmp(a.as_i64().as_ref().unwrap())
                        } else if a.is_u64() {
                            b.as_u64().unwrap().cmp(a.as_u64().as_ref().unwrap())
                        } else if a.is_string() {
                            b.as_str().unwrap().cmp(a.as_str().unwrap())
                        } else if a.is_boolean() {
                            b.as_bool().unwrap().cmp(a.as_bool().as_ref().unwrap())
                        } else {
                            Ordering::Greater
                        }
                    });
                    desc_arrs.append(&mut field1);
                }
                let desc_arrs =
                    json::to_string(&desc_arrs).expect("Failed to stringify descending array");
                Some(desc_arrs)
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
    async fn test_arr_descending_udf() {
        let sqls = [
            (
                "select arr_descending(bools) as ret from t",
                vec![
                    "+-------------------+",
                    "| ret               |",
                    "+-------------------+",
                    "| [true,true,false] |",
                    "+-------------------+",
                ],
            ),
            (
                "select arr_descending(names) as ret from t",
                vec![
                    "+-------------------------+",
                    "| ret                     |",
                    "+-------------------------+",
                    "| [\"hi2\",\"hello2\",\"bye2\"] |",
                    "+-------------------------+",
                ],
            ),
            (
                "select arr_descending(nums) as ret from t",
                vec![
                    "+----------------+",
                    "| ret            |",
                    "+----------------+",
                    "| [345,45,23,12] |",
                    "+----------------+",
                ],
            ),
            (
                "select arr_descending(floats) as ret from t",
                vec![
                    "+--------------------+",
                    "| ret                |",
                    "+--------------------+",
                    "| [34.5,4.5,2.6,1.9] |",
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
        ctx.register_udf(ARR_DESCENDING_UDF.clone());

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
