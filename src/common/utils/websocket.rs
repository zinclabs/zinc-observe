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

use std::ops::ControlFlow;

use config::meta::{
    search::{SearchEventContext, SearchEventType},
    stream::StreamType,
    websocket::SearchResultType,
};
use infra::errors::Error;
use sqlparser::{
    ast::{visit_statements_mut, Expr, FunctionArguments, Statement},
    dialect::PostgreSqlDialect,
    parser::Parser,
};

#[inline(always)]
pub(crate) fn get_search_type_from_ws_req(
    search_event_type: &SearchEventType,
    search_event_context: SearchEventContext,
) -> Option<SearchEventContext> {
    match search_event_type {
        SearchEventType::Dashboards => Some(SearchEventContext::with_dashboard(
            search_event_context.dashboard_id,
            search_event_context.dashboard_name,
            search_event_context.dashboard_folder_id,
            search_event_context.dashboard_folder_name,
        )),
        SearchEventType::Alerts => Some(SearchEventContext::with_alert(
            search_event_context.alert_key,
        )),
        SearchEventType::Reports => Some(SearchEventContext::with_report(
            search_event_context.report_key,
        )),
        _ => None,
    }
}

/// Calculates the actual queried range for a search request, adjusted for the result cache ratio.
///
/// This function computes the effective queried time range in hours by considering the total time
/// range (in microseconds) and reducing it based on the percentage of results cached.
///
/// # Parameters
/// - `start_time` (`i64`): Start time in microseconds since the epoch.
/// - `end_time` (`i64`): End time in microseconds since the epoch.
/// - `result_cache_ratio` (`usize`): Percentage of results cached (0 to 100).
///
/// # Returns
/// - `f64`: The effective queried range in hours, reduced by the cache ratio.
pub(crate) fn calc_queried_range(start_time: i64, end_time: i64, result_cache_ratio: usize) -> f64 {
    let result_cache_ratio = result_cache_ratio.min(100); // ensure ratio in between 0 and 100
    let range = (end_time - start_time) as f64 / 3_600_000_000.0; // convert microseconds to hours
    range * (1.0 - result_cache_ratio as f64 / 100.0)
}

/// Updates the `HISTOGRAM` function in a SQL query to include or modify the interval.
pub(crate) fn update_histogram_interval_in_query(
    sql: &str,
    histogram_interval: i64,
) -> Result<String, Error> {
    let mut statement = Parser::parse_sql(&PostgreSqlDialect {}, sql)
        .map_err(|e| Error::Message(e.to_string()))?
        .pop()
        .unwrap();

    visit_statements_mut(&mut statement, |stmt| {
        if let Statement::Query(ref mut query) = stmt {
            if let sqlparser::ast::SetExpr::Select(select) = query.body.as_mut() {
                for projection in &mut select.projection {
                    match projection {
                        // Handle expressions with aliases
                        sqlparser::ast::SelectItem::ExprWithAlias { expr, alias: _ } => {
                            update_histogram_in_expr(expr, histogram_interval);
                        }
                        // Handle unnamed expressions
                        sqlparser::ast::SelectItem::UnnamedExpr(expr) => {
                            update_histogram_in_expr(expr, histogram_interval);
                        }
                        // Ignore other types of projections
                        _ => {}
                    }
                }
            }
        }
        ControlFlow::Break(())
    });

    Ok(statement.to_string())
}

/// Updates the `HISTOGRAM` function in an expression to include or modify the interval.
fn update_histogram_in_expr(expr: &mut Expr, histogram_interval: i64) {
    if let Expr::Function(func) = expr {
        if func.name.to_string().to_lowercase() == "histogram" {
            if let FunctionArguments::List(list) = &mut func.args {
                let mut args = list.args.iter();
                // first is field
                let _ = args.next();
                // second is interval
                if args.next().is_none() {
                    let interval_value = format!("{} seconds", histogram_interval);
                    list.args.push(sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(Expr::Value(
                            sqlparser::ast::Value::SingleQuotedString(interval_value),
                        )),
                    ));
                }
            }
        }
    }
}

/// Get the maximum query range for a list of streams in hours
pub async fn get_max_query_range(
    stream_names: &[String],
    org_id: &str,
    stream_type: StreamType,
) -> i64 {
    futures::future::join_all(
        stream_names
            .iter()
            .map(|stream_name| infra::schema::get_settings(org_id, stream_name, stream_type)),
    )
    .await
    .into_iter()
    .filter_map(|settings| settings.map(|s| s.max_query_range))
    .max()
    .unwrap_or(0)
}

/// Calculates the ratio of cache hits to search hits in the accumulated search results.
pub fn _calc_result_cache_ratio(accumulated_results: &[SearchResultType]) -> usize {
    let (search_hits, cache_hits) =
        accumulated_results
            .iter()
            .fold((0, 0), |(search_hits, cache_hits), result| match result {
                SearchResultType::Search(s) => (search_hits + s.hits.len(), cache_hits),
                SearchResultType::Cached(c) => (search_hits, cache_hits + c.hits.len()),
            });

    let total_hits = search_hits + cache_hits;
    if total_hits == 0 {
        return 0; // avoid division by zero
    }
    ((cache_hits as f64 / total_hits as f64) * 100.0).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_histogram_interval_if_missing() {
        let sql = "SELECT histogram(_timestamp) AS 'x_axis_1', count(gold_medals) AS 'y_axis_1', count(silver_medals) AS 'y_axis_2', count(total_medals) AS 'y_axis_3' FROM 'default' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC";
        let histogram_interval = 3600;

        let updated_sql = update_histogram_interval_in_query(sql, histogram_interval).unwrap();
        let expected = "SELECT histogram(_timestamp, '3600 seconds') AS 'x_axis_1', count(gold_medals) AS 'y_axis_1', count(silver_medals) AS 'y_axis_2', count(total_medals) AS 'y_axis_3' FROM 'default' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC";

        assert_eq!(updated_sql, expected);
    }

    #[test]
    fn test_do_not_update_existing_histogram_interval() {
        let sql = "SELECT histogram(_timestamp, '3600 seconds') AS 'x_axis_1', count(gold_medals) AS 'y_axis_1', count(silver_medals) AS 'y_axis_2', count(total_medals) AS 'y_axis_3' FROM 'default' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC";
        let histogram_interval = 3600;

        let updated_sql = update_histogram_interval_in_query(sql, histogram_interval).unwrap();
        let expected = "SELECT histogram(_timestamp, '3600 seconds') AS 'x_axis_1', count(gold_medals) AS 'y_axis_1', count(silver_medals) AS 'y_axis_2', count(total_medals) AS 'y_axis_3' FROM 'default' GROUP BY x_axis_1 ORDER BY x_axis_1 ASC";

        assert_eq!(sql, expected);
    }

    #[test]
    fn test_calc_queried_range() {
        // Provided values
        let start_time: i64 = 1733290663000000;
        let end_time: i64 = 1733290783000000;
        let result_cache_ratio: usize = 0; // No caching
        let mut remaining_query_range = 4.0; // Initial remaining query range in hours

        // Expected partition duration
        let partition_duration = (end_time - start_time) as f64 / 3_600_000_000.0; // Convert microseconds to hours
        assert!(
            (partition_duration - 0.0333).abs() < 0.0001,
            "Partition duration should be approximately 0.0333 hours"
        );

        // Calculate the queried range
        let queried_range = calc_queried_range(start_time, end_time, result_cache_ratio);
        assert!(
            (queried_range - partition_duration).abs() < f64::EPSILON,
            "Queried range should equal partition duration when cache ratio is 0"
        );

        // Update the remaining query range
        remaining_query_range -= queried_range;

        // Check the updated remaining query range
        assert!(
            (remaining_query_range - (4.0 - queried_range)).abs() < f64::EPSILON,
            "Updated remaining query range should be correct"
        );
    }
}
