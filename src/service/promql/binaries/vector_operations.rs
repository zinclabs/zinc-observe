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
    collections::{HashMap, HashSet},
    sync::Arc,
};

use datafusion::error::{DataFusionError, Result};
use once_cell::sync::Lazy;
use promql_parser::parser::{token, BinaryExpr, VectorMatchCardinality};
use rayon::prelude::*;

use crate::{
    common::meta::prom::NAME_LABEL,
    service::promql::{
        binaries::scalar_binary_operations,
        value::{signature, InstantValue, Label, LabelsExt, Sample, Signature, Value},
    },
};

// DROP_METRIC_VECTOR_BIN_OP if the operation is one of these, drop the metric
// __name__
pub static DROP_METRIC_VECTOR_BIN_OP: Lazy<HashSet<u8>> = Lazy::new(|| {
    HashSet::from_iter([
        token::T_ADD,
        token::T_SUB,
        token::T_DIV,
        token::T_MUL,
        token::T_POW,
        token::T_MOD,
    ])
});

/// Implement the operation between a vector and a float.
///
/// https://prometheus.io/docs/prometheus/latest/querying/operators/#arithmetic-binary-operators
pub async fn vector_scalar_bin_op(
    expr: &BinaryExpr,
    left: &[InstantValue],
    right: f64,
    swapped_lhs_rhs: bool,
) -> Result<Value> {
    let is_comparison_operator = expr.op.is_comparison_operator();
    let return_bool = expr.return_bool();
    let output: Vec<InstantValue> = left
        .par_iter()
        .flat_map(|instant| {
            let (lhs, rhs) = if swapped_lhs_rhs {
                (right, instant.sample.value)
            } else {
                (instant.sample.value, right)
            };
            match scalar_binary_operations(
                expr.op.id(),
                lhs,
                rhs,
                return_bool,
                is_comparison_operator,
            )
            .ok()
            {
                Some(value) => {
                    let labels = if return_bool || DROP_METRIC_VECTOR_BIN_OP.contains(&expr.op.id())
                    {
                        instant.labels.without_metric_name()
                    } else {
                        instant.labels.clone()
                    };

                    let final_value = if is_comparison_operator && swapped_lhs_rhs && return_bool {
                        value
                    } else if is_comparison_operator && swapped_lhs_rhs {
                        instant.sample.value
                    } else {
                        value
                    };

                    Some(InstantValue {
                        labels,
                        sample: Sample {
                            timestamp: instant.sample.timestamp,
                            value: final_value,
                        },
                    })
                }
                None => None,
            }
        })
        .collect();

    Ok(Value::Vector(output))
}

/// vector1 or vector2 results in a vector that contains all original elements
/// (label sets + values) of vector1 and additionally all elements of vector2
/// which do not have matching label sets in vector1.
///
/// https://prometheus.io/docs/prometheus/latest/querying/operators/#logical-set-binary-operators
fn vector_or(expr: &BinaryExpr, left: &[InstantValue], right: &[InstantValue]) -> Result<Value> {
    if expr.modifier.as_ref().unwrap().card != VectorMatchCardinality::ManyToMany {
        return Err(DataFusionError::NotImplemented(
            "set operations must only use many-to-many matching".to_string(),
        ));
    }

    if left.is_empty() {
        return Ok(Value::Vector(right.to_vec()));
    }

    if right.is_empty() {
        return Ok(Value::Vector(left.to_vec()));
    }

    let lhs_sig: HashSet<Signature> = left
        .par_iter()
        .map(|item| signature(&item.labels))
        .collect();

    // Add all right-hand side elements which have not been added from the left-hand
    // side.
    let right_instants: Vec<InstantValue> = right
        .par_iter()
        .filter(|item| {
            let right_sig = signature(&item.labels);
            !lhs_sig.contains(&right_sig)
        })
        .map(|item| item.clone())
        .collect();

    let output = [left, &right_instants].concat();
    Ok(Value::Vector(output))
}

/// vector1 unless vector2 results in a vector consisting of the elements of
/// vector1 for which there are no elements in vector2 with exactly matching
/// label sets. All matching elements in both vectors are dropped.
///
/// https://prometheus.io/docs/prometheus/latest/querying/operators/#logical-set-binary-operators
fn vector_unless(
    expr: &BinaryExpr,
    left: &[InstantValue],
    right: &[InstantValue],
) -> Result<Value> {
    if expr.modifier.as_ref().unwrap().card != VectorMatchCardinality::ManyToMany {
        return Err(DataFusionError::NotImplemented(
            "set operations must only use many-to-many matching".to_string(),
        ));
    }

    // If right is empty, we simply return the left
    // if left is empty we will return it anyway.
    if left.is_empty() || right.is_empty() {
        return Ok(Value::Vector(left.to_vec()));
    }
    // Generate all the signatures from the right hand.
    let rhs_sig: HashSet<Signature> = right
        .par_iter()
        .map(|item| signature(&item.labels))
        .collect();

    // Now filter out all the matching labels from left.
    let output: Vec<InstantValue> = left
        .par_iter()
        .filter(|item| {
            let left_sig = signature(&item.labels);
            !rhs_sig.contains(&left_sig)
        })
        .map(|val| val.clone())
        .collect();
    Ok(Value::Vector(output))
}

/// vector1 and vector2 results in a vector consisting of the elements of
/// vector1 for which there are elements in vector2 with exactly matching label
/// sets. Other elements are dropped. The metric name and values are carried
/// over from the left-hand side vector.
///
/// https://prometheus.io/docs/prometheus/latest/querying/operators/#logical-set-binary-operators
fn vector_and(expr: &BinaryExpr, left: &[InstantValue], right: &[InstantValue]) -> Result<Value> {
    if expr.modifier.as_ref().unwrap().card != VectorMatchCardinality::ManyToMany {
        return Err(DataFusionError::NotImplemented(
            "set operations must only use many-to-many matching".to_string(),
        ));
    }

    if left.is_empty() || right.is_empty() {
        return Err(DataFusionError::NotImplemented(
            "Either left or right operand is empty.".to_string(),
        ));
    }

    let rhs_sig: HashSet<Signature> = right
        .par_iter()
        .map(|item| signature(&item.labels))
        .collect();

    // Now include all the matching ones from the right
    let output: Vec<InstantValue> = left
        .par_iter()
        .filter(|item| {
            let left_sig = signature(&item.labels);
            rhs_sig.contains(&left_sig)
        })
        .map(|instant| InstantValue {
            labels: instant.labels.without_metric_name(),
            sample: instant.sample,
        })
        .collect();

    Ok(Value::Vector(output))
}

fn vector_arithmetic_operators(
    expr: &BinaryExpr,
    left: &[InstantValue],
    right: &[InstantValue],
) -> Result<Value> {
    let operator = expr.op.id();

    let return_bool = expr.return_bool();
    let comparison_operator = expr.op.is_comparison_operator();

    // These are here so that we can generate signatures based on these labels.
    let mut labels_to_include_set = vec![];
    let mut labels_to_exclude_set = vec![NAME_LABEL.to_string()];

    let is_matching_on = expr.is_matching_on();
    if is_matching_on {
        let modifier = expr.modifier.as_ref().unwrap();
        if modifier.is_matching_on() {
            labels_to_include_set = modifier.matching.as_ref().unwrap().labels().labels.clone();
            labels_to_include_set.sort();
        } else {
            let excluded_labels = modifier.matching.as_ref().unwrap().labels().labels.clone();
            labels_to_exclude_set.extend(excluded_labels);
            labels_to_exclude_set.sort();
        }
    }

    // These labels should be used to compare values between lhs - rhs
    let labels_to_compare = |labels: &Vec<Arc<Label>>| {
        if is_matching_on {
            labels.keep(&labels_to_include_set)
        } else {
            labels.delete(&labels_to_exclude_set)
        }
    };

    // Get the hash for the labels on the right
    let rhs_sig: HashMap<Signature, InstantValue> = right
        .par_iter()
        .map(|instant| {
            let signature = labels_to_compare(&instant.labels).signature();
            (signature, instant.clone())
        })
        .collect();

    // Iterate over left and pick up the corresponding instance from rhs
    let output: Vec<InstantValue> = left
        .par_iter()
        .flat_map(|instant| {
            let left_sig = labels_to_compare(&instant.labels).signature();
            rhs_sig
                .get(&left_sig)
                .map(|rhs_instant| (instant, rhs_instant))
        })
        .flat_map(|(lhs_instant, rhs_instant)| {
            scalar_binary_operations(
                operator,
                lhs_instant.sample.value,
                rhs_instant.sample.value,
                return_bool,
                comparison_operator,
            )
            .ok()
            .map(|value| {
                let mut labels = if return_bool || DROP_METRIC_VECTOR_BIN_OP.contains(&operator) {
                    lhs_instant.labels.without_metric_name()
                } else {
                    lhs_instant.labels.clone()
                };

                if let Some(modifier) = expr.modifier.as_ref() {
                    if modifier.card == VectorMatchCardinality::OneToOne {
                        labels = labels_to_compare(&labels);
                    }

                    // group_labels from the `group_x` modifier are taken from the "one"-side.
                    if let Some(group_labels) = modifier.card.labels() {
                        for ln in group_labels.labels.iter() {
                            let value = rhs_instant.labels.get_value(ln);
                            if !value.is_empty() {
                                labels.set(ln, &value);
                            }
                        }
                    }
                }
                InstantValue {
                    labels,
                    sample: Sample {
                        timestamp: lhs_instant.sample.timestamp,
                        value,
                    },
                }
            })
        })
        .collect();

    Ok(Value::Vector(output))
}

/// Implement binary operations between two vectors
///
/// https://prometheus.io/docs/prometheus/latest/querying/operators/#comparison-binary-operators
///
/// Between two instant vectors, a binary arithmetic operator is applied to each
/// entry in the left-hand side vector and its matching element in the
/// right-hand vector. The result is propagated into the result vector with the
/// grouping labels becoming the output label set. The metric name is dropped.
/// Entries for which no matching entry in the right-hand vector can be found
/// are not part of the result.
///
///
/// Between two instant vectors, comparison binary operators behave as a filter
/// by default, applied to matching entries. Vector elements for which the
/// expression is not true or which do not find a match on the other side of the
/// expression get dropped from the result, while the others are propagated into
/// a result vector with the grouping labels becoming the output label set. If
/// the bool modifier is provided, vector elements that would have been dropped
/// instead have the value 0 and vector elements that would be kept have the
/// value 1, with the grouping labels again becoming the output label set. The
/// metric name is dropped if the bool modifier is provided.
pub fn vector_bin_op(
    expr: &BinaryExpr,
    left: &[InstantValue],
    right: &[InstantValue],
) -> Result<Value> {
    match expr.op.id() {
        token::T_LAND => vector_and(expr, left, right),
        token::T_LOR => vector_or(expr, left, right),
        token::T_LUNLESS => vector_unless(expr, left, right),
        _ => vector_arithmetic_operators(expr, left, right),
    }
}
