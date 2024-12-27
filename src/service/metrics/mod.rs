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

use datafusion::arrow::datatypes::Schema;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::common::meta::prom::{Metadata, HASH_LABEL, METADATA_LABEL, VALUE_LABEL};

pub mod json;
pub mod otlp;
pub mod prom;

const EXCLUDE_LABELS: [&str; 5] = [
    VALUE_LABEL,
    HASH_LABEL,
    "is_monotonic",
    "exemplars",
    "_timestamp",
];

static RE_CORRECT_LABEL_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9_]+").unwrap());

pub fn get_prom_metadata_from_schema(schema: &Schema) -> Option<Metadata> {
    let metadata = schema.metadata.get(METADATA_LABEL)?;
    let metadata: Metadata = config::utils::json::from_str(metadata).unwrap();
    Some(metadata)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Signature([u8; 32]);

impl From<Signature> for String {
    fn from(sig: Signature) -> Self {
        hex::encode(sig.0)
    }
}

/// `signature_without_labels` is just as [`signature`], but only for labels not
/// matching `names`.
// REFACTORME: make this a method of `Metric`
pub fn signature_without_labels(
    labels: &config::utils::json::Map<String, config::utils::json::Value>,
    exclude_names: &[&str],
) -> Signature {
    let mut labels: Vec<(&str, String)> = labels
        .iter()
        .filter(|(key, _value)| !exclude_names.contains(&key.as_str()))
        .map(|(key, value)| (key.as_str(), value.to_string()))
        .collect();
    labels.sort_by(|a, b| a.0.cmp(b.0));

    let mut hasher = blake3::Hasher::new();
    labels.iter().for_each(|(key, value)| {
        hasher.update(key.as_bytes());
        hasher.update(value.as_bytes());
    });
    Signature(hasher.finalize().into())
}

fn get_exclude_labels() -> Vec<&'static str> {
    let vec: Vec<&str> = EXCLUDE_LABELS.to_vec();
    // TODO: fixed _timestamp
    // let column_timestamp = config::get_config().common.column_timestamp.as_str();
    // vec.push(column_timestamp);
    vec
}

// format stream name
pub fn format_label_name(label: &str) -> String {
    RE_CORRECT_LABEL_NAME.replace_all(label, "_").to_string()
}
