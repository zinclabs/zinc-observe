use config::meta::{search::Response, sql::OrderBy};
use serde_json::Value;

/// Represents different sorting strategies
enum SortStrategy {
    SqlOrderBy,
    FallbackColumn(String, OrderBy),
    AutoDetermine(String, bool), // (column, is_string)
    NoSort,
}

/// Determines and applies sorting to search results
pub(crate) fn order_search_results(
    mut search_res: Response,
    fallback_order_by_col: Option<String>,
) -> Response {
    if search_res.hits.is_empty() {
        return search_res;
    }

    let strategy = determine_sort_strategy(&search_res, fallback_order_by_col);
    apply_sort_strategy(&mut search_res, strategy);

    search_res
}

/// Applies the chosen sort strategy to results
fn apply_sort_strategy(search_res: &mut Response, strategy: SortStrategy) {
    match strategy {
        SortStrategy::FallbackColumn(col, order) => {
            // Auto-detect column type from first hit
            let is_string = search_res
                .hits
                .first()
                .and_then(|hit| hit.get(&col))
                .map(|v| !v.is_number())
                .unwrap_or(true);

            // Sorting behavior:
            // - String columns: Always sort ascending (A->Z)
            // - Numeric columns: Respect the specified order (ASC/DESC)
            let is_descending = if is_string {
                false // Strings always sort ascending
            } else {
                order == OrderBy::Desc // Numbers follow specified order
            };

            sort_by_column(search_res, &col, is_string, is_descending);
            if search_res.order_by.is_none() {
                search_res.order_by = Some(order);
            }
        }
        SortStrategy::AutoDetermine(col, is_string) => {
            sort_by_column(search_res, &col, is_string, !is_string);
        }
        _ => (),
    }
}

/// Determines which sorting strategy to use
fn determine_sort_strategy(
    search_res: &Response,
    fallback_order_by_col: Option<String>,
) -> SortStrategy {
    // Check SQL ORDER BY first
    if let Some(order_by) = search_res.order_by {
        log::info!(
            "[trace_id: {}] Using user-specified ORDER BY: {:?}",
            &search_res.trace_id,
            order_by
        );
        return SortStrategy::SqlOrderBy;
    }

    // Check fallback column
    // Default to descending order
    if let Some(col) = find_fallback_column(search_res, fallback_order_by_col) {
        return SortStrategy::FallbackColumn(col, OrderBy::Desc);
    }

    // Auto-determine as last resort
    if let Some((col, is_string)) = determine_sort_column(&search_res.hits[0]) {
        log::info!(
            "[trace_id: {}] Auto-sorting by column: {}, type: {}",
            &search_res.trace_id,
            col,
            if is_string { "string" } else { "numeric" }
        );
        return SortStrategy::AutoDetermine(col, is_string);
    }

    SortStrategy::NoSort
}

/// Finds and validates fallback column in results
fn find_fallback_column(search_res: &Response, fallback_col: Option<String>) -> Option<String> {
    let fallback_col = fallback_col?;
    let first_hit = search_res.hits.first()?.as_object()?;

    // Find case-insensitive match
    first_hit
        .keys()
        .find(|k| k.eq_ignore_ascii_case(&fallback_col))
        .map(|k| {
            log::info!(
                "[trace_id: {}] Using fallback ORDER BY: {}",
                &search_res.trace_id,
                k
            );
            k.to_string()
        })
}

/// Sorts results by a specific column
fn sort_by_column(search_res: &mut Response, column: &str, is_string: bool, descending: bool) {
    search_res.hits.sort_by(|a, b| {
        let ordering = if is_string {
            compare_string_values(a, b, column)
        } else {
            compare_numeric_values(a, b, column)
        };
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
}

/// Compares string values with null handling
fn compare_string_values(a: &Value, b: &Value, column: &str) -> std::cmp::Ordering {
    let a_val = a.get(column).and_then(|v| v.as_str());
    let b_val = b.get(column).and_then(|v| v.as_str());
    match (a_val, b_val) {
        // When both values exist: "apple" vs "banana" -> normal alphabetical order
        (Some(a), Some(b)) => a.cmp(b),

        // When first value is null and second exists: null vs "apple" -> null goes last
        // Example: [apple, banana, null] in ascending order
        (None, Some(_)) => std::cmp::Ordering::Greater,

        // When first value exists and second is null: "apple" vs null -> non-null goes first
        // Example: [apple, banana, null] in ascending order
        (Some(_), None) => std::cmp::Ordering::Less,

        // When both values are null: null vs null -> treat as equal
        // Example: [null, null] -> order doesn't change
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// Compares numeric values with null handling
fn compare_numeric_values(a: &Value, b: &Value, column: &str) -> std::cmp::Ordering {
    let a_val = a.get(column).and_then(|v| v.as_f64());
    let b_val = b.get(column).and_then(|v| v.as_f64());
    match (a_val, b_val) {
        // When both are numbers: 1 vs 2 -> normal numeric order
        // If NaN encountered, treat values as equal
        // Example: [1, 2, 3] in ascending order
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),

        // When first is null and second is a number: null vs 1 -> null goes last
        // Example: [1, 2, null] in ascending order
        (None, Some(_)) => std::cmp::Ordering::Greater,

        // When first is a number and second is null: 1 vs null -> number goes first
        // Example: [1, 2, null] in ascending order
        (Some(_), None) => std::cmp::Ordering::Less,

        // When both are null: null vs null -> treat as equal
        // Example: [null, null] -> order doesn't change
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn determine_sort_column(first_hit: &config::utils::json::Value) -> Option<(String, bool)> {
    if let Some(obj) = first_hit.as_object() {
        // First try to find non-numeric columns
        for (key, value) in obj {
            if !value.is_number() {
                log::debug!("Using string column for sorting: {}", key);
                return Some((key.clone(), true)); // (column, is_string)
            }
        }

        // If no non-numeric column found, take first numeric column
        for (key, value) in obj {
            if value.is_number() {
                log::debug!("Using numeric column for sorting: {}", key);
                return Some((key.clone(), false)); // (column, is_string)
            }
        }
    }
    log::warn!("No suitable sort column found in results");
    None
}
