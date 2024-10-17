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

use datafusion::error::{DataFusionError, Result};
use hashbrown::HashMap;

use crate::{
    common::meta::prom::{BUCKET_LABEL, HASH_LABEL, NAME_LABEL},
    service::promql::value::{
        signature_without_labels, InstantValue, Labels, LabelsExt, Sample, Signature, Value,
    },
};

// https://github.com/prometheus/prometheus/blob/cf1bea344a3c390a90c35ea8764c4a468b345d5e/promql/quantile.go#L33
#[derive(Debug, Clone, Copy, PartialEq)]
struct Bucket {
    upper_bound: f64,
    count: f64,
}

impl Bucket {
    #[allow(dead_code)]
    fn new(upper_bound: f64, count: f64) -> Self {
        Self { upper_bound, count }
    }
}

// https://github.com/prometheus/prometheus/blob/cf1bea344a3c390a90c35ea8764c4a468b345d5e/promql/quantile.go#L45
#[derive(Debug)]
struct MetricWithBuckets {
    labels: Labels,
    buckets: Vec<Bucket>,
}

/// TODO: support native histograms; see [`histogramQuantile`]
///
/// [`histogramQuantile`]: https://github.com/prometheus/prometheus/blob/f7c6130ff27a2a12412c02cce223f7a8abc59e49/promql/quantile.go#L146
pub(crate) fn histogram_quantile(sample_time: i64, phi: f64, data: Value) -> Result<Value> {
    let in_vec = match data {
        Value::Vector(v) => v,
        Value::None => return Ok(Value::None),
        _ => {
            return Err(DataFusionError::Plan(
                "histogram_quantile: vector argument expected".to_owned(),
            ));
        }
    };

    let mut metrics_with_buckets: HashMap<Signature, MetricWithBuckets> = HashMap::default();
    for InstantValue { mut labels, sample } in in_vec {
        // [https://prometheus.io/docs/prometheus/latest/querying/functions/#histogram_quantile]:
        //
        // The conventional float samples in `in_vec` are considered the counts
        // of observations in each bucket of one or more conventional
        // histograms. Each float sample must have a label `le` where the label
        // value denotes the inclusive upper bound of the bucket. *Float samples
        // without such a label are silently ignored.*
        let upper_bound: f64 = match labels.get_value(BUCKET_LABEL).parse() {
            Ok(u) => u,
            Err(_) => continue,
        };

        let sig = signature_without_labels(&labels, &[HASH_LABEL, NAME_LABEL, BUCKET_LABEL]);
        let entry = metrics_with_buckets.entry(sig).or_insert_with(|| {
            labels
                .retain(|l| l.name != HASH_LABEL && l.name != NAME_LABEL && l.name != BUCKET_LABEL);
            MetricWithBuckets {
                labels,
                buckets: Vec::new(),
            }
        });
        entry.buckets.push(Bucket {
            upper_bound,
            count: sample.value,
        });
    }

    let values = metrics_with_buckets
        .into_values()
        .map(|mb| InstantValue {
            labels: mb.labels,
            sample: Sample::new(sample_time, bucket_quantile(phi, mb.buckets)),
        })
        .collect();

    Ok(Value::Vector(values))
}

// cf. https://github.com/prometheus/prometheus/blob/cf1bea344a3c390a90c35ea8764c4a468b345d5e/promql/quantile.go#L76
fn bucket_quantile(phi: f64, mut buckets: Vec<Bucket>) -> f64 {
    if phi.is_nan() || buckets.is_empty() {
        return f64::NAN;
    }
    if phi < 0.0 {
        return f64::NEG_INFINITY;
    }
    if phi > 1.0 {
        return f64::INFINITY;
    }
    buckets.sort_by(|a, b| a.upper_bound.partial_cmp(&b.upper_bound).unwrap());
    // The caller of `bucket_quantile` guarantees that `buckets` is non-empty.
    let highest_bucket = buckets[buckets.len() - 1];
    if !(highest_bucket.upper_bound.is_infinite() && highest_bucket.upper_bound.is_sign_positive())
    {
        return f64::NAN;
    }
    let mut buckets = coalesce_buckets(buckets);
    ensure_monotonic(&mut buckets);
    let buckets = buckets;
    if buckets.len() < 2 {
        return f64::NAN;
    }
    let observations = buckets[buckets.len() - 1].count;
    if observations == 0.0 {
        return f64::NAN;
    }
    let mut rank = phi * observations;
    let b = match buckets[..buckets.len() - 1]
        .iter()
        .position(|b| b.count >= rank)
    {
        Some(b) => b,
        None => buckets.len() - 1, // Should not reach here if data is valid
    };
    if b == buckets.len() - 1 {
        return buckets[buckets.len() - 2].upper_bound;
    }
    if b == 0 && buckets[0].upper_bound <= 0.0 {
        return buckets[0].upper_bound;
    }
    let bucket_end = buckets[b].upper_bound;
    let mut count = buckets[b].count;
    let bucket_start = if b > 0 {
        count -= buckets[b - 1].count;
        rank -= buckets[b - 1].count;
        buckets[b - 1].upper_bound
    } else {
        0.0
    };

    bucket_start + (bucket_end - bucket_start) * (rank / count)
}

/// `coalesce_buckets` merges buckets with the same upper bound.
/// The input buckets must be sorted.
fn coalesce_buckets(buckets: Vec<Bucket>) -> Vec<Bucket> {
    let mut st = None;
    let mut buckets = buckets
        .into_iter()
        .filter_map(|b| match st {
            None => {
                st = Some(b);
                None
            }
            Some(last) => {
                if b.upper_bound == last.upper_bound {
                    st = Some(Bucket {
                        count: last.count + b.count,
                        ..last
                    });
                    None
                } else {
                    st = Some(b);
                    Some(last)
                }
            }
        })
        .collect::<Vec<_>>();
    if let Some(last) = st {
        buckets.push(last);
    }
    buckets
}

// For the rationale behind this function, see
// https://github.com/prometheus/prometheus/blob/0bf707e288eaa8694105e53c81a102017529793d/promql/quantile.go#L314-L347
fn ensure_monotonic(buckets: &mut [Bucket]) {
    let mut max = buckets[0].count;
    for bucket in &mut buckets[1..] {
        if bucket.count > max {
            max = bucket.count;
        } else if bucket.count < max {
            bucket.count = max;
        }
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use super::*;

    #[test]
    fn test_coalesce_buckets() {
        let buckets = vec![
            Bucket {
                upper_bound: 1.0,
                count: 2.0,
            },
            Bucket {
                upper_bound: 1.0,
                count: 3.0,
            },
            Bucket {
                upper_bound: 2.0,
                count: 4.0,
            },
            Bucket {
                upper_bound: 3.0,
                count: 1.0,
            },
            Bucket {
                upper_bound: 3.0,
                count: 1.0,
            },
        ];

        expect![[r#"
            [
                Bucket {
                    upper_bound: 1.0,
                    count: 5.0,
                },
                Bucket {
                    upper_bound: 2.0,
                    count: 4.0,
                },
                Bucket {
                    upper_bound: 3.0,
                    count: 2.0,
                },
            ]
        "#]]
        .assert_debug_eq(&coalesce_buckets(buckets));
    }

    #[test]
    fn test_coalesce_buckets_regular() {
        let buckets = vec![
            Bucket::new(1.0, 2.0),
            Bucket::new(2.0, 3.0),
            Bucket::new(2.0, 5.0),
            Bucket::new(3.0, 4.0),
            Bucket::new(4.0, 1.0),
        ];

        let expected_result = vec![
            Bucket::new(1.0, 2.0),
            Bucket::new(2.0, 8.0),
            Bucket::new(3.0, 4.0),
            Bucket::new(4.0, 1.0),
        ];

        let result = coalesce_buckets(buckets.clone());

        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_coalesce_buckets_empty() {
        let buckets = vec![];

        let expected_result = vec![];

        let result = coalesce_buckets(buckets.clone());

        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_coalesce_buckets_single_element() {
        let buckets = vec![Bucket::new(1.0, 2.0)];

        let expected_result = vec![Bucket::new(1.0, 2.0)];

        let result = coalesce_buckets(buckets.clone());

        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_coalesce_buckets_all_same() {
        let buckets = vec![
            Bucket::new(1.0, 2.0),
            Bucket::new(1.0, 3.0),
            Bucket::new(1.0, 5.0),
        ];

        let expected_result = vec![Bucket::new(1.0, 10.0)];

        let result = coalesce_buckets(buckets.clone());

        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_ensure_monotonic() {
        let mut buckets = vec![
            Bucket {
                upper_bound: 1.0,
                count: 2.0,
            },
            Bucket {
                upper_bound: 2.0,
                count: 1.0,
            },
            Bucket {
                upper_bound: 3.0,
                count: 4.0,
            },
            Bucket {
                upper_bound: 4.0,
                count: 3.0,
            },
            Bucket {
                upper_bound: 5.0,
                count: 5.0,
            },
        ];
        ensure_monotonic(&mut buckets);
        expect![[r#"
            [
                Bucket {
                    upper_bound: 1.0,
                    count: 2.0,
                },
                Bucket {
                    upper_bound: 2.0,
                    count: 2.0,
                },
                Bucket {
                    upper_bound: 3.0,
                    count: 4.0,
                },
                Bucket {
                    upper_bound: 4.0,
                    count: 4.0,
                },
                Bucket {
                    upper_bound: 5.0,
                    count: 5.0,
                },
            ]
        "#]]
        .assert_debug_eq(&buckets);
    }

    #[test]
    fn test_ensure_monotonic_single_bucket() {
        let mut buckets = vec![Bucket::new(1.0, 2.0)];
        ensure_monotonic(&mut buckets);
        assert_eq!(buckets, vec![Bucket::new(1.0, 2.0),]);
    }

    #[test]
    fn test_ensure_monotonic_increasing() {
        let mut buckets = vec![
            Bucket::new(1.0, 2.0),
            Bucket::new(2.0, 3.0),
            Bucket::new(3.0, 4.0),
            Bucket::new(4.0, 5.0),
        ];
        ensure_monotonic(&mut buckets);
        assert_eq!(
            buckets,
            vec![
                Bucket::new(1.0, 2.0),
                Bucket::new(2.0, 3.0),
                Bucket::new(3.0, 4.0),
                Bucket::new(4.0, 5.0),
            ]
        );
    }

    #[test]
    fn test_ensure_monotonic_decreasing() {
        let mut buckets = vec![
            Bucket::new(1.0, 5.0),
            Bucket::new(2.0, 4.0),
            Bucket::new(3.0, 3.0),
            Bucket::new(4.0, 2.0),
        ];
        ensure_monotonic(&mut buckets);
        assert_eq!(
            buckets,
            vec![
                Bucket::new(1.0, 5.0),
                Bucket::new(2.0, 5.0),
                Bucket::new(3.0, 5.0),
                Bucket::new(4.0, 5.0),
            ]
        );
    }

    #[test]
    fn test_ensure_monotonic_mixed() {
        let mut buckets = vec![
            Bucket::new(1.0, 5.0),
            Bucket::new(2.0, 3.0),
            Bucket::new(3.0, 7.0),
            Bucket::new(4.0, 2.0),
        ];
        ensure_monotonic(&mut buckets);
        assert_eq!(
            buckets,
            vec![
                Bucket::new(1.0, 5.0),
                Bucket::new(2.0, 5.0),
                Bucket::new(3.0, 7.0),
                Bucket::new(4.0, 7.0),
            ]
        );
    }
}
