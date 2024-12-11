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
    io::{Cursor, Read},
    sync::Arc,
};

use arrow::{array::Int64Array, record_batch::RecordBatch};
use arrow_schema::Schema;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use config::utils::record_batch_ext::{convert_json_to_record_batch, RecordBatchExt};
use snafu::ResultExt;

use crate::errors::*;

pub struct Entry {
    pub stream: Arc<str>,
    pub schema: Option<Arc<Schema>>,
    pub schema_key: Arc<str>,
    pub partition_key: Arc<str>, // 2023/12/18/00/country=US/state=CA
    pub data: Vec<Arc<serde_json::Value>>,
    pub data_size: usize,
}

impl Entry {
    pub fn new() -> Self {
        Self {
            stream: "".into(),
            schema: None,
            schema_key: "".into(),
            partition_key: "".into(),
            data: Vec::new(),
            data_size: 0,
        }
    }
    pub fn into_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(4096);
        let stream = self.stream.as_bytes();
        let schema_key = self.schema_key.as_bytes();
        let partition_key = self.partition_key.as_bytes();
        let data = serde_json::to_vec(&self.data).context(JSONSerializationSnafu)?;
        let data_size = data.len();
        self.data_size = data_size; // reset data size
        buf.write_u16::<BigEndian>(stream.len() as u16)
            .context(WriteDataSnafu)?;
        buf.extend_from_slice(stream);
        buf.write_u16::<BigEndian>(schema_key.len() as u16)
            .context(WriteDataSnafu)?;
        buf.extend_from_slice(schema_key);
        buf.write_u16::<BigEndian>(partition_key.len() as u16)
            .context(WriteDataSnafu)?;
        buf.extend_from_slice(partition_key);
        buf.write_u32::<BigEndian>(data_size as u32)
            .context(WriteDataSnafu)?;
        buf.extend_from_slice(&data);
        Ok(buf)
    }

    pub fn from_bytes(value: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(value);
        let stream_len = cursor.read_u16::<BigEndian>().context(ReadDataSnafu)?;
        let mut stream = vec![0; stream_len as usize];
        cursor.read_exact(&mut stream).context(ReadDataSnafu)?;
        let stream = String::from_utf8(stream).context(FromUtf8Snafu)?;
        let schema_key_len = cursor.read_u16::<BigEndian>().context(ReadDataSnafu)?;
        let mut schema_key = vec![0; schema_key_len as usize];
        cursor.read_exact(&mut schema_key).context(ReadDataSnafu)?;
        let schema_key = String::from_utf8(schema_key).context(FromUtf8Snafu)?;
        let partition_key_len = cursor.read_u16::<BigEndian>().context(ReadDataSnafu)?;
        let mut partition_key = vec![0; partition_key_len as usize];
        cursor
            .read_exact(&mut partition_key)
            .context(ReadDataSnafu)?;
        let partition_key = String::from_utf8(partition_key).context(FromUtf8Snafu)?;
        let data_len = cursor.read_u32::<BigEndian>().context(ReadDataSnafu)?;
        let mut data = vec![0; data_len as usize];
        cursor.read_exact(&mut data).context(ReadDataSnafu)?;
        let data = serde_json::from_slice(&data).context(JSONSerializationSnafu)?;
        Ok(Self {
            stream: stream.into(),
            schema: None,
            schema_key: schema_key.into(),
            partition_key: partition_key.into(),
            data,
            data_size: data_len as usize,
        })
    }

    pub fn into_batch(
        &self,
        stream_type: Arc<str>,
        schema: Arc<Schema>,
    ) -> Result<Arc<RecordBatchEntry>> {
        let batch =
            convert_json_to_record_batch(&schema, &self.data).context(ArrowJsonEncodeSnafu)?;

        let arrow_size = batch.size();
        Ok(RecordBatchEntry::new(
            stream_type,
            batch,
            self.data_size,
            arrow_size,
        ))
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RecordBatchEntry {
    pub data: RecordBatch,
    pub data_json_size: usize,
    pub data_arrow_size: usize,
    pub min_ts: i64,
    pub max_ts: i64,
}

impl RecordBatchEntry {
    pub fn new(
        stream_type: Arc<str>,
        data: RecordBatch,
        data_json_size: usize,
        data_arrow_size: usize,
    ) -> Arc<RecordBatchEntry> {
        let (min_ts, max_ts) = if stream_type == Arc::from("index") {
            pop_time_range(&data, Some("min_ts"), Some("max_ts"))
        } else {
            pop_time_range(&data, None, None)
        };
        Arc::new(Self {
            data,
            data_json_size,
            data_arrow_size,
            min_ts,
            max_ts,
        })
    }
}

fn pop_time_range(
    batch: &RecordBatch,
    min_field: Option<&str>,
    max_field: Option<&str>,
) -> (i64, i64) {
    let mut min_ts = 0;
    let mut max_ts = 0;
    let time_field = config::get_config().common.column_timestamp.to_string();
    let min_field = min_field.unwrap_or(time_field.as_str());
    let max_field = max_field.unwrap_or(time_field.as_str());
    if min_field == max_field {
        let Some(col) = batch.column_by_name(min_field) else {
            return (0, 0);
        };
        let Some(col) = col.as_any().downcast_ref::<Int64Array>() else {
            return (0, 0);
        };
        for v in col.values() {
            if min_ts == 0 || min_ts > *v {
                min_ts = *v;
            }
            if max_ts < *v {
                max_ts = *v;
            }
        }
        return (min_ts, max_ts);
    }

    // min_ts
    let Some(col) = batch.column_by_name(min_field) else {
        return (0, 0);
    };
    let Some(col) = col.as_any().downcast_ref::<Int64Array>() else {
        return (0, 0);
    };
    for v in col.values() {
        if min_ts == 0 || min_ts > *v {
            min_ts = *v;
        }
    }

    // max_ts
    let Some(col) = batch.column_by_name(max_field) else {
        return (0, 0);
    };
    let Some(col) = col.as_any().downcast_ref::<Int64Array>() else {
        return (0, 0);
    };
    for v in col.values() {
        if max_ts < *v {
            max_ts = *v;
        }
    }

    (min_ts, max_ts)
}

#[derive(Default)]
pub struct PersistStat {
    pub json_size: i64,
    pub arrow_size: usize,
    pub file_num: usize,
    pub batch_num: usize,
}

impl std::ops::Add for PersistStat {
    type Output = PersistStat;

    fn add(self, other: PersistStat) -> PersistStat {
        PersistStat {
            json_size: self.json_size + other.json_size,
            arrow_size: self.arrow_size + other.arrow_size,
            file_num: self.file_num + other.file_num,
            batch_num: self.batch_num + other.batch_num,
        }
    }
}

impl std::ops::AddAssign for PersistStat {
    fn add_assign(&mut self, other: PersistStat) {
        self.json_size += other.json_size;
        self.arrow_size += other.arrow_size;
        self.file_num += other.file_num;
        self.batch_num += other.batch_num;
    }
}
