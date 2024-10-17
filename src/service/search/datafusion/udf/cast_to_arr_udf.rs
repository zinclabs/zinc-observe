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

use arrow::array::{Array, ListBuilder, StringBuilder};
use arrow_schema::Field;
use config::utils::json;
use datafusion::{
    arrow::{array::ArrayRef, datatypes::DataType},
    common::cast::as_generic_string_array,
    error::DataFusionError,
    logical_expr::{ColumnarValue, ScalarUDF, Volatility},
    prelude::create_udf,
    sql::sqlparser::parser::ParserError,
};
use once_cell::sync::Lazy;

/// The name of the cast_to_arr UDF given to DataFusion.
pub const CAST_TO_ARR_UDF_NAME: &str = "cast_to_arr";

/// Implementation of cast_to_arr
pub(crate) static CAST_TO_ARR_UDF: Lazy<ScalarUDF> = Lazy::new(|| {
    create_udf(
        CAST_TO_ARR_UDF_NAME,
        // takes one argument: the field
        vec![DataType::Utf8],
        Arc::new(DataType::List(Arc::new(Field::new(
            "item",
            DataType::Utf8,
            true,
        )))),
        Volatility::Immutable,
        Arc::new(cast_to_arr_impl),
    )
});

/// cast_to_arr function for datafusion
pub fn cast_to_arr_impl(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    let args = ColumnarValue::values_to_arrays(args)?;
    log::debug!("Inside cast_to_arr_impl");
    if args.len() != 1 {
        return Err(DataFusionError::SQL(
            ParserError::ParserError("UDF params should be: cast_to_arr(field)".to_string()),
            None,
        ));
    }

    let string_array = as_generic_string_array::<i32>(&args[0])?;

    let mut list_builder = ListBuilder::new(StringBuilder::with_capacity(
        string_array.len(),
        string_array.get_buffer_memory_size(),
    ));

    string_array.iter().for_each(|string| {
        if let Some(string) = string {
            let arr: json::Value =
                json::from_str(string).expect("Failed to deserialize the field into an array");
            if let json::Value::Array(arr) = arr {
                arr.iter().for_each(|field| {
                    let field = super::stringify_json_value(field);
                    if !field.is_empty() {
                        list_builder.values().append_value(field);
                    }
                });
                list_builder.append(true);
            }
        }
    });

    let list_array = list_builder.finish();
    Ok(ColumnarValue::from(Arc::new(list_array) as ArrayRef))
}

#[cfg(test)]
mod tests {
    use datafusion::{
        arrow::{array::StringArray, datatypes::Schema, record_batch::RecordBatch},
        assert_batches_eq,
        datasource::MemTable,
        prelude::SessionContext,
    };

    use super::*;

    #[tokio::test]
    async fn test_cast_to_arr() {
        let log_line = r#"[12, 345, 23, 45]"#;
        let sqls = [(
            "select cast_to_arr(log) as ret from t",
            vec![
                "+-------------------+",
                "| ret               |",
                "+-------------------+",
                "| [12, 345, 23, 45] |",
                "+-------------------+",
            ],
        )];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![Field::new("log", DataType::Utf8, false)]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(StringArray::from(vec![log_line]))],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(CAST_TO_ARR_UDF.clone());

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

    #[tokio::test]
    async fn test_cast_to_arr_string() {
        let log_line = r#"["hello2","hi2","bye2"]"#;
        let sqls = [(
            "select cast_to_arr(log) as ret from t",
            vec![
                "+---------------------+",
                "| ret                 |",
                "+---------------------+",
                "| [hello2, hi2, bye2] |",
                "+---------------------+",
            ],
        )];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![Field::new("log", DataType::Utf8, false)]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(StringArray::from(vec![log_line]))],
        )
        .unwrap();

        // declare a new context. In spark API, this corresponds to a new spark
        // SQLsession
        let ctx = SessionContext::new();
        ctx.register_udf(CAST_TO_ARR_UDF.clone());

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
