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

/// The name of the arrjoin UDF given to DataFusion.
pub const SPATH_UDF_NAME: &str = "spath";

/// Implementation of arrjoin
pub(crate) static SPATH_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        SPATH_UDF_NAME,
        // expects two string - the field and the delimiter
        vec![DataType::Utf8, DataType::Utf8],
        // returns string
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(spath_impl),
    )
});

/// arrjoin function for datafusion
pub fn spath_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    log::debug!("Inside spath");
    if args.len() != 2 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError("UDF params should be: spath(field, path)".to_string()),
            None,
        ));
    }
    let args = ColumnarValue::values_to_arrays(args)?;
    // log::debug!("Got the args: {:#?}", args);

    // 1. cast both arguments to be aligned with the signature
    let field = as_string_array(&args[0])?;
    let path = as_string_array(&args[1])?;

    // 2. perform the computation
    let array = zip(field.iter(), path.iter())
        .map(|(field, path)| {
            match (field, path) {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return null when either argument is null.
                (Some(field), Some(path)) => {
                    let mut field: json::Value =
                        json::from_str(field).expect("Failed to deserialize arrzip field1");
                    let mut found = true;
                    let paths = path.split('.').collect::<Vec<&str>>();
                    for path in paths.into_iter() {
                        if let json::Value::Object(mut obj) = field.clone() {
                            if let Some(val) = obj.remove(path) {
                                field = val;
                            } else {
                                found = false;
                                break;
                            }
                        } else {
                            found = false;
                            break;
                        }
                    }
                    if found {
                        let field = super::stringify_json_value(&field);
                        Some(field)
                    } else {
                        None
                    }
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
    async fn test_spath_udf() {
        let sqls = [
            (
                // Should include values at index 0 to index 1 (inclusive)
                "select spath(object, 'nested.value') as ret from t",
                vec!["+------+", "| ret  |", "+------+", "| jene |", "+------+"],
            ),
            (
                // Should include all the elements
                "select spath(object, 'unnested') as ret from t",
                vec!["+-----+", "| ret |", "+-----+", "| doe |", "+-----+"],
            ),
            (
                "select spath(object, 'array') as ret from t",
                vec![
                    "+------------+",
                    "| ret        |",
                    "+------------+",
                    "| [34,54,45] |",
                    "+------------+",
                ],
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![Field::new(
            "object",
            DataType::Utf8,
            false,
        )]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(StringArray::from(vec![
                "{\"nested\":{\"value\":\"jene\"},\"unnested\":\"doe\",\"array\":[34,54,45]}",
            ]))],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(SPATH_UDF.clone());

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
