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

use config::utils::{str, time};
use datafusion::{
    arrow::{
        array::{ArrayRef, BooleanArray},
        datatypes::DataType,
    },
    common::cast::{as_int64_array, as_string_array},
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// The name of the time_range UDF given to DataFusion.
pub const TIME_RANGE_UDF_NAME: &str = "time_range";

/// Implementation of time_range
pub(crate) static TIME_RANGE_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        TIME_RANGE_UDF_NAME,
        // expects three string
        vec![DataType::Int64, DataType::Utf8, DataType::Utf8],
        // returns boolean
        DataType::Boolean,
        Volatility::Immutable,
        Arc::new(time_range_expr_impl),
    )
});

/// time_range function for datafusion
pub fn time_range_expr_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    if args.len() != 3 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError(
                "UDF params should be: time_range(field, start, end)".to_string(),
            ),
            None,
        ));
    }

    let args = ColumnarValue::values_to_arrays(args)?;

    // 1. cast both arguments to be aligned with the signature
    let base = as_int64_array(&args[0])?;
    let min = as_string_array(&args[1])?;
    let max = as_string_array(&args[2])?;

    // 2. perform the computation
    let array = zip(base.iter(), zip(min.iter(), max.iter()))
        .map(|(base, val)| {
            match (base, val) {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return null when either argument is null.
                (Some(base), (Some(min), Some(max))) => {
                    let min = match time::parse_str_to_timestamp_micros(min) {
                        Ok(v) => v,
                        Err(_) => return None,
                    };
                    let max = match time::parse_str_to_timestamp_micros(max) {
                        Ok(v) => v,
                        Err(_) => return None,
                    };
                    let result: bool;
                    if min > 0 && max > 0 {
                        result = base >= min && base < max;
                    } else if min > 0 {
                        result = base >= min;
                    } else if max > 0 {
                        result = base < max;
                    } else {
                        result = true;
                    }
                    Some(result)
                }
                _ => None,
            }
        })
        .collect::<BooleanArray>();

    // `Ok` because no error occurred during the calculation
    // `Arc` because arrays are immutable, thread-safe, trait objects.
    Ok(ColumnarValue::from(Arc::new(array) as ArrayRef))
}

#[cfg(test)]
mod tests {
    use datafusion::{
        arrow::{
            array::{Int64Array, StringArray},
            datatypes::{Field, Schema},
            record_batch::RecordBatch,
        },
        datasource::MemTable,
        prelude::SessionContext,
    };

    use super::*;

    #[tokio::test]
    async fn test_time_range() {
        let data_time = time::parse_str_to_timestamp_micros("2021-01-01T00:00:00.000Z").unwrap();
        let sql = "select * from t where time_range(time, '2021-01-01T00:00:00.000Z', '2021-01-02T00:00:00.000Z')";

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("time", DataType::Int64, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["a", "b", "c", "d"])),
                Arc::new(Int64Array::from(vec![
                    data_time - 1,
                    data_time,
                    data_time + 1,
                    data_time + 2,
                ])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(TIME_RANGE_UDF.clone());

        // declare a table in memory. In spark API, this corresponds to
        // createDataFrame(...).
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();

        let df = ctx.sql(sql).await.unwrap();
        let result = df.collect().await.unwrap();
        let count = result.iter().map(|batch| batch.num_rows()).sum::<usize>();
        assert_eq!(count, 3);
    }
}
