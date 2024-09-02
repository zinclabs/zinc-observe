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
    array::{Array, RecordBatch},
    compute::{filter, is_not_null},
};
use arrow_schema::Schema;
use datafusion::{
    arrow::{
        array::{ArrayRef, Float64Array},
        datatypes::DataType,
    },
    common::{downcast_value, not_impl_err, DFSchema, DataFusionError},
    error::Result,
    logical_expr::{
        function::AccumulatorArgs, Accumulator, AggregateUDFImpl, ColumnarValue, Signature,
        TypeSignature,
    },
    physical_expr_common::{
        aggregate::tdigest::TryIntoF64,
        utils::limited_convert_logical_expr_to_physical_expr_with_dfschema,
    },
    prelude::Expr,
    scalar::ScalarValue,
};

use super::PERCENTILE_EXACT;

#[derive(Debug)]
struct PercentileContUdaf(Signature);

impl PercentileContUdaf {
    pub fn new() -> Self {
        Self(Signature {
            type_signature: TypeSignature::Exact(vec![DataType::Float64, DataType::Float64]),
            volatility: datafusion::logical_expr::Volatility::Immutable,
        })
    }
}

impl AggregateUDFImpl for PercentileContUdaf {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        PERCENTILE_EXACT
    }

    fn signature(&self) -> &datafusion::logical_expr::Signature {
        &self.0
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Float64)
    }

    fn accumulator(&self, acc_args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        let percentile = validate_input_percentile_expr(&acc_args.input_exprs[1])?;
        Ok(Box::new(PercentileCont::new(Some(percentile))))
    }
}

fn validate_input_percentile_expr(expr: &Expr) -> Result<f64> {
    let lit = get_lit_value(expr)?;
    let percentile = match &lit {
        ScalarValue::Float32(Some(q)) => *q as f64,
        ScalarValue::Float64(Some(q)) => *q,
        got => {
            return not_impl_err!(
                "Percentile value for 'PERCENTILE_CONT' must be Float32 or Float64 literal (got data type {})",
                got.data_type()
            );
        }
    };
    Ok(percentile)
}

fn get_lit_value(expr: &Expr) -> Result<ScalarValue> {
    let empty_schema = Arc::new(Schema::empty());
    let empty_batch = RecordBatch::new_empty(Arc::clone(&empty_schema));
    let dfschema = DFSchema::empty();
    let expr = limited_convert_logical_expr_to_physical_expr_with_dfschema(expr, &dfschema)?;
    let result = expr.evaluate(&empty_batch)?;
    match result {
        ColumnarValue::Array(_) => Err(DataFusionError::Internal(format!(
            "The expr {:?} can't be evaluated to scalar value",
            expr
        ))),
        ColumnarValue::Scalar(scalar_value) => Ok(scalar_value),
    }
}

fn merge_sorted_arrays(existing: &mut Vec<f64>, new: &[f64]) {
    let mut result = Vec::with_capacity(existing.len() + new.len());
    let mut i = 0;
    let mut j = 0;

    while i < existing.len() && j < new.len() {
        if existing[i] <= new[j] {
            result.push(existing[i]);
            i += 1;
        } else {
            result.push(new[j]);
            j += 1;
        }
    }

    // Append remaining elements
    if i < existing.len() {
        result.extend_from_slice(&existing[i..]);
    }
    if j < new.len() {
        result.extend_from_slice(&new[j..]);
    }

    // Copy result back to existing
    existing.clear();
    existing.extend_from_slice(&result);
}

/// This accumulator will return the exact percentile value
/// from the given data
#[derive(Default, Debug)]
struct PercentileCont {
    data: Vec<f64>,
    percentile: f64,
}

impl PercentileCont {
    // Default to median
    fn new(percentile: Option<f64>) -> Self {
        Self {
            data: Vec::new(),
            percentile: percentile.unwrap_or(0.5),
        }
    }
}

// TODO(Uddhav): Add function to convert DataTypes to f64Array for this accumulator

impl Accumulator for PercentileCont {
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
        let percentile = self.percentile; // default: median
        if self.data.is_empty() {
            return Ok(ScalarValue::Float64(None));
        }

        // Calculate rank of the percentile
        let rank = percentile * (self.data.len() as f64 - 1.0);
        let lower_index = rank.floor() as usize;
        let upper_index = rank.ceil() as usize;

        if lower_index == upper_index {
            return Ok(ScalarValue::from(self.data[lower_index]));
        }

        // Calculate the delta and return the fractioned value
        let lower_value = self.data[lower_index];
        let upper_value = self.data[upper_index];
        let fraction = rank - lower_index as f64;

        Ok(ScalarValue::from(
            lower_value + (upper_value - lower_value) * fraction,
        ))
    }

    fn size(&self) -> usize {
        self.data.len()
    }

    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let mut values = Arc::clone(&values[0]);
        // Filter out null values
        if values.nulls().is_some() {
            values = filter(&values, &is_not_null(&values)?)?;
        }

        let sorted_values = &arrow::compute::sort(&values, None)?;
        let array = downcast_value!(sorted_values, Float64Array);
        let array = array
            .values()
            .iter()
            .filter_map(|v| v.try_as_f64().transpose())
            .collect::<Result<Vec<f64>>>()?;
        merge_sorted_arrays(&mut self.data, &array);
        Ok(())
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        // TODO(Uddhav): Most of the code is duplicated from update_batch.
        // Reduce duplication and improve performance.
        let mut values = Arc::clone(&states[0]);
        // Filter out null values
        if values.nulls().is_some() {
            values = filter(&values, &is_not_null(&values)?)?;
        }

        let sorted_values = &arrow::compute::sort(&values, None)?;
        let array = downcast_value!(sorted_values, Float64Array);
        let array = array
            .values()
            .iter()
            .filter_map(|v| v.try_as_f64().transpose())
            .collect::<Result<Vec<f64>>>()?;
        merge_sorted_arrays(&mut self.data, &array);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use arrow::array::{ArrayRef, RecordBatch};
    use arrow_schema::{Field, Schema};
    use datafusion::{
        common::cast::as_float64_array,
        datasource::MemTable,
        logical_expr::{Accumulator, AggregateUDF, Volatility},
        prelude::{create_udaf, SessionContext},
    };

    use super::{super::PERCENTILE_EXACT, *};

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
        let schema = Schema::new(vec![Field::new("value", DataType::Float64, false)]);
        let values: Vec<_> = NUMBERS.into_iter().map(|v| v as f64).collect();
        let batch = RecordBatch::try_new(
            Arc::new(schema.clone()),
            vec![Arc::new(Float64Array::from(values))],
        )
        .unwrap();
        let table = MemTable::try_new(Arc::new(schema), vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(table)).unwrap();
        ctx
    }

    #[test]
    fn test_exact_percentile() {
        let mut acc = PercentileCont::new(Some(0.75));
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
    async fn test_percentile_udaf() {
        let ctx = create_context();
        let percentile = 0.75;
        let sql = &format!("select percentile_cont(value, {}) from t", percentile);
        let acc_udaf = AggregateUDF::from(PercentileContUdaf::new());
        ctx.register_udaf(acc_udaf);

        let df = ctx.sql(sql).await.unwrap();
        let results = df.collect().await.unwrap();
        // downcast the array to the expected type
        let result = as_float64_array(results[0].column(0)).unwrap();

        // verify that the calculation is correct
        assert_eq!(result.value(0), 2456.5);
    }
}
