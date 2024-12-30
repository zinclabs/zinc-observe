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
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use config::meta::search::ScanStats;
use datafusion::{arrow::datatypes::Schema, error::Result, prelude::SessionContext};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod aggregations;
mod binaries;
pub mod common;
mod engine;
mod exec;
mod functions;
#[cfg(feature = "enterprise")]
pub mod name_visitor;
pub mod search;
pub mod value;

pub use engine::Engine;
pub use exec::Query;

pub(crate) const DEFAULT_LOOKBACK: Duration = Duration::from_secs(300); // 5m
pub(crate) const MINIMAL_INTERVAL: Duration = Duration::from_secs(1); // 1s
pub(crate) const MAX_DATA_POINTS: i64 = 256; // Width of panel: window.innerWidth / 4
pub(crate) const DEFAULT_MAX_POINTS_PER_SERIES: usize = 30000; // Maximum number of points per series
const DEFAULT_MAX_SERIES_PER_QUERY: usize = 30000; // Maximum number of series in a single query
const DEFAULT_STEP: Duration = Duration::from_secs(15); // default step in seconds
const MIN_TIMESERIES_POINTS_FOR_TIME_ROUNDING: i64 = 10; // Adjust this value as needed

#[async_trait]
pub trait TableProvider: Sync + Send + 'static {
    async fn create_context(
        &self,
        org_id: &str,
        stream_name: &str,
        time_range: (i64, i64),
        filters: &mut [(String, Vec<String>)],
    ) -> Result<Vec<(SessionContext, Arc<Schema>, ScanStats)>>;
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricsQueryRequest {
    pub query: String,
    pub start: i64,
    pub end: i64,
    pub step: i64,
    pub no_cache: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Success,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub result_type: String, // vector, matrix, scalar, string
    pub result: value::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<QueryResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub(crate) enum ApiFuncResponse<T: Serialize> {
    Success {
        data: T,
    },
    Error {
        #[serde(rename = "errorType")]
        error_type: ApiErrorType,
        error: String,
    },
}

impl<T: Serialize> ApiFuncResponse<T> {
    pub(crate) fn ok(data: T) -> Self {
        ApiFuncResponse::Success { data }
    }

    pub(crate) fn err_bad_data(error: impl ToString) -> Self {
        ApiFuncResponse::Error {
            error_type: ApiErrorType::BadData,
            error: error.to_string(),
        }
    }

    pub(crate) fn err_internal(error: impl ToString) -> Self {
        ApiFuncResponse::Error {
            error_type: ApiErrorType::Internal,
            error: error.to_string(),
        }
    }
}

// cf. https://github.com/prometheus/prometheus/blob/5c5fa5c319fca713506fa144ec6768fddf00d466/web/api/v1/api.go#L73-L82
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorType {
    Timeout,
    Cancelled,
    Exec,
    BadData,
    Internal,
    Unavailable,
    NotFound,
}

/// Converts `t` to the number of microseconds elapsed since the beginning of
/// the Unix epoch.
pub(crate) fn micros_since_epoch(t: SystemTime) -> i64 {
    micros(
        t.duration_since(UNIX_EPOCH)
            .expect("BUG: {t} is earlier than Unix epoch"),
    )
}

pub(crate) fn micros(t: Duration) -> i64 {
    t.as_micros()
        .try_into()
        .expect("BUG: time value is too large to fit in i64")
}

pub fn round_step(mut step: i64) -> i64 {
    // align step to seconds
    let second = micros(Duration::from_secs(1));
    if step >= second {
        step -= step % second;
    }
    if step == 0 {
        micros(DEFAULT_STEP)
    } else if step > (100 * second) {
        step - (step % (10 * second))
    } else if step > (10 * second) {
        step - (step % (5 * second))
    } else {
        step
    }
}

pub fn align_start_end(mut start: i64, mut end: i64, step: i64) -> (i64, i64) {
    // Round start to the nearest smaller value divisible by step.
    start -= start % step;
    // Round end to the nearest bigger value divisible by step.
    let adjust = end % step;
    if adjust > 0 {
        end += step - adjust
    }
    (start, end)
}

pub fn adjust_start_end(start: i64, end: i64, step: i64, disable_cache: bool) -> (i64, i64) {
    if disable_cache {
        // Do not adjust start and end values when cache is disabled.
        return (start, end);
    }

    let points = (end - start) / step + 1;

    if points < MIN_TIMESERIES_POINTS_FOR_TIME_ROUNDING {
        // Too small number of points for rounding.
        return (start, end);
    }

    // Round start and end to values divisible by step in order
    // to enable response caching.
    let (start, mut end) = align_start_end(start, end, step);

    // Make sure that the new number of points is the same as the initial number of points.
    let mut new_points = (end - start) / step + 1;
    while new_points > points {
        end -= step;
        new_points -= 1;
    }

    (start, end)
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn test_api_func_response_serialize() {
        let ok = ApiFuncResponse::ok("hello".to_owned());
        assert_eq!(
            serde_json::to_string(&ok).unwrap(),
            r#"{"status":"success","data":"hello"}"#
        );

        let err = ApiFuncResponse::<()>::err_internal("something went wrong".to_owned());
        assert_eq!(
            serde_json::to_string(&err).unwrap(),
            r#"{"status":"error","errorType":"internal","error":"something went wrong"}"#
        );

        let err = ApiFuncResponse::<()>::err_bad_data(
            r#"invalid parameter \"start\": Invalid time value for 'start': cannot parse \"foobar\" to a valid timestamp"#,
        );
        expect![[r#"
            {
              "status": "error",
              "errorType": "bad_data",
              "error": "invalid parameter \\\"start\\\": Invalid time value for 'start': cannot parse \\\"foobar\\\" to a valid timestamp"
            }"#
        ]].assert_eq(&serde_json::to_string_pretty(&err).unwrap());
    }
}
