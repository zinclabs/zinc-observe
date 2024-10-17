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

pub use serde_json::{
    from_slice, from_str, from_value, json, to_string, to_value, to_vec, Error, Map, Number, Value,
};

pub fn get_float_value(val: &Value) -> f64 {
    match val {
        Value::String(v) => v.parse::<f64>().unwrap_or(0.0),
        Value::Number(v) => v.as_f64().unwrap_or(0.0),
        Value::Bool(v) => {
            if *v {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

pub fn get_int_value(val: &Value) -> i64 {
    match val {
        Value::String(v) => v.parse::<i64>().unwrap_or(0),
        Value::Number(v) => v.as_i64().unwrap_or(0),
        Value::Bool(v) => {
            if *v {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

pub fn get_uint_value(val: &Value) -> u64 {
    match val {
        Value::String(v) => v.parse::<u64>().unwrap_or(0),
        Value::Number(v) => v.as_u64().unwrap_or(0),
        Value::Bool(v) => {
            if *v {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

pub fn get_bool_value(val: &Value) -> bool {
    match val {
        Value::String(v) => v.parse::<bool>().unwrap_or(false),
        Value::Number(v) => v.as_f64().unwrap_or(0.0) > 0.0,
        Value::Bool(v) => *v,
        _ => false,
    }
}

pub fn get_string_value(value: &Value) -> String {
    if value.is_string() {
        value.as_str().unwrap_or_default().to_string()
    } else if value.is_i64() {
        value.as_i64().unwrap_or_default().to_string()
    } else if value.is_u64() {
        value.as_u64().unwrap_or_default().to_string()
    } else if value.is_f64() {
        value.as_f64().unwrap_or_default().to_string()
    } else if value.is_boolean() {
        value.as_bool().unwrap_or_default().to_string()
    } else if value.is_null() {
        "".to_string()
    } else {
        value.to_string()
    }
}

pub fn pickup_string_value(val: Value) -> String {
    match val {
        Value::String(v) => v,
        Value::Number(v) => v.to_string(),
        Value::Bool(v) => v.to_string(),
        _ => val.to_string(),
    }
}

pub fn estimate_json_bytes(val: &Value) -> usize {
    let mut size = 0;
    match val {
        Value::Object(map) => {
            // {?} extra 2
            size += 2;
            for (k, v) in map {
                if k == crate::ORIGINAL_DATA_COL_NAME {
                    continue;
                }
                // "key":?, extra 4 bytes
                size += k.len() + estimate_json_bytes(v) + 4;
            }
            // remove ',' for last item
            if !map.is_empty() {
                size -= 1;
            }
        }
        Value::Array(arr) => {
            // []=>2 [?]=>2 [?,?] extra 1+n
            size += std::cmp::max(arr.len(), 1) + 1;
            for v in arr {
                size += estimate_json_bytes(v);
            }
        }
        Value::String(s) => {
            // "?"=>2
            size += s.len() + 2;
        }
        Value::Number(n) => {
            size += n.to_string().len();
        }
        Value::Bool(b) => {
            // true for 4 bytes, false for 5 bytes
            size += if *b { 4 } else { 5 };
        }
        Value::Null => {
            size += 4;
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_json_bytes() {
        let json = r#"{"a":null,"b":true,"c":false,"d":{"a":"b","c":true,"d":false,"e":123456},"e":[""],"f":["a"],"g":["a","b"],"h":"bcdef","i":{},"j":{"ok":"yes"}}"#;
        let val: Value = from_str(json).unwrap();
        assert_eq!(estimate_json_bytes(&val), json.len());
    }
}
