// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! [`ScalarUDFImpl`] definitions for array_resize function.

use std::{any::Any, sync::Arc};

use arrow::array::{Capacities, MutableArrayData};
use arrow_array::{ArrayRef, GenericListArray, Int64Array, OffsetSizeTrait};
use arrow_buffer::{ArrowNativeType, OffsetBuffer};
use arrow_schema::{
    DataType,
    DataType::{FixedSizeList, LargeList, List},
    FieldRef,
};
use datafusion_common::{
    cast::{as_int64_array, as_large_list_array, as_list_array},
    exec_err, internal_datafusion_err, Result, ScalarValue,
};
use datafusion_expr::{
    ColumnarValue, ScalarFunctionImplementation, ScalarUDFImpl, Signature, Volatility,
};

make_udf_expr_and_func!(
    ArrayResize,
    array_resize,
    array size value,
    "returns an array with the specified size filled with the given value.",
    array_resize_udf
);

#[derive(Debug)]
pub(super) struct ArrayResize {
    signature: Signature,
    aliases: Vec<String>,
}

impl ArrayResize {
    pub fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
            aliases: vec!["list_resize".to_string()],
        }
    }
}

impl ScalarUDFImpl for ArrayResize {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        "array_resize"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType> {
        match &arg_types[0] {
            List(field) | FixedSizeList(field, _) => Ok(List(field.clone())),
            LargeList(field) => Ok(LargeList(field.clone())),
            _ => exec_err!("Not reachable, data_type should be List, LargeList or FixedSizeList"),
        }
    }

    fn invoke(&self, args: &[ColumnarValue]) -> Result<ColumnarValue> {
        make_scalar_function(array_resize_inner)(args)
    }

    fn aliases(&self) -> &[String] {
        &self.aliases
    }
}

/// array_resize SQL function
pub(crate) fn array_resize_inner(arg: &[ArrayRef]) -> Result<ArrayRef> {
    if arg.len() < 2 || arg.len() > 3 {
        return exec_err!("array_resize needs two or three arguments");
    }

    let new_len = as_int64_array(&arg[1])?;
    let new_element = if arg.len() == 3 {
        Some(arg[2].clone())
    } else {
        None
    };

    match &arg[0].data_type() {
        List(field) => {
            let array = as_list_array(&arg[0])?;
            general_list_resize::<i32>(array, new_len, field, new_element)
        }
        LargeList(field) => {
            let array = as_large_list_array(&arg[0])?;
            general_list_resize::<i64>(array, new_len, field, new_element)
        }
        array_type => exec_err!("array_resize does not support type '{array_type:?}'."),
    }
}

/// array_resize keep the original array and append the default element to the end
fn general_list_resize<O: OffsetSizeTrait + TryInto<i64>>(
    array: &GenericListArray<O>,
    count_array: &Int64Array,
    field: &FieldRef,
    default_element: Option<ArrayRef>,
) -> Result<ArrayRef> {
    let data_type = array.value_type();

    let values = array.values();
    let original_data = values.to_data();

    // create default element array
    let default_element = if let Some(default_element) = default_element {
        default_element
    } else {
        let null_scalar = ScalarValue::try_from(&data_type)?;
        null_scalar.to_array_of_size(original_data.len())?
    };
    let default_value_data = default_element.to_data();

    // create a mutable array to store the original data
    let capacity = Capacities::Array(original_data.len() + default_value_data.len());
    let mut offsets = vec![O::usize_as(0)];
    let mut mutable = MutableArrayData::with_capacities(
        vec![&original_data, &default_value_data],
        false,
        capacity,
    );

    for (row_index, offset_window) in array.offsets().windows(2).enumerate() {
        let count = count_array.value(row_index).to_usize().ok_or_else(|| {
            internal_datafusion_err!("array_resize: failed to convert size to usize")
        })?;
        let count = O::usize_as(count);
        let start = offset_window[0];
        if start + count > offset_window[1] {
            let extra_count = (start + count - offset_window[1]).try_into().map_err(|_| {
                internal_datafusion_err!("array_resize: failed to convert size to i64")
            })?;
            let end = offset_window[1];
            mutable.extend(0, (start).to_usize().unwrap(), (end).to_usize().unwrap());
            // append default element
            for _ in 0..extra_count {
                mutable.extend(1, row_index, row_index + 1);
            }
        } else {
            let end = start + count;
            mutable.extend(0, (start).to_usize().unwrap(), (end).to_usize().unwrap());
        };
        offsets.push(offsets[row_index] + count);
    }

    let data = mutable.freeze();
    Ok(Arc::new(GenericListArray::<O>::try_new(
        field.clone(),
        OffsetBuffer::<O>::new(offsets.into()),
        arrow_array::make_array(data),
        None,
    )?))
}

/// array function wrapper that differentiates between scalar (length 1) and array.
pub(crate) fn make_scalar_function<F>(inner: F) -> ScalarFunctionImplementation
where
    F: Fn(&[ArrayRef]) -> Result<ArrayRef> + Sync + Send + 'static,
{
    Arc::new(move |args: &[ColumnarValue]| {
        // first, identify if any of the arguments is an Array. If yes, store its `len`,
        // as any scalar will need to be converted to an array of len `len`.
        let len = args
            .iter()
            .fold(Option::<usize>::None, |acc, arg| match arg {
                ColumnarValue::Scalar(_) => acc,
                ColumnarValue::Array(a) => Some(a.len()),
            });

        let is_scalar = len.is_none();

        let args = ColumnarValue::values_to_arrays(args)?;

        let result = (inner)(&args);

        if is_scalar {
            // If all inputs are scalar, keeps output as scalar
            let result = result.and_then(|arr| ScalarValue::try_from_array(&arr, 0));
            result.map(ColumnarValue::Scalar)
        } else {
            result.map(ColumnarValue::Array)
        }
    })
}

#[cfg(test)]
mod tests {
    use arrow_array::ListArray;
    use arrow_schema::Field;
    use datafusion::{
        arrow::{
            array::{Int64Array, StringArray},
            datatypes::Schema,
            record_batch::RecordBatch,
        },
        assert_batches_eq,
        datasource::MemTable,
        prelude::SessionContext,
    };

    use super::*;

    #[tokio::test]
    async fn test_array_resize() {
        // Example data setup
        let string_data = vec!["a", "b", "c", "d"];
        let new_length = vec![2]; // New desired length
        let padding_element = vec!["z"]; // Element to use for padding if necessary

        let string_array = StringArray::from(string_data.clone());

        // Create an in-memory table
        let schema = Arc::new(Schema::new(vec![Field::new(
            "array",
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        )]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(ListArray::from(vec![(
                MapArray::from(string_array.to_data()),
                None,
            )]))],
        )
        .unwrap();

        let ctx = SessionContext::new();
        ctx.register_table(
            "t",
            Arc::new(MemTable::try_new(schema, vec![vec![batch]]).unwrap()),
        )
        .unwrap();

        // Registering the UDF
        ctx.register_udf(ArrayResize::new().to_udf());

        // Query to resize the array
        let sql = "SELECT array_resize(array, 2, padding_element) AS resized FROM t";
        let df = ctx.sql(sql).await.unwrap();
        let results = df.collect().await.unwrap();

        // Expected output
        let expected = vec![
            "+---------+",
            "| resized |",
            "+---------+",
            "| [a, b]  |",
            "+---------+",
        ];

        assert_batches_eq!(expected, &results);
    }
}
