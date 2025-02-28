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

use config::{
    meta::promql::{Metadata, EXEMPLARS_LABEL, HASH_LABEL, METADATA_LABEL, VALUE_LABEL},
    utils::hash::{gxhash, Sum64},
};
use datafusion::arrow::datatypes::Schema;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod json;
pub mod otlp;
pub mod prom;

const EXCLUDE_LABELS: [&str; 7] = [
    VALUE_LABEL,
    HASH_LABEL,
    EXEMPLARS_LABEL,
    "is_monotonic",
    "trace_id",
    "span_id",
    "_timestamp",
];

static RE_CORRECT_LABEL_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9_]+").unwrap());

pub fn get_prom_metadata_from_schema(schema: &Schema) -> Option<Metadata> {
    let metadata = schema.metadata.get(METADATA_LABEL)?;
    let metadata: Metadata = config::utils::json::from_str(metadata).unwrap();
    Some(metadata)
}

/// `signature_without_labels` is just as [`signature`], but only for labels not
/// matching `names`.
// REFACTORME: make this a method of `Metric`
pub fn signature_without_labels(
    labels: &config::utils::json::Map<String, config::utils::json::Value>,
    exclude_names: &[&str],
) -> u64 {
    let mut labels: Vec<(&str, &str)> = labels
        .iter()
        .filter(|(key, _value)| !exclude_names.contains(&key.as_str()))
        .map(|(key, value)| (key.as_str(), value.as_str().unwrap_or("")))
        .collect();
    labels.sort_by(|a, b| a.0.cmp(b.0));

    let key = labels
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<String>>()
        .join("|");
    gxhash::new().sum64(&key)
}

fn get_exclude_labels() -> Vec<&'static str> {
    let vec: Vec<&str> = EXCLUDE_LABELS.to_vec();
    // TODO: fixed _timestamp
    // let column_timestamp = config::TIMESTAMP_COL_NAME.as_str();
    // vec.push(column_timestamp);
    vec
}

// format stream name
pub fn format_label_name(label: &str) -> String {
    RE_CORRECT_LABEL_NAME.replace_all(label, "_").to_string()
}
