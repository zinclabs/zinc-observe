use std::ops::ControlFlow;

use config::meta::search::{SearchEventContext, SearchEventType};
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

/// Calculate the actual queried range for a search request omitting the result cache ratio.
pub(crate) fn calc_queried_range(start_time: i64, end_time: i64, result_cache_ratio: usize) -> f64 {
    let range = (end_time - start_time) as f64 / 3600000.0; // hours
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
}
