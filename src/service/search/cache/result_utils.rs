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

use std::io::{self, Read, Write};

use base64::Engine;
use config::utils::{json, time::parse_str_to_timestamp_micros_as_option};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sqlparser::{
    ast::{Expr, Function, Query, SelectItem, SetExpr, Statement},
    dialect::GenericDialect,
    parser::Parser,
};

pub fn is_aggregate_query(query: &str) -> Result<bool, sqlparser::parser::ParserError> {
    let ast = Parser::parse_sql(&GenericDialect {}, query)?;

    for statement in ast {
        if let Statement::Query(query) = statement {
            if is_aggregate_in_select(query) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn is_aggregate_in_select(query: Box<Query>) -> bool {
    if let SetExpr::Select(ref select) = *query.body {
        for select_item in &select.projection {
            if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, alias: _ } =
                select_item
            {
                if is_aggregate_expression(expr) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_aggregate_expression(expr: &Expr) -> bool {
    match expr {
        Expr::Function(Function { name, args: _, .. }) => {
            AGGREGATE_UDF_LIST.contains(&name.to_string().to_lowercase().as_str())
        }

        _ => false,
    }
}

const AGGREGATE_UDF_LIST: [&str; 7] = [
    "min",
    "max",
    "count",
    "avg",
    "sum",
    "array_agg",
    "approx_percentile_cont",
];

pub fn encode_sql_to_foldername(sql_query: &str) -> io::Result<String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(sql_query.as_bytes())?;
    let compressed_bytes = encoder.finish()?;

    Ok(base64::engine::general_purpose::STANDARD.encode(compressed_bytes))
}

pub fn decode_foldername_to_sql(folder_name: &str) -> io::Result<String> {
    let compressed_bytes = base64::engine::general_purpose::STANDARD
        .decode(folder_name.as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let mut decoder = ZlibDecoder::new(&compressed_bytes[..]);
    let mut decompressed_bytes = Vec::new();

    decoder.read_to_end(&mut decompressed_bytes)?;

    String::from_utf8(decompressed_bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn get_ts_value(ts_column: &str, record: &json::Value) -> i64 {
    match record.get(ts_column).unwrap() {
        serde_json::Value::String(ts) => {
            parse_str_to_timestamp_micros_as_option(ts.as_str()).unwrap()
        }
        serde_json::Value::Number(ts) => ts.as_i64().unwrap(),
        _ => 0_i64,
    }
}

pub fn round_down_to_nearest_minute(microseconds: i64) -> i64 {
    let microseconds_per_second = 1_000_000;
    let seconds_per_minute = 60;
    // Convert microseconds to seconds
    let total_seconds = microseconds / microseconds_per_second;
    // Find how many seconds past the last full minute
    let seconds_past_minute = total_seconds % seconds_per_minute;
    // Calculate the adjustment to round down to the nearest minute
    let adjusted_seconds = total_seconds - seconds_past_minute;
    // Convert the adjusted time back to microseconds
    adjusted_seconds * microseconds_per_second
}
