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

use chrono::{FixedOffset, TimeZone, Utc};
use config::utils::time;
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

/// The name of the date_format UDF given to DataFusion.
pub const DATE_FORMAT_UDF_NAME: &str = "date_format";

/// Implementation of date_format
pub(crate) static DATE_FORMAT_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        DATE_FORMAT_UDF_NAME,
        // expects three string
        vec![DataType::Int64, DataType::Utf8, DataType::Utf8],
        // returns string
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(date_format_expr_impl),
    )
});

/// date_format function for datafusion
pub fn date_format_expr_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    if args.len() != 3 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError(
                "UDF params should be: date_format(field, format, zone)".to_string(),
            ),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;

    // 1. cast both arguments to be aligned with the signature
    let timestamp = as_int64_array(&args[0])?;
    let format = as_string_array(&args[1])?;
    let timezone = as_string_array(&args[2])?;

    // 2. perform the computation
    let array = zip(timestamp.iter(), zip(format.iter(), timezone.iter()))
        .map(|(timestamp, val)| {
            match (timestamp, val) {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return null when either argument is null.
                (Some(timestamp), (Some(format), Some(timezone))) => {
                    let timestamp = time::parse_i64_to_timestamp_micros(timestamp);
                    let t = Utc.timestamp_nanos(timestamp * 1000);
                    let offset = time::parse_timezone_to_offset(timezone) as i32;
                    let result = if offset == 0 {
                        t.format(format).to_string()
                    } else {
                        let t = t.with_timezone(&FixedOffset::east_opt(offset).unwrap());
                        t.format(format).to_string()
                    };
                    Some(result)
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
    use arrow::array::Int64Array;
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
    async fn test_date_format_udf() {
        let data_time = time::parse_str_to_timestamp_micros("2021-01-01T00:00:00.000Z").unwrap();
        let sqls = [
            (
                "select date_format(time, '%Y-%m-%d', '') as ret from t",
                vec![
                    "+------------+",
                    "| ret        |",
                    "+------------+",
                    "| 2021-01-01 |",
                    "+------------+",
                ],
            ),
            (
                "select date_format(time, '%Y-%m-%d', 'UTC') as ret from t",
                vec![
                    "+------------+",
                    "| ret        |",
                    "+------------+",
                    "| 2021-01-01 |",
                    "+------------+",
                ],
            ),
            (
                "select date_format(time, '%H:%M:%S', 'UTC') as ret from t",
                vec![
                    "+----------+",
                    "| ret      |",
                    "+----------+",
                    "| 00:00:00 |",
                    "+----------+",
                ],
            ),
            (
                "select date_format(time, '%H:%M:%S', '+08:00') as ret from t",
                vec![
                    "+----------+",
                    "| ret      |",
                    "+----------+",
                    "| 08:00:00 |",
                    "+----------+",
                ],
            ),
            (
                "select date_format(time, '%Y-%m-%dT%H:%M:%S', '-08:00') as ret from t",
                vec![
                    "+---------------------+",
                    "| ret                 |",
                    "+---------------------+",
                    "| 2020-12-31T16:00:00 |",
                    "+---------------------+",
                ],
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("log", DataType::Utf8, false),
            Field::new("time", DataType::Int64, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec!["a"])),
                Arc::new(Int64Array::from(vec![data_time])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(DATE_FORMAT_UDF.clone());

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
