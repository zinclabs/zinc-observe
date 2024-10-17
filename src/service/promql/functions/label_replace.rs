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

use std::sync::Arc;

use datafusion::error::{DataFusionError, Result};
use rayon::prelude::*;
use regex::Regex;

use crate::service::promql::value::{InstantValue, Label, LabelsExt, Value};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#label_replace
pub(crate) fn label_replace(
    data: &Value,
    dest_label: &str,
    replacement: &str,
    source_label: &str,
    regex: &str,
) -> Result<Value> {
    let data = match data {
        Value::Vector(v) => v,
        Value::None => return Ok(Value::None),
        _ => {
            return Err(DataFusionError::Plan(
                "label_replace: vector argument expected".into(),
            ));
        }
    };

    // Check if the destination label is a valid name
    if !Label::is_valid_label_name(dest_label) {
        return Err(DataFusionError::NotImplemented(format!(
            "label_replace: invalid destination label provided {dest_label}"
        )));
    }

    let re = Regex::new(regex)
        .map_err(|_e| DataFusionError::NotImplemented("Invalid regex found".into()))?;

    let rate_values: Vec<InstantValue> = data
        .par_iter()
        .map(|instant| {
            let labels = if replacement.is_empty() {
                instant.labels.without_label(dest_label)
            } else {
                let label_value = instant.labels.get_value(source_label);
                let output_value = re.replace_all(&label_value, replacement);
                let mut new_labels = instant.labels.clone();
                if output_value != label_value {
                    new_labels.push(Arc::new(Label {
                        name: dest_label.to_string(),
                        value: output_value.to_string(),
                    }));
                }
                new_labels
            };
            InstantValue {
                labels,
                sample: instant.sample,
            }
        })
        .collect();
    Ok(Value::Vector(rate_values))
}
