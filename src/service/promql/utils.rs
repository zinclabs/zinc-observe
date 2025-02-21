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

use std::collections::HashSet;

use config::{
    meta::promql::{BUCKET_LABEL, HASH_LABEL, VALUE_LABEL},
    TIMESTAMP_COL_NAME,
};
use datafusion::{
    arrow::datatypes::Schema,
    error::Result,
    prelude::{col, lit, DataFrame},
};
use promql_parser::label::{MatchOp, Matchers};

use crate::service::search::datafusion::udf::regexp_udf::{REGEX_MATCH_UDF, REGEX_NOT_MATCH_UDF};

pub fn apply_matchers(df: DataFrame, schema: &Schema, matchers: &Matchers) -> Result<DataFrame> {
    let mut df = df;
    for mat in matchers.matchers.iter() {
        if mat.name == TIMESTAMP_COL_NAME
            || mat.name == VALUE_LABEL
            || schema.field_with_name(&mat.name).is_err()
        {
            continue;
        }
        match &mat.op {
            MatchOp::Equal => df = df.filter(col(mat.name.clone()).eq(lit(mat.value.clone())))?,
            MatchOp::NotEqual => {
                df = df.filter(col(mat.name.clone()).not_eq(lit(mat.value.clone())))?
            }
            MatchOp::Re(regex) => {
                let regexp_match_udf = REGEX_MATCH_UDF.clone();
                let regex = format!("^{}$", regex.as_str());
                df = df.filter(regexp_match_udf.call(vec![col(mat.name.clone()), lit(regex)]))?
            }
            MatchOp::NotRe(regex) => {
                let regexp_not_match_udf = REGEX_NOT_MATCH_UDF.clone();
                let regex = format!("^{}$", regex.as_str());
                df =
                    df.filter(regexp_not_match_udf.call(vec![col(mat.name.clone()), lit(regex)]))?
            }
        }
    }
    Ok(df)
}

pub fn apply_label_selector(
    df: DataFrame,
    schema: &Schema,
    label_selectors: &Option<HashSet<String>>,
) -> Option<DataFrame> {
    let mut df = df;
    if let Some(label_selector) = label_selectors {
        if !label_selector.is_empty() {
            let schema_fields = schema
                .fields()
                .iter()
                .map(|f| f.name())
                .collect::<HashSet<_>>();
            let mut def_labels = vec![
                HASH_LABEL.to_string(),
                VALUE_LABEL.to_string(),
                BUCKET_LABEL.to_string(),
                TIMESTAMP_COL_NAME.to_string(),
            ];
            for label in label_selector.iter() {
                if def_labels.contains(label) {
                    def_labels.retain(|x| x != label);
                }
            }
            // include only found columns and required _timestamp, hash, value, le cols
            let selected_cols: Vec<_> = label_selector
                .iter()
                .chain(def_labels.iter())
                .filter_map(|label| {
                    if schema_fields.contains(label) {
                        Some(col(label))
                    } else {
                        None
                    }
                })
                .collect();
            df = match df.select(selected_cols) {
                Ok(df) => df,
                Err(e) => {
                    log::error!("Selecting cols error: {}", e);
                    return None;
                }
            };
        }
    }
    Some(df)
}
