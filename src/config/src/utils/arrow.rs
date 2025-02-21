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

use arrow::array::RecordBatch;

use super::json::{Map as JsonMap, Value};

/// Converts an arrow [`RecordBatch`] into a `Vec` of Serde JSON
/// [`JsonMap`]s (objects)
pub fn record_batches_to_json_rows(
    batches: &[&RecordBatch],
) -> Result<Vec<JsonMap<String, Value>>, anyhow::Error> {
    if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
        return Ok(Vec::new());
    }

    let json_buf = Vec::with_capacity(
        batches
            .iter()
            .map(|b| b.get_array_memory_size())
            .sum::<usize>(),
    );
    let mut writer = arrow_json::ArrayWriter::new(json_buf);
    writer.write_batches(batches)?;
    writer.finish()?;
    let json_data = writer.into_inner();
    let ret: Vec<JsonMap<String, Value>> = serde_json::from_reader(json_data.as_slice())?;

    // This effects other function, comment it for now
    //
    // Hack for uint64, because Chrome V8 does not support uint64
    // for field in schema.fields() {
    //     if field.data_type() == &DataType::UInt64 {
    //         for row in ret.iter_mut() {
    //             if let Some(val) = row.get_mut(field.name()) {
    //                 if val.is_u64() {
    //                     *val = Value::String(val.to_string());
    //                 }
    //             }
    //         }
    //     }
    // }

    Ok(ret)
}
