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

use chrono::Utc;
use config::{
    meta::stream::StreamType,
    utils::{json, time::BASE_TIME},
};
use infra::cache::stats;
use vrl::prelude::NotNan;

use crate::service::search as SearchService;

pub async fn get(org_id: &str, name: &str) -> Result<Vec<vrl::value::Value>, anyhow::Error> {
    let stats = stats::get_stream_stats(org_id, name, StreamType::EnrichmentTables);

    let rec_num = if stats.doc_num == 0 {
        100000
    } else {
        stats.doc_num
    };

    let query = config::meta::search::Query {
        sql: format!("SELECT * FROM \"{name}\" limit {rec_num}"),
        start_time: BASE_TIME.timestamp_micros(),
        end_time: Utc::now().timestamp_micros(),
        ..Default::default()
    };

    let req = config::meta::search::Request {
        query,
        encoding: config::meta::search::RequestEncoding::Empty,
        regions: vec![],
        clusters: vec![],
        timeout: 0,
        search_type: None,
    };
    // do search
    match SearchService::search("", org_id, StreamType::EnrichmentTables, None, &req).await {
        Ok(res) => {
            if !res.hits.is_empty() {
                Ok(res.hits.iter().map(convert_to_vrl).collect())
            } else {
                Ok(vec![])
            }
        }
        Err(err) => {
            log::error!("get enrichment table data error: {:?}", err);
            Ok(vec![])
        }
    }
}

fn convert_to_vrl(value: &json::Value) -> vrl::value::Value {
    match value {
        json::Value::Null => vrl::value::Value::Null,
        json::Value::Bool(b) => vrl::value::Value::Boolean(*b),
        json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                vrl::value::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                vrl::value::Value::Float(NotNan::new(f).unwrap_or(NotNan::new(0.0).unwrap()))
            } else {
                unimplemented!("handle other number types")
            }
        }
        json::Value::String(s) => vrl::value::Value::from(s.as_str()),
        json::Value::Array(arr) => {
            vrl::value::Value::Array(arr.iter().map(convert_to_vrl).collect())
        }
        json::Value::Object(obj) => vrl::value::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.to_string().into(), convert_to_vrl(v)))
                .collect(),
        ),
    }
}
