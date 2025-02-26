// Copyright 2025 OpenObserve Inc.
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

use std::any::Any;

use arrow_schema::TimeUnit;
use datafusion::{
    arrow::datatypes::DataType,
    common::{exec_err, not_impl_err},
    error::Result,
    functions::datetime::to_timestamp::ToTimestampMicrosFunc,
    logical_expr::{ColumnarValue, ScalarUDF, ScalarUDFImpl, Signature, Volatility},
};
use once_cell::sync::Lazy;

/// The name of the cast_to_timestamp UDF given to DataFusion.
pub const CAST_TO_TIMESTAMP_UDF_NAME: &str = "cast_to_timestamp";

/// Implementation of cast_to_timestamp
pub(crate) static CAST_TO_TIMESTAMP_UDF: Lazy<ScalarUDF> =
    Lazy::new(|| ScalarUDF::from(CastToTimestampUdf::new()));

#[derive(Debug, Clone)]
struct CastToTimestampUdf {
    signature: Signature,
}

impl CastToTimestampUdf {
    fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
        }
    }
}

impl ScalarUDFImpl for CastToTimestampUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        CAST_TO_TIMESTAMP_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Int64)
    }

    fn invoke(&self, _args: &[ColumnarValue]) -> Result<ColumnarValue> {
        not_impl_err!(
            "Function {} does not implement invoke but called",
            self.name()
        )
    }

    fn invoke_batch(&self, args: &[ColumnarValue], number_rows: usize) -> Result<ColumnarValue> {
        let ttm = ToTimestampMicrosFunc::new();
        let ret = ttm.invoke_batch(args, number_rows)?;

        match ret.data_type() {
            DataType::Int32 | DataType::Int64 | DataType::Null | DataType::Float64 => {
                ret.cast_to(&DataType::Int64, None)
            }
            DataType::Date64 | DataType::Date32 | DataType::Timestamp(_, None) => ret
                .cast_to(&DataType::Timestamp(TimeUnit::Microsecond, None), None)?
                .cast_to(&DataType::Int64, None),
            DataType::Timestamp(_, Some(tz)) => ret
                .cast_to(&DataType::Timestamp(TimeUnit::Microsecond, Some(tz)), None)?
                .cast_to(&DataType::Int64, None),
            DataType::Utf8 => CastToTimestampUdf::new()
                .invoke_batch(args, number_rows)?
                .cast_to(&DataType::Int64, None),
            other => {
                exec_err!(
                    "Unsupported data type {:?} for function {}",
                    other,
                    self.name()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{Int64Array, StringArray};
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
    async fn test_cast_to_timestamp_udf() {
        let now = config::utils::time::now_micros();
        let sqls = [
            (
                "SELECT 1, LENGTH(cast_to_timestamp(NOW())) AS len from t WHERE time <= cast_to_timestamp(NOW())",
                vec![
                    "+----------+-----+",
                    "| Int64(1) | len |",
                    "+----------+-----+",
                    "| 1        | 16  |",
                    "+----------+-----+",
                ],
            ),
            (
                "SELECT 1 from t WHERE time > cast_to_timestamp(NOW() - INTERVAL '1 minute')",
                vec![
                    "+----------+",
                    "| Int64(1) |",
                    "+----------+",
                    "| 1        |",
                    "+----------+",
                ],
            ),
            (
                "SELECT 1 from t WHERE time > cast_to_timestamp(NOW() - INTERVAL '1 hour')",
                vec![
                    "+----------+",
                    "| Int64(1) |",
                    "+----------+",
                    "| 1        |",
                    "+----------+",
                ],
            ),
            (
                "SELECT 1 from t WHERE time > cast_to_timestamp('2025-01-01T00:00:00.000Z')",
                vec![
                    "+----------+",
                    "| Int64(1) |",
                    "+----------+",
                    "| 1        |",
                    "+----------+",
                ],
            ),
            (
                "SELECT 1 from t WHERE time > cast_to_timestamp('2025-01-01T00:00:00+08:00')",
                vec![
                    "+----------+",
                    "| Int64(1) |",
                    "+----------+",
                    "| 1        |",
                    "+----------+",
                ],
            ),
            (
                "SELECT 1 from t WHERE time > cast_to_timestamp(1740398708294000)",
                vec![
                    "+----------+",
                    "| Int64(1) |",
                    "+----------+",
                    "| 1        |",
                    "+----------+",
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
                Arc::new(Int64Array::from(vec![now])),
            ],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(CAST_TO_TIMESTAMP_UDF.clone());

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
