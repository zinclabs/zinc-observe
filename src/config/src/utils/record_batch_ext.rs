// Copyright 2024 Zinc Labs Inc.
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

use arrow::{
    array::{
        make_builder, new_null_array, ArrayBuilder, ArrayRef, BooleanArray, BooleanBuilder,
        Float64Array, Float64Builder, Int64Array, Int64Builder, NullBuilder, StringArray,
        StringBuilder, UInt64Array, UInt64Builder,
    },
    record_batch::RecordBatch,
};
use arrow_schema::{ArrowError, DataType, Schema};
use hashbrown::{HashMap, HashSet};

use super::{json::get_string_value, schema_ext::SchemaExt};
use crate::FxIndexMap;

const USIZE_SIZE: usize = std::mem::size_of::<usize>();

/// RecordBatchExt helper...
pub trait RecordBatchExt {
    fn size(&self) -> usize;
}

impl RecordBatchExt for RecordBatch {
    fn size(&self) -> usize {
        self.schema().size() + self.get_array_memory_size() + USIZE_SIZE
    }
}

// convert json value to record batch
pub fn convert_json_to_record_batch(
    schema: &Arc<Schema>,
    data: &[Arc<serde_json::Value>],
) -> Result<RecordBatch, ArrowError> {
    // collect all keys from the json data
    let mut keys = HashSet::with_capacity(schema.fields().len());
    let mut max_keys = 0;
    for record in data.iter() {
        let record = record.as_object().unwrap();
        if record.len() > max_keys {
            max_keys = record.len();
        }
        for (k, _) in record {
            keys.insert(k);
        }
    }

    // create builders for each key
    let records_len = data.len();
    let mut builders: FxIndexMap<&String, (&DataType, Box<dyn ArrayBuilder>)> = schema
        .fields()
        .iter()
        .filter(|f| keys.contains(f.name()))
        .map(|f| {
            (
                f.name(),
                (f.data_type(), make_builder(f.data_type(), records_len)),
            )
        })
        .collect();

    // fill builders with data
    let mut record_keys = HashSet::with_capacity(max_keys);
    for record in data.iter() {
        record_keys.clear();
        for (k, v) in record.as_object().unwrap() {
            record_keys.insert(k);
            let res = builders.get_mut(k);
            // where the value is null, the key maybe not exists in the schema
            // so we skip it
            if res.is_none() && v.is_null() {
                continue;
            }
            let (data_type, builder) = res.ok_or_else(|| {
                ArrowError::SchemaError(format!(
                    "Cannot find key {} (value: {:?}) in schema {:?}",
                    k, v, schema
                ))
            })?;
            match data_type {
                DataType::Utf8 => {
                    let b = builder
                        .as_any_mut()
                        .downcast_mut::<StringBuilder>()
                        .unwrap();
                    if v.is_null() {
                        b.append_null();
                    } else {
                        b.append_value(get_string_value(v));
                    }
                }
                DataType::Int64 => {
                    let b = builder.as_any_mut().downcast_mut::<Int64Builder>().unwrap();
                    if v.is_null() {
                        b.append_null();
                    } else {
                        b.append_value(v.as_i64().unwrap());
                    }
                }
                DataType::UInt64 => {
                    let b = builder
                        .as_any_mut()
                        .downcast_mut::<UInt64Builder>()
                        .unwrap();
                    if v.is_null() {
                        b.append_null();
                    } else {
                        b.append_value(v.as_u64().unwrap());
                    }
                }
                DataType::Float64 => {
                    let b = builder
                        .as_any_mut()
                        .downcast_mut::<Float64Builder>()
                        .unwrap();
                    if v.is_null() {
                        b.append_null();
                    } else {
                        b.append_value(v.as_f64().unwrap());
                    }
                }
                DataType::Boolean => {
                    let b = builder
                        .as_any_mut()
                        .downcast_mut::<BooleanBuilder>()
                        .unwrap();
                    if v.is_null() {
                        b.append_null();
                    } else {
                        b.append_value(v.as_bool().unwrap());
                    }
                }
                DataType::Null => {
                    let b = builder.as_any_mut().downcast_mut::<NullBuilder>().unwrap();
                    b.append_null();
                }
                _ => {
                    return Err(ArrowError::SchemaError(
                        "Cannot convert json to RecordBatch from non-basic type value".to_string(),
                    ));
                }
            }
        }

        // fill null for missing keys
        builders.iter_mut().for_each(|(k, (data_type, b))| {
            if !record_keys.contains(k) {
                match data_type {
                    DataType::Utf8 => {
                        b.as_any_mut()
                            .downcast_mut::<StringBuilder>()
                            .unwrap()
                            .append_null();
                    }
                    DataType::Int64 => {
                        b.as_any_mut()
                            .downcast_mut::<Int64Builder>()
                            .unwrap()
                            .append_null();
                    }
                    DataType::UInt64 => {
                        b.as_any_mut()
                            .downcast_mut::<UInt64Builder>()
                            .unwrap()
                            .append_null();
                    }
                    DataType::Float64 => {
                        b.as_any_mut()
                            .downcast_mut::<Float64Builder>()
                            .unwrap()
                            .append_null();
                    }
                    DataType::Boolean => {
                        b.as_any_mut()
                            .downcast_mut::<BooleanBuilder>()
                            .unwrap()
                            .append_null();
                    }
                    DataType::Null => {
                        b.as_any_mut()
                            .downcast_mut::<NullBuilder>()
                            .unwrap()
                            .append_null();
                    }
                    _ => {}
                }
            }
        });
    }

    let mut cols: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        if let Some((_, builder)) = builders.get_mut(field.name()) {
            cols.push(builder.finish());
        } else {
            cols.push(new_null_array(field.data_type(), records_len))
        }
    }

    RecordBatch::try_new(schema.clone(), cols)
}

pub fn format_recordbatch_by_schema(schema: Arc<Schema>, batch: RecordBatch) -> RecordBatch {
    if schema.fields().is_empty() || schema.fields() == batch.schema().fields() {
        return batch;
    }
    let records_len = batch.num_rows();
    if records_len == 0 {
        return RecordBatch::new_empty(schema);
    }

    let batch_schema = batch.schema();
    let batch_fields = batch_schema
        .fields()
        .iter()
        .map(|f| (f.name(), f.data_type()))
        .collect::<HashMap<_, _>>();

    let mut cols: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        match batch_fields.get(field.name()) {
            Some(data_type) if *data_type == field.data_type() => {
                cols.push(batch.column_by_name(field.name()).unwrap().clone());
            }
            Some(data_type) => {
                let col = batch.column_by_name(field.name()).unwrap();
                let mut builder = make_builder(field.data_type(), records_len);
                match field.data_type() {
                    DataType::Utf8 => {
                        let b = builder
                            .as_any_mut()
                            .downcast_mut::<StringBuilder>()
                            .unwrap();
                        match data_type {
                            DataType::Utf8 => {
                                let col = col.as_any().downcast_ref::<StringArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i));
                                }
                            }
                            DataType::Int64 => {
                                let col = col.as_any().downcast_ref::<Int64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).to_string());
                                }
                            }
                            DataType::UInt64 => {
                                let col = col.as_any().downcast_ref::<UInt64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).to_string());
                                }
                            }
                            DataType::Float64 => {
                                let col = col.as_any().downcast_ref::<Float64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).to_string());
                                }
                            }
                            DataType::Boolean => {
                                let col = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).to_string());
                                }
                            }
                            DataType::Null => {
                                for _ in 0..records_len {
                                    b.append_null();
                                }
                            }
                            _ => {}
                        }
                    }
                    DataType::Int64 => {
                        let b = builder.as_any_mut().downcast_mut::<Int64Builder>().unwrap();
                        match data_type {
                            DataType::Utf8 => {
                                let col = col.as_any().downcast_ref::<StringArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).parse().unwrap_or_default());
                                }
                            }
                            DataType::Int64 => {
                                let col = col.as_any().downcast_ref::<Int64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i));
                                }
                            }
                            DataType::UInt64 => {
                                let col = col.as_any().downcast_ref::<UInt64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as i64);
                                }
                            }
                            DataType::Float64 => {
                                let col = col.as_any().downcast_ref::<Float64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as i64);
                                }
                            }
                            DataType::Boolean => {
                                let col = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as i64);
                                }
                            }
                            DataType::Null => {
                                for _ in 0..records_len {
                                    b.append_null();
                                }
                            }
                            _ => {}
                        }
                    }
                    DataType::UInt64 => {
                        let b = builder
                            .as_any_mut()
                            .downcast_mut::<UInt64Builder>()
                            .unwrap();
                        match data_type {
                            DataType::Utf8 => {
                                let col = col.as_any().downcast_ref::<StringArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).parse().unwrap_or_default());
                                }
                            }
                            DataType::Int64 => {
                                let col = col.as_any().downcast_ref::<Int64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as u64);
                                }
                            }
                            DataType::UInt64 => {
                                let col = col.as_any().downcast_ref::<UInt64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i));
                                }
                            }
                            DataType::Float64 => {
                                let col = col.as_any().downcast_ref::<Float64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as u64);
                                }
                            }
                            DataType::Boolean => {
                                let col = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as u64);
                                }
                            }
                            DataType::Null => {
                                for _ in 0..records_len {
                                    b.append_null();
                                }
                            }
                            _ => {}
                        }
                    }
                    DataType::Float64 => {
                        let b = builder
                            .as_any_mut()
                            .downcast_mut::<Float64Builder>()
                            .unwrap();
                        match data_type {
                            DataType::Utf8 => {
                                let col = col.as_any().downcast_ref::<StringArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).parse().unwrap_or_default());
                                }
                            }
                            DataType::Int64 => {
                                let col = col.as_any().downcast_ref::<Int64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as f64);
                                }
                            }
                            DataType::UInt64 => {
                                let col = col.as_any().downcast_ref::<UInt64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as f64);
                                }
                            }
                            DataType::Float64 => {
                                let col = col.as_any().downcast_ref::<Float64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i));
                                }
                            }
                            DataType::Boolean => {
                                let col = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) as i64 as f64);
                                }
                            }
                            DataType::Null => {
                                for _ in 0..records_len {
                                    b.append_null();
                                }
                            }
                            _ => {}
                        }
                    }
                    DataType::Boolean => {
                        let b = builder
                            .as_any_mut()
                            .downcast_mut::<BooleanBuilder>()
                            .unwrap();
                        match data_type {
                            DataType::Utf8 => {
                                let col = col.as_any().downcast_ref::<StringArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i).parse().unwrap_or_default());
                                }
                            }
                            DataType::Int64 => {
                                let col = col.as_any().downcast_ref::<Int64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) > 0);
                                }
                            }
                            DataType::UInt64 => {
                                let col = col.as_any().downcast_ref::<UInt64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) > 0);
                                }
                            }
                            DataType::Float64 => {
                                let col = col.as_any().downcast_ref::<Float64Array>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i) > 0.0);
                                }
                            }
                            DataType::Boolean => {
                                let col = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                                for i in 0..records_len {
                                    b.append_value(col.value(i));
                                }
                            }
                            DataType::Null => {
                                for _ in 0..records_len {
                                    b.append_null();
                                }
                            }
                            _ => {}
                        }
                    }
                    DataType::Null => {
                        builder
                            .as_any_mut()
                            .downcast_mut::<NullBuilder>()
                            .unwrap()
                            .append_null();
                    }
                    _ => {}
                }
                cols.push(builder.finish());
            }
            _ => cols.push(new_null_array(field.data_type(), records_len)),
        }
    }
    RecordBatch::try_new(schema, cols).unwrap()
}

#[cfg(test)]
mod test {
    use arrow::util::pretty::pretty_format_batches;
    use arrow_schema::Field;

    use super::*;

    #[tokio::test]
    async fn test_format_recordbatch_by_schema() {
        // Create a sample schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("age", DataType::Float64, false),
            Field::new("city", DataType::Utf8, true),
        ]));

        // Create a sample record batch
        let name = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
        let age = Float64Array::from(vec![25.0, 30.0, 35.0]);
        let city = StringArray::from(vec!["New York", "London", "Paris"]);
        let record_batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(name.clone()) as ArrayRef,
                Arc::new(age.clone()) as ArrayRef,
                Arc::new(city.clone()) as ArrayRef,
            ],
        )
        .unwrap();

        // Format the record batch by the schema
        let formatted_batch = format_recordbatch_by_schema(schema.clone(), record_batch);

        // Assert that the formatted batch has the same schema as the input schema
        assert_eq!(formatted_batch.schema(), schema);

        // Assert that the formatted batch has the same number of rows as the input batch
        assert_eq!(formatted_batch.num_rows(), 3);

        // Assert that the formatted batch has the same data as the input batch
        assert_eq!(
            pretty_format_batches(&[formatted_batch])
                .unwrap()
                .to_string(),
            "+---------+------+----------+
| name    | age  | city     |
+---------+------+----------+
| Alice   | 25.0 | New York |
| Bob     | 30.0 | London   |
| Charlie | 35.0 | Paris    |
+---------+------+----------+"
        );
    }

    #[tokio::test]
    async fn test_format_recordbatch_by_schema_with_mismatch_schema() {
        // Create a sample schema
        let schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("age", DataType::Float64, false),
            Field::new("city", DataType::Utf8, true),
        ]));

        // Create a sample schema
        let record_schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("age", DataType::Utf8, false),
            Field::new("city", DataType::Utf8, true),
        ]));

        // Create a sample record batch
        let name = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
        let age = StringArray::from(vec!["25", "30", "35"]);
        let city = StringArray::from(vec!["New York", "London", "Paris"]);
        let record_batch = RecordBatch::try_new(
            record_schema,
            vec![
                Arc::new(name.clone()) as ArrayRef,
                Arc::new(age.clone()) as ArrayRef,
                Arc::new(city.clone()) as ArrayRef,
            ],
        )
        .unwrap();

        // Format the record batch by the schema
        let formatted_batch = format_recordbatch_by_schema(schema.clone(), record_batch);

        // Assert that the formatted batch has the same schema as the input schema
        assert_eq!(formatted_batch.schema(), schema);

        // Assert that the formatted batch has the same number of rows as the input batch
        assert_eq!(formatted_batch.num_rows(), 3);

        // Assert that the formatted batch has the same data as the input batch
        assert_eq!(
            pretty_format_batches(&[formatted_batch])
                .unwrap()
                .to_string(),
            "+---------+------+----------+
| name    | age  | city     |
+---------+------+----------+
| Alice   | 25.0 | New York |
| Bob     | 30.0 | London   |
| Charlie | 35.0 | Paris    |
+---------+------+----------+"
        );
    }

    #[tokio::test]
    async fn test_format_recordbatch_by_schema_with_empty_schema() {
        // Create a sample schema
        let schema = Arc::new(Schema::empty());

        // Create a sample schema
        let record_schema = Arc::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("age", DataType::Utf8, false),
            Field::new("city", DataType::Utf8, true),
        ]));

        // Create a sample record batch
        let name = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
        let age = StringArray::from(vec!["25", "30", "35"]);
        let city = StringArray::from(vec!["New York", "London", "Paris"]);
        let record_batch = RecordBatch::try_new(
            record_schema.clone(),
            vec![
                Arc::new(name.clone()) as ArrayRef,
                Arc::new(age.clone()) as ArrayRef,
                Arc::new(city.clone()) as ArrayRef,
            ],
        )
        .unwrap();

        // Format the record batch by the schema
        let formatted_batch = format_recordbatch_by_schema(schema.clone(), record_batch);

        // Assert that the formatted batch has the same schema as the input schema
        assert_eq!(formatted_batch.schema(), record_schema);

        // Assert that the formatted batch has the same number of rows as the input batch
        assert_eq!(formatted_batch.num_rows(), 3);

        // Assert that the formatted batch has the same data as the input batch
        assert_eq!(
            pretty_format_batches(&[formatted_batch])
                .unwrap()
                .to_string(),
            "+---------+-----+----------+
| name    | age | city     |
+---------+-----+----------+
| Alice   | 25  | New York |
| Bob     | 30  | London   |
| Charlie | 35  | Paris    |
+---------+-----+----------+"
        );
    }
}
