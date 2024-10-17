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

use std::{
    borrow::Borrow,
    io::{BufRead, Seek},
    sync::Arc,
};

use arrow_json::reader;
use arrow_schema::{ArrowError, DataType, Field, Schema};
use serde_json::{Map, Value};

use crate::{get_config, meta::stream::StreamType, FxIndexMap};

pub fn infer_json_schema<R: BufRead>(
    reader: R,
    max_read_records: Option<usize>,
    stream_type: StreamType,
) -> Result<Schema, ArrowError> {
    let (schema, _) = reader::infer_json_schema(reader, max_read_records)?;
    Ok(fix_schema(schema, stream_type))
}

pub fn infer_json_schema_from_seekable<R: BufRead + Seek>(
    reader: R,
    max_read_records: Option<usize>,
    stream_type: StreamType,
) -> Result<Schema, ArrowError> {
    let (schema, _) = reader::infer_json_schema_from_seekable(reader, max_read_records)?;
    Ok(fix_schema(schema, stream_type))
}

pub fn infer_json_schema_from_map<I, V>(
    value_iter: I,
    stream_type: impl Into<StreamType>,
) -> Result<Schema, ArrowError>
where
    I: Iterator<Item = V>,
    V: Borrow<Map<String, Value>>,
{
    let mut fields = None;
    for value in value_iter {
        if fields.is_none() {
            fields = Some(FxIndexMap::with_capacity_and_hasher(
                value.borrow().len(),
                Default::default(),
            ));
        }
        infer_json_schema_from_object(fields.as_mut().unwrap(), value.borrow())?;
    }
    let fields = fields.unwrap_or_default();
    let fields = fields
        .into_iter()
        .map(|(_, field)| field)
        .collect::<Vec<_>>();
    Ok(fix_schema(Schema::new(fields), stream_type.into()))
}

pub fn infer_json_schema_from_values<I, V>(
    value_iter: I,
    stream_type: impl Into<StreamType>,
) -> Result<Schema, ArrowError>
where
    I: Iterator<Item = V>,
    V: Borrow<Value>,
{
    let mut fields = None;
    for value in value_iter {
        match value.borrow() {
            Value::Object(v) => {
                if fields.is_none() {
                    fields = Some(FxIndexMap::with_capacity_and_hasher(
                        v.len(),
                        Default::default(),
                    ));
                }
                infer_json_schema_from_object(fields.as_mut().unwrap(), v)?;
            }
            _ => {
                return Err(ArrowError::SchemaError(
                    "Cannot infer schema from non-object value".to_string(),
                ));
            }
        }
    }
    let fields = fields.unwrap_or_default();
    let fields = fields
        .into_iter()
        .map(|(_, field)| field)
        .collect::<Vec<_>>();
    Ok(fix_schema(Schema::new(fields), stream_type.into()))
}

fn infer_json_schema_from_object(
    fields: &mut FxIndexMap<String, Field>,
    value: &Map<String, Value>,
) -> Result<(), ArrowError> {
    for (key, value) in value.iter() {
        match value {
            Value::String(_) => {
                convert_data_type(fields, key, DataType::Utf8)?;
            }
            Value::Number(v) => {
                if v.is_i64() {
                    convert_data_type(fields, key, DataType::Int64)?;
                } else if v.is_u64() {
                    convert_data_type(fields, key, DataType::UInt64)?;
                } else if v.is_f64() {
                    convert_data_type(fields, key, DataType::Float64)?;
                } else {
                    return Err(ArrowError::SchemaError(
                        "Cannot infer schema from non-basic-number type value".to_string(),
                    ));
                }
            }
            Value::Bool(_) => {
                convert_data_type(fields, key, DataType::Boolean)?;
            }
            Value::Null => {}
            _ => {
                return Err(ArrowError::SchemaError(
                    "Cannot infer schema from non-basic type value".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn convert_data_type(
    fields: &mut FxIndexMap<String, Field>,
    key: &str,
    data_type: DataType,
) -> Result<(), ArrowError> {
    let Some(f) = fields.get(key) else {
        fields.insert(key.to_string(), Field::new(key, data_type, true));
        return Ok(());
    };
    let f_type = f.data_type();
    if f_type == &data_type {
        return Ok(());
    }
    match (f_type, &data_type) {
        (DataType::Utf8, _) => {}
        (DataType::Float64, DataType::UInt64)
        | (DataType::Float64, DataType::Int64)
        | (DataType::Float64, DataType::Boolean) => {}
        (DataType::Int64, DataType::UInt64)
        | (DataType::Int64, DataType::Float64)
        | (DataType::Int64, DataType::Utf8) => {
            fields.insert(key.to_string(), Field::new(key, data_type, true));
        }
        (DataType::Int64, DataType::Boolean) | (DataType::UInt64, DataType::Boolean) => {}
        (DataType::UInt64, DataType::Float64) | (DataType::UInt64, DataType::Utf8) => {
            fields.insert(key.to_string(), Field::new(key, data_type, true));
        }
        (DataType::Float64, DataType::Utf8) => {
            fields.insert(key.to_string(), Field::new(key, data_type, true));
        }

        (DataType::Boolean, _) => {
            fields.insert(key.to_string(), Field::new(key, data_type, true));
        }
        _ => {
            return Err(ArrowError::SchemaError(format!(
                "Cannot infer schema from conflicting types: {:?} and {:?}",
                f_type, data_type
            )));
        }
    }
    Ok(())
}

/// Fix the schema to ensure that the start_time and end_time fields are always present with uint64
/// and that null fields are removed and sort the fields by name.
fn fix_schema(schema: Schema, stream_type: StreamType) -> Schema {
    let mut fields = if stream_type == StreamType::Traces {
        itertools::chain(
            schema.fields().into_iter().filter_map(|f| {
                if f.name() != "start_time"
                    && f.name() != "end_time"
                    && f.data_type() != &DataType::Null
                {
                    Some(f.to_owned())
                } else {
                    None
                }
            }),
            vec![
                Arc::new(Field::new("start_time", DataType::UInt64, true)),
                Arc::new(Field::new("end_time", DataType::UInt64, true)),
            ],
        )
        .collect::<Vec<_>>()
    } else {
        schema
            .fields()
            .into_iter()
            .filter_map(|f| {
                if f.data_type() == &DataType::Null {
                    None
                } else {
                    Some(f.to_owned())
                }
            })
            .collect::<Vec<_>>()
    };
    let cfg = get_config();
    fields = fields
        .into_iter()
        .map(|x| {
            if x.name() == &cfg.common.column_timestamp {
                Arc::new(Field::new(
                    cfg.common.column_timestamp.clone(),
                    DataType::Int64,
                    false,
                ))
            } else {
                x
            }
        })
        .collect::<Vec<_>>();
    fields.sort_by(|a, b| a.name().cmp(b.name()));
    Schema::new(fields)
}
