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

use std::{fmt::Formatter, sync::Arc};

use arrow::array::{
    Array, AsArray, Float32Array, Int16Array, Int32Array, Int64Array, Int8Array, RecordBatch,
    UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use arrow_schema::{Field, Schema};
use datafusion::{
    arrow::{
        array::{ArrayRef, Float64Array},
        datatypes::DataType,
    },
    common::{downcast_value, internal_err, not_impl_err, plan_err, DataFusionError},
    error::Result,
    logical_expr::{
        function::{AccumulatorArgs, StateFieldsArgs},
        utils::format_state_name,
        Accumulator, AggregateUDFImpl, ColumnarValue, Signature, TypeSignature, Volatility,
    },
    physical_plan::PhysicalExpr,
    scalar::ScalarValue,
};
use datafusion_functions_aggregate_common::tdigest::TryIntoF64;

use super::NUMERICS;

const PERCENTILE_CONT: &str = "percentile_cont";

pub(crate) struct PercentileCont(Signature);

impl PercentileCont {
    pub fn new() -> Self {
        let mut variants = Vec::with_capacity(NUMERICS.len());
        // Accept any numeric value paired with a float64 percentile
        for num in NUMERICS {
            variants.push(TypeSignature::Exact(vec![num.clone(), DataType::Float64]));
        }
        Self(Signature::one_of(variants, Volatility::Immutable))
    }
}

impl std::fmt::Debug for PercentileCont {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("PercentileCont")
            .field("name", &self.name())
            .field("signature", &self.0)
            .finish()
    }
}

impl Default for PercentileCont {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregateUDFImpl for PercentileCont {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        PERCENTILE_CONT
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.0
    }

    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType> {
        if !arg_types[0].is_numeric() {
            return plan_err!("percentile_cont requires numeric input types");
        }
        Ok(arg_types[0].clone())
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<Field>> {
        // Intermediate state is a list of the elements we have collected so far
        let field = Field::new("item", DataType::Float64, true);
        let state_name = "percentile_cont";
        Ok(vec![Field::new(
            format_state_name(args.name, state_name),
            DataType::List(Arc::new(field)),
            true,
        )])
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        let percentile = validate_input_percentile_expr(&args.exprs[1])?;
        let data_type = args.exprs[0].data_type(args.schema)?;
        let accumulator: PercentileContAccumulator = match data_type {
            t @ (DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64) => PercentileContAccumulator::new(percentile, t.clone()),
            other => {
                return not_impl_err!(
                    "Support for 'PERCENTILE_CONT' for data type {other} is not implemented"
                );
            }
        };
        Ok(Box::new(accumulator))
    }
}

fn validate_input_percentile_expr(expr: &Arc<dyn PhysicalExpr>) -> Result<f64> {
    let percentile = match get_scalar_value(expr)? {
        ScalarValue::Float32(Some(value)) => value as f64,
        ScalarValue::Float64(Some(value)) => value,
        sv => {
            return not_impl_err!(
                "Percentile value for 'PERCENTILE_CONT' must be Float32 or Float64 literal (got data type {})",
                sv.data_type()
            );
        }
    };

    // Ensure the percentile is between 0 and 1.
    if !(0.0..=1.0).contains(&percentile) {
        return plan_err!(
            "Percentile value must be between 0.0 and 1.0 inclusive, {percentile} is invalid"
        );
    }
    Ok(percentile)
}

fn get_scalar_value(expr: &Arc<dyn PhysicalExpr>) -> Result<ScalarValue> {
    let empty_schema = Arc::new(Schema::empty());
    let batch = RecordBatch::new_empty(Arc::clone(&empty_schema));
    if let ColumnarValue::Scalar(s) = expr.evaluate(&batch)? {
        Ok(s)
    } else {
        internal_err!("Didn't expect ColumnarValue::Array")
    }
}

/// This accumulator will return the exact percentile value from the given data
struct PercentileContAccumulator {
    data: Vec<f64>,
    percentile: f64,
    return_type: DataType,
}

impl std::fmt::Debug for PercentileContAccumulator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PercentileContAccumulator({}, {})",
            self.return_type, self.percentile
        )
    }
}

impl PercentileContAccumulator {
    // Default to median
    fn new(percentile: f64, return_type: DataType) -> Self {
        Self {
            data: Vec::new(),
            percentile,
            return_type,
        }
    }

    // public for approx_percentile_cont_with_weight
    pub fn convert_to_float(values: &ArrayRef) -> Result<Vec<f64>> {
        match values.data_type() {
            DataType::Float64 => {
                let array = downcast_value!(values, Float64Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::Float32 => {
                let array = downcast_value!(values, Float32Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::Int64 => {
                let array = downcast_value!(values, Int64Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::Int32 => {
                let array = downcast_value!(values, Int32Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::Int16 => {
                let array = downcast_value!(values, Int16Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::Int8 => {
                let array = downcast_value!(values, Int8Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::UInt64 => {
                let array = downcast_value!(values, UInt64Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::UInt32 => {
                let array = downcast_value!(values, UInt32Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::UInt16 => {
                let array = downcast_value!(values, UInt16Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            DataType::UInt8 => {
                let array = downcast_value!(values, UInt8Array);
                Ok(array
                    .values()
                    .iter()
                    .filter_map(|v| v.try_as_f64().transpose())
                    .collect::<Result<Vec<_>>>()?)
            }
            e => internal_err!("PERCENTILE_CONT is not expected to receive the type {e:?}"),
        }
    }
}

impl Accumulator for PercentileContAccumulator {
    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![ScalarValue::List(ScalarValue::new_list_nullable(
            &self
                .data
                .iter()
                .map(|v| ScalarValue::Float64(Some(*v)))
                .collect::<Vec<ScalarValue>>(),
            &DataType::Float64,
        ))])
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        // Sort the data
        self.data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let percentile = self.percentile;
        if self.data.is_empty() {
            return Ok(match &self.return_type {
                DataType::Int8 => ScalarValue::Int8(None),
                DataType::Int16 => ScalarValue::Int16(None),
                DataType::Int32 => ScalarValue::Int32(None),
                DataType::Int64 => ScalarValue::Int64(None),
                DataType::UInt8 => ScalarValue::UInt8(None),
                DataType::UInt16 => ScalarValue::UInt16(None),
                DataType::UInt32 => ScalarValue::UInt32(None),
                DataType::UInt64 => ScalarValue::UInt64(None),
                DataType::Float32 => ScalarValue::Float32(None),
                DataType::Float64 => ScalarValue::Float64(None),
                v => unreachable!("unexpected return type {:?}", v),
            });
        }

        // Calculate rank of the percentile
        let rank = percentile * (self.data.len() as f64 - 1.0);
        let lower_index = rank.floor() as usize;
        let upper_index = rank.ceil() as usize;

        let q = if lower_index == upper_index {
            self.data[lower_index]
        } else {
            // Calculate the delta and return the fractioned value
            let lower_value = self.data[lower_index];
            let upper_value = self.data[upper_index];
            let fraction = rank - lower_index as f64;
            lower_value + (upper_value - lower_value) * fraction
        };
        Ok(match &self.return_type {
            DataType::Int8 => ScalarValue::Int8(Some(q as i8)),
            DataType::Int16 => ScalarValue::Int16(Some(q as i16)),
            DataType::Int32 => ScalarValue::Int32(Some(q as i32)),
            DataType::Int64 => ScalarValue::Int64(Some(q as i64)),
            DataType::UInt8 => ScalarValue::UInt8(Some(q as u8)),
            DataType::UInt16 => ScalarValue::UInt16(Some(q as u16)),
            DataType::UInt32 => ScalarValue::UInt32(Some(q as u32)),
            DataType::UInt64 => ScalarValue::UInt64(Some(q as u64)),
            DataType::Float32 => ScalarValue::Float32(Some(q as f32)),
            DataType::Float64 => ScalarValue::Float64(Some(q)),
            v => unreachable!("unexpected return type {:?}", v),
        })
    }

    fn size(&self) -> usize {
        self.data.len()
    }

    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let values = PercentileContAccumulator::convert_to_float(&values[0])?;
        self.data.reserve(values.len());
        self.data.extend(values);
        Ok(())
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        if states.is_empty() {
            return Ok(());
        }

        let array = states[0].as_list::<i32>();
        for v in array.iter().flatten() {
            self.update_batch(&[v])?
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, RecordBatch};
    use arrow_schema::{Field, Schema};
    use datafusion::{
        common::cast::{as_float64_array, as_uint32_array},
        datasource::MemTable,
        logical_expr::{Accumulator, AggregateUDF},
        prelude::SessionContext,
    };

    use super::*;

    // list of numbers to test
    const NUMBERS: [u16; 92] = [
        2973, 1018, 5898, 52, 17296, 943, 1363, 1075, 1176, 2257, 1263, 1132, 1749, 967, 1737,
        1380, 1506, 2021, 1341, 3240, 1430, 1632, 2127, 2547, 1346, 1249, 11700, 1491, 1202, 8444,
        916, 1132, 1417, 2527, 1163, 15003, 1299, 2073, 1523, 3783, 2170, 6640, 1493, 981, 1926,
        2066, 2621, 1062, 2108, 852, 3634, 1322, 2433, 1015, 2271, 1819, 2978, 1635, 2102, 2847,
        1208, 3896, 2603, 1174, 8444, 1846, 3291, 1, 1638, 1647, 1101, 1602, 1558, 808, 734, 16227,
        1304, 2219, 1163, 1135, 1429, 2778, 1439, 2553, 1480, 1129, 2054, 1203, 3653, 679, 1591,
        1811,
    ];

    fn create_context() -> SessionContext {
        let ctx = SessionContext::new();
        // Create two fields with the same data but different data types
        let schema = Schema::new(vec![
            Field::new("value_float", DataType::Float64, false),
            Field::new("value_uint", DataType::UInt32, false),
        ]);
        let values_float: Vec<_> = NUMBERS.into_iter().map(|v| v as f64).collect();
        let values_uint: Vec<_> = NUMBERS.into_iter().map(|v| v as u32).collect();
        let batch = RecordBatch::try_new(
            Arc::new(schema.clone()),
            vec![
                Arc::new(Float64Array::from(values_float)),
                Arc::new(UInt32Array::from(values_uint)),
            ],
        )
        .unwrap();
        let table = MemTable::try_new(Arc::new(schema), vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(table)).unwrap();
        ctx
    }

    #[test]
    fn test_percentile_cont() {
        let mut acc = PercentileContAccumulator::new(0.75, DataType::Float64);
        let values: Vec<_> = NUMBERS.into_iter().map(|v| v as f64).collect();
        // Convert values to arrayref
        let values = vec![arrow::array::Float64Array::from(values)]
            .into_iter()
            .map(|v| Arc::new(v) as ArrayRef)
            .collect::<Vec<_>>();
        acc.update_batch(&values).unwrap();

        // Check the result
        assert_eq!(acc.evaluate().unwrap(), ScalarValue::Float64(Some(2456.5)));
    }

    #[tokio::test]
    async fn test_percentile_cont_udaf() {
        let ctx = create_context();
        let percentile = 0.75;
        let sql_float_field =
            &format!("select percentile_cont(value_float, {}) from t", percentile);
        let sql_uint_field = &format!("select percentile_cont(value_uint, {}) from t", percentile);
        let acc_udaf = AggregateUDF::from(PercentileCont::new());
        ctx.register_udaf(acc_udaf);

        let df = ctx.sql(sql_float_field).await.unwrap();
        let results = df.collect().await.unwrap();
        // downcast the array to the expected type
        let result = as_float64_array(results[0].column(0)).unwrap();
        assert_eq!(result.value(0), 2456.5);

        let df = ctx.sql(sql_uint_field).await.unwrap();
        let results = df.collect().await.unwrap();
        // downcast the array to the expected type
        let result = as_uint32_array(results[0].column(0)).unwrap();
        assert_eq!(result.value(0), 2456);
    }
}
