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

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use once_cell::sync::Lazy;

use crate::utils::json;

// BASE_TIME is the time when the timestamp is 1 year, used to check a timestamp
// is in seconds or milliseconds or microseconds or nanoseconds
pub static BASE_TIME: Lazy<DateTime<Utc>> =
    Lazy::new(|| Utc.with_ymd_and_hms(1971, 1, 1, 0, 0, 0).unwrap());

// check format: 1s, 1m, 1h, 1d, 1w, 1y, 1h10m30s
static TIME_UNITS: [(char, u64); 7] = [
    ('!', 1), // ms
    ('s', 1000),
    ('m', 60 * 1000),
    ('h', 3600 * 1000),
    ('d', 24 * 3600 * 1000),
    ('w', 7 * 24 * 3600 * 1000),
    ('y', 365 * 24 * 3600 * 1000),
];

#[inline(always)]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

#[inline(always)]
pub fn now_micros() -> i64 {
    Utc::now().timestamp_micros()
}

#[inline(always)]
pub fn parse_i64_to_timestamp_micros(v: i64) -> i64 {
    if v == 0 {
        return Utc::now().timestamp_micros();
    }
    let mut duration = v;
    if duration > BASE_TIME.timestamp_nanos_opt().unwrap_or_default() {
        // nanoseconds
        duration /= 1000;
    } else if duration > BASE_TIME.timestamp_micros() {
        // microseconds
        // noop
    } else if duration > BASE_TIME.timestamp_millis() {
        // milliseconds
        duration *= 1000;
    } else {
        // seconds
        duration *= 1_000_000;
    }
    duration
}

#[inline(always)]
pub fn parse_str_to_timestamp_micros(v: &str) -> Result<i64, anyhow::Error> {
    match v.parse() {
        Ok(i) => Ok(parse_i64_to_timestamp_micros(i)),
        Err(_) => match parse_str_to_time(v) {
            Ok(v) => Ok(v.timestamp_micros()),
            Err(_) => Err(anyhow::anyhow!("invalid time format [string]")),
        },
    }
}

#[inline(always)]
pub fn parse_str_to_time(s: &str) -> Result<DateTime<Utc>, anyhow::Error> {
    if let Ok(v) = s.parse::<f64>() {
        let v = parse_i64_to_timestamp_micros(v as i64);
        return Ok(Utc.timestamp_nanos(v * 1000));
    }

    let ret = if s.contains(' ') && s.len() == 19 {
        let fmt = "%Y-%m-%d %H:%M:%S";
        NaiveDateTime::parse_from_str(s, fmt)?.and_utc()
    } else if s.contains('T') && !s.contains(' ') {
        if s.len() == 19 {
            let fmt = "%Y-%m-%dT%H:%M:%S";
            NaiveDateTime::parse_from_str(s, fmt)?.and_utc()
        } else {
            let t = DateTime::parse_from_rfc3339(s)?;
            t.into()
        }
    } else {
        let t = DateTime::parse_from_rfc2822(s)?;
        t.into()
    };
    Ok(ret)
}

#[inline(always)]
pub fn parse_timestamp_micro_from_value(v: &json::Value) -> Result<i64, anyhow::Error> {
    let n = match v {
        json::Value::String(s) => parse_str_to_timestamp_micros(s)?,
        json::Value::Number(n) => {
            if n.is_i64() {
                n.as_i64().unwrap()
            } else if n.is_u64() {
                n.as_u64().unwrap() as i64
            } else if n.is_f64() {
                n.as_f64().unwrap() as i64
            } else {
                return Err(anyhow::anyhow!("Invalid time format [timestamp]"));
            }
        }
        _ => return Err(anyhow::anyhow!("Invalid time format [type]")),
    };
    Ok(parse_i64_to_timestamp_micros(n))
}

pub fn parse_milliseconds(s: &str) -> Result<u64, anyhow::Error> {
    let chars = s.chars().collect::<Vec<char>>();

    // without unit, default is second
    if chars.iter().all(|c| c.is_ascii_digit()) {
        return Ok(s.parse::<u64>().unwrap_or(0) * 1000);
    }

    let mut unit_pos = TIME_UNITS.len();
    let mut start = 0;
    let mut total = 0;

    let chars_count = chars.len();
    let mut i = 0;
    while i < chars_count {
        let c = chars.get(i).unwrap();
        if c.is_ascii_digit() {
            i += 1;
            continue;
        }
        if i == 0 {
            return Err(anyhow::anyhow!("Invalid time format: {c}"));
        }
        let step_value = chars[start..i]
            .iter()
            .collect::<String>()
            .parse::<u64>()
            .unwrap_or(0);
        start = i + 1;
        // check unit
        let pos = TIME_UNITS[..unit_pos].iter().position(|&x| x.0 == *c);
        if pos.is_none() && *c != 'm' {
            return Err(anyhow::anyhow!("Invalid time format: {c}"));
        }
        // check unit: ms
        let cur_unit = if *c == 'm' && i + 1 < chars_count && chars.get(i + 1).unwrap() == &'s' {
            i += 1;
            unit_pos = 0;
            &TIME_UNITS[unit_pos]
        } else {
            unit_pos = pos.unwrap();
            &TIME_UNITS[unit_pos]
        };
        // calc
        total += step_value * cur_unit.1;
        i += 1;
    }
    Ok(total)
}

pub fn parse_timezone_to_offset(offset: &str) -> i64 {
    // let offset = "+08:00"; // or "-07:00" or "UTC"
    let sign: i64;
    let time: String;

    if let Some(stripped) = offset.strip_prefix('+') {
        sign = 1;
        time = stripped.to_string();
    } else if let Some(stripped) = offset.strip_prefix('-') {
        sign = -1;
        time = stripped.to_string();
    } else if offset.to_uppercase() == "CST" {
        sign = 1;
        time = "+08:00".to_string();
    } else if offset.to_uppercase() == "UTC" || offset.to_uppercase() == "" {
        sign = 0;
        time = "00:00".to_string();
    } else {
        panic!("Invalid time zone offset");
    }

    // convert time to seconds
    let seconds: i64 = time
        .split(':')
        .map(|val| val.parse::<i64>().unwrap())
        .fold(0, |acc, val| acc * 60 + val * 60);

    sign * seconds
}

#[inline(always)]
pub fn parse_str_to_timestamp_micros_as_option(v: &str) -> Option<i64> {
    match v.parse() {
        Ok(i) => Some(parse_i64_to_timestamp_micros(i)),
        Err(_) => match parse_str_to_time(v) {
            Ok(v) => Some(v.timestamp_micros()),
            Err(_) => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_i64_to_timestamp_micros() {
        let v = 1609459200000000000;
        let t = parse_i64_to_timestamp_micros(v);
        assert_eq!(t, v / 1000);

        let v = 1609459200000000;
        let t = parse_i64_to_timestamp_micros(v);
        assert_eq!(t, v);

        let v = 1609459200000;
        let t = parse_i64_to_timestamp_micros(v);
        assert_eq!(t, v * 1000);

        let v = 1609459200;
        let t = parse_i64_to_timestamp_micros(v);
        assert_eq!(t, v * 1_000_000);
    }

    #[test]
    fn test_parse_str_to_time() {
        let s = "2021-01-01T00:00:00";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609459200000000);

        let s = "2021-01-01T00:00:00Z";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609459200000000);

        let s = "2021-01-01T23:59:59Z";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609545599000000);

        let s = "2021-01-01T00:00:00+08:00";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609430400000000);

        let s = "2021-01-01T00:00:00-08:00";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609488000000000);

        let s = "2021-01-01 00:00:00";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609459200000000);

        let s = "2021-01-01T00:00:00.000000Z";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609459200000000);

        let s = "2021-01-01T00:00:00.000000+08:00";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1609430400000000);

        let s = "Wed, 8 Mar 2023 16:46:51 CST";
        let t = parse_str_to_time(s).unwrap();
        assert_eq!(t.timestamp_micros(), 1678315611000000);
    }

    #[test]
    fn test_parse_str_to_timestamp_micros() {
        let s = "2021-01-01T00:00:00";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609459200000000);

        let s = "2021-01-01T00:00:00Z";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609459200000000);

        let s = "2021-01-01T00:00:00+08:00";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609430400000000);

        let s = "2021-01-01T00:00:00-08:00";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609488000000000);

        let s = "2021-01-01 00:00:00";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609459200000000);

        let s = "2021-01-01T00:00:00.000000Z";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609459200000000);

        let s = "2021-01-01T00:00:00.000000+08:00";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1609430400000000);

        let s = "Wed, 8 Mar 2023 16:46:51 CST";
        let t = parse_str_to_timestamp_micros(s).unwrap();
        assert_eq!(t, 1678315611000000);
    }

    #[test]
    fn test_parse_timestamp_micro_from_value() {
        let v = json::json!(1609459200000000i64);
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609459200000000);

        let v = json::json!("2021-01-01T00:00:00");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609459200000000);

        let v = json::json!("2021-01-01T00:00:00Z");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609459200000000);

        let v = json::json!("2021-01-01T00:00:00+08:00");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609430400000000);

        let v = json::json!("2021-01-01T00:00:00-08:00");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609488000000000);

        let v = json::json!("2021-01-01 00:00:00");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609459200000000);

        let v = json::json!("2021-01-01T00:00:00.000000Z");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1609459200000000);

        let v = json::json!("Wed, 8 Mar 2023 16:46:51 CST");
        let t = parse_timestamp_micro_from_value(&v).unwrap();
        assert_eq!(t, 1678315611000000);
    }

    #[test]
    fn test_parse_milliseconds_without_unit() {
        assert_eq!(parse_milliseconds("123").unwrap(), 123000);
        assert_eq!(parse_milliseconds("0").unwrap(), 0);
        assert_eq!(parse_milliseconds("").unwrap(), 0);
        assert!(parse_milliseconds("abc").is_err());
    }

    #[test]
    fn test_parse_milliseconds_with_unit() {
        assert_eq!(parse_milliseconds("1s").unwrap(), 1000);
        assert_eq!(parse_milliseconds("1m").unwrap(), 60 * 1000);
        assert_eq!(parse_milliseconds("1h").unwrap(), 3600 * 1000);
        assert_eq!(parse_milliseconds("1d").unwrap(), 24 * 3600 * 1000);
        assert_eq!(parse_milliseconds("1w").unwrap(), 7 * 24 * 3600 * 1000);
        assert_eq!(parse_milliseconds("1y").unwrap(), 365 * 24 * 3600 * 1000);
        assert_eq!(parse_milliseconds("1h10m30s").unwrap(), 4230000);
        assert_eq!(parse_milliseconds("1h10m30s10ms").unwrap(), 4230010);
        assert!(parse_milliseconds("s").is_err());
        assert!(parse_milliseconds("10z").is_err());
    }

    #[test]
    fn test_parse_timezone_to_offset() {
        assert_eq!(parse_timezone_to_offset(""), 0);
        assert_eq!(parse_timezone_to_offset("UTC"), 0);
        assert_eq!(parse_timezone_to_offset("CST"), 28800);
        assert_eq!(parse_timezone_to_offset("+08:00"), 28800);
        assert_eq!(parse_timezone_to_offset("-08:00"), -28800);
    }
}
