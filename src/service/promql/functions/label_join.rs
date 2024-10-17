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

use std::{collections::HashSet, sync::Arc};

use datafusion::error::{DataFusionError, Result};
use itertools::Itertools;

use crate::{
    common::meta::prom::NAME_LABEL,
    service::promql::value::{InstantValue, Label, Value},
};

/// https://prometheus.io/docs/prometheus/latest/querying/functions/#label_join
pub(crate) fn label_join(
    data: &Value,
    dest_label: &str,
    separator: &str,
    source_labels: Vec<String>,
) -> Result<Value> {
    let data = match data {
        Value::Vector(v) => v,
        Value::None => return Ok(Value::None),
        _ => {
            return Err(DataFusionError::Plan(
                "label_join: vector argument expected".into(),
            ));
        }
    };

    let keep_source_labels: HashSet<String> = HashSet::from_iter(source_labels);
    let rate_values: Vec<InstantValue> = data
        .iter()
        .map(|instant| {
            let new_label = instant
                .labels
                .iter()
                .filter(|l| l.name != NAME_LABEL && keep_source_labels.contains(&l.name))
                .map(|label| label.value.clone())
                .join(separator);

            let mut new_labels = instant.labels.clone();
            new_labels.push(Arc::new(Label {
                name: dest_label.to_string(),
                value: new_label,
            }));
            InstantValue {
                labels: new_labels,
                sample: instant.sample,
            }
        })
        .collect();
    Ok(Value::Vector(rate_values))
}
