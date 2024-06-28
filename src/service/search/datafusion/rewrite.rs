// Copyright 2024 Zinc Labs Inc.
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

// rewrite count(distinct(field)) query

use core::ops::ControlFlow;

use config::FxIndexSet;
use datafusion::error::Result;
use itertools::Itertools;
use sqlparser::{
    ast::{
        Expr, Function, FunctionArguments, GroupByExpr, Ident, ObjectName, Query, TableFactor,
        TableWithJoins, VisitMut, VisitorMut,
    },
    dialect::GenericDialect,
    parser::Parser,
};

const AGGREGATE_UDF_LIST: [&str; 7] = [
    "min",
    "max",
    "count",
    "avg",
    "sum",
    "array_agg",
    "approx_percentile_cont",
];

pub fn rewrite_count_distinct_sql(sql: &str, is_first_phase: bool) -> Result<String> {
    let mut statements = Parser::parse_sql(&GenericDialect {}, sql)?;
    if is_first_phase {
        statements.visit(&mut Rewrite {
            is_first_phase: true,
        });
    } else {
        statements.visit(&mut Rewrite {
            is_first_phase: false,
        });
    }
    Ok(statements[0].to_string())
}

pub fn rewrite_count_distinct_merge_sql(sql: &str) -> Result<String> {
    let mut statements = Parser::parse_sql(&GenericDialect {}, sql)?;
    statements.visit(&mut RewriteFinal);
    Ok(statements[0].to_string())
}

// A visitor that count(distinct(field)) query
struct Rewrite {
    is_first_phase: bool,
}

// Visit each expression after its children have been visited
impl VisitorMut for Rewrite {
    type Break = ();

    /// Invoked for any queries that appear in the AST before visiting children
    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        // remove the limit clause
        query.limit = None;
        query.offset = None;
        // remove the order by clause
        query.order_by = Vec::new();
        // rewrite the query body
        if let sqlparser::ast::SetExpr::Select(ref mut select) = *query.body {
            // add distinct to the projection list
            select.distinct = Some(sqlparser::ast::Distinct::Distinct);
            // remove the having clause
            select.having = None;
            // remove the group by clause
            select.group_by = GroupByExpr::Expressions(Vec::new());
            if !self.is_first_phase {
                // remove the where clause
                select.selection = None;
            }
            let mut field_names = FxIndexSet::default();
            let mut delete_idx = Vec::new();
            let empty_vec = vec![];
            for (idx, select_item) in &mut select.projection.iter_mut().enumerate() {
                match select_item {
                    sqlparser::ast::SelectItem::ExprWithAlias { expr, alias } => {
                        match expr {
                            Expr::Function(Function { name, args, .. }) => {
                                // extract field name from the aggregate function
                                if AGGREGATE_UDF_LIST
                                    .contains(&name.to_string().to_lowercase().as_str())
                                {
                                    let distinct = match &args {
                                        FunctionArguments::List(args) => matches!(
                                            args.duplicate_treatment,
                                            Some(sqlparser::ast::DuplicateTreatment::Distinct,)
                                        ),
                                        _ => false,
                                    };
                                    let args = match &args {
                                        FunctionArguments::List(args) => &args.args,
                                        _ => &empty_vec,
                                    };
                                    // check if aggregate function is count and distinct is false
                                    if name.to_string().to_lowercase() == "count" && !(distinct) {
                                        select.distinct = None;
                                    }
                                    // check if the field name is already in the projection list
                                    let field_name =
                                        remove_brackets(format!("{}", args[0]).as_str());
                                    if field_names.contains(&field_name) {
                                        delete_idx.push(idx);
                                        continue;
                                    }
                                    field_names.insert(field_name.clone());
                                    *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                        Expr::Identifier(Ident::new(field_name)),
                                    )
                                } else if !self.is_first_phase {
                                    *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                        Expr::Identifier(alias.clone()),
                                    );
                                }
                            }
                            _ => {
                                if !self.is_first_phase {
                                    *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                        Expr::Identifier(alias.clone()),
                                    );
                                }
                            }
                        }
                    }
                    // extrace function paraments from the projection list
                    sqlparser::ast::SelectItem::UnnamedExpr(expr) => {
                        if let Expr::Function(Function { name, args, .. }) = expr {
                            if !AGGREGATE_UDF_LIST
                                .contains(&name.to_string().to_lowercase().as_str())
                            {
                                if !self.is_first_phase {
                                    *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                        Expr::Identifier(Ident::new(format!("\"{}\"", expr))),
                                    )
                                } else {
                                    *select_item = sqlparser::ast::SelectItem::ExprWithAlias {
                                        expr: expr.clone(),
                                        alias: Ident::new(format!("\"{}\"", expr)),
                                    }
                                }
                            } else {
                                let distinct = match &args {
                                    FunctionArguments::List(args) => matches!(
                                        args.duplicate_treatment,
                                        Some(sqlparser::ast::DuplicateTreatment::Distinct)
                                    ),
                                    _ => false,
                                };
                                let args = match &args {
                                    FunctionArguments::List(args) => &args.args,
                                    _ => &empty_vec,
                                };
                                if name.to_string().to_lowercase() == "count" && !(distinct) {
                                    select.distinct = None;
                                }
                                let field_name = remove_brackets(format!("{}", args[0]).as_str());
                                if field_names.contains(&field_name) {
                                    delete_idx.push(idx);
                                    continue;
                                }
                                field_names.insert(field_name.clone());
                                *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                    Expr::Identifier(Ident::new(field_name)),
                                )
                            }
                        }
                    }
                    _ => {}
                }
            }
            if !delete_idx.is_empty() {
                for idx in delete_idx.iter().rev() {
                    select.projection.remove(*idx);
                }
            }
        }
        ControlFlow::Continue(())
    }
}

// A visitor that count(distinct(field)) query
struct RewriteFinal;

// Visit each expression after its children have been visited
impl VisitorMut for RewriteFinal {
    type Break = ();

    /// Invoked for any queries that appear in the AST before visiting children
    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        // rewrite the order by clause
        for expr in query.order_by.iter_mut() {
            expr.expr = match expr.expr.clone() {
                Expr::Identifier(ident) => Expr::Identifier(ident.clone()),
                _ => Expr::Identifier(Ident::new(format!("\"{}\"", expr.expr))),
            }
        }
        // rewrite the query body
        if let sqlparser::ast::SetExpr::Select(ref mut select) = *query.body {
            // remove the where clause
            select.selection = None;

            // rewrite the group by clause
            if let GroupByExpr::Expressions(ref mut exprs) = select.group_by {
                for expr in exprs.iter_mut() {
                    *expr = match expr.clone() {
                        Expr::Identifier(ident) => Expr::Identifier(ident.clone()),
                        _ => Expr::Identifier(Ident::new(format!("\"{}\"", expr))),
                    }
                }
            }

            for select_item in &mut select.projection {
                match select_item {
                    sqlparser::ast::SelectItem::ExprWithAlias { expr, alias, .. } => {
                        match expr {
                            Expr::Function(Function { name, .. }) => {
                                // extract field name from the aggregate function
                                if !AGGREGATE_UDF_LIST
                                    .contains(&name.to_string().to_lowercase().as_str())
                                {
                                    *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                        Expr::Identifier(alias.clone()),
                                    );
                                }
                            }
                            _ => {
                                *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                    Expr::Identifier(alias.clone()),
                                );
                            }
                        }
                    }
                    // extrace function paraments from the projection list
                    sqlparser::ast::SelectItem::UnnamedExpr(expr) => {
                        if let Expr::Function(Function { name, .. }) = expr {
                            if !AGGREGATE_UDF_LIST
                                .contains(&name.to_string().to_lowercase().as_str())
                            {
                                *select_item = sqlparser::ast::SelectItem::UnnamedExpr(
                                    Expr::Identifier(Ident::new(format!("\"{}\"", expr))),
                                )
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        ControlFlow::Continue(())
    }
}

fn remove_brackets(input: &str) -> String {
    if input.starts_with('(') && input.ends_with(')') {
        input[1..input.len() - 1].to_string()
    } else {
        input.to_string()
    }
}

pub fn remove_where_clause(sql: &str) -> Result<String> {
    let mut statements = Parser::parse_sql(&GenericDialect {}, sql)?;
    statements.visit(&mut RemoveWhere);
    Ok(statements[0].to_string())
}

// A visitor that remove where clause
struct RemoveWhere;

// Visit each expression after its children have been visited
impl VisitorMut for RemoveWhere {
    type Break = ();

    /// Invoked for any queries that appear in the AST before visiting children
    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let sqlparser::ast::SetExpr::Select(ref mut select) = *query.body {
            // remove the where clause
            select.selection = None;
        }
        ControlFlow::Continue(())
    }
}

pub fn add_group_by_order_by_field_to_select(sql: &str) -> Result<String> {
    let mut statements = Parser::parse_sql(&GenericDialect {}, sql)?;
    statements.visit(&mut AddGroupByOrderBy);
    Ok(statements[0].to_string())
}

// A visitor that add group by field to select
struct AddGroupByOrderBy;

// Visit each expression after its children have been visited
impl VisitorMut for AddGroupByOrderBy {
    type Break = ();

    /// Invoked for any queries that appear in the AST before visiting children
    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        // collect select field in query
        if let sqlparser::ast::SetExpr::Select(ref mut select) = *query.body {
            // early return if group by clause is empty
            if let GroupByExpr::Expressions(ref exprs) = select.group_by {
                if exprs.is_empty() && query.order_by.is_empty() {
                    return ControlFlow::Break(());
                }
            }
            // Warning: collect field name use String
            // here no log use expr because of the table identifier
            // it is maybe Some("tbl") or Some("") or NULL, it is not always equal
            let mut select_exprs = Vec::new();
            for select_item in &select.projection {
                match select_item {
                    sqlparser::ast::SelectItem::ExprWithAlias { alias, .. } => {
                        select_exprs.push(trim_quotes(alias.to_string()));
                    }
                    sqlparser::ast::SelectItem::UnnamedExpr(expr) => {
                        select_exprs.push(trim_quotes(expr.to_string()));
                    }
                    sqlparser::ast::SelectItem::Wildcard(_) => {
                        // select * from tbl, don't need add group by and order by field
                        return ControlFlow::Break(());
                    }
                    _ => {}
                }
            }
            let mut expr_list = vec![];
            // add group by field to select
            if let GroupByExpr::Expressions(ref exprs) = select.group_by {
                expr_list.extend(exprs);
            }
            // add order by field to select
            if !query.order_by.is_empty() {
                expr_list.extend(query.order_by.iter().map(|e| &e.expr));
            }
            for expr in expr_list.into_iter().sorted().dedup() {
                let expr_name = trim_quotes(expr.to_string());
                if !select_exprs.contains(&expr_name) {
                    select
                        .projection
                        .push(sqlparser::ast::SelectItem::UnnamedExpr(expr.clone()));
                }
            }
        }
        ControlFlow::Continue(())
    }
}

fn trim_quotes(input: String) -> String {
    input.trim_matches(|v| v == '\'' || v == '"').to_string()
}

pub fn replace_data_source_to_tbl(sql: &str) -> Result<String> {
    let mut statements = Parser::parse_sql(&GenericDialect {}, sql)?;
    statements.visit(&mut ReplaceDataSource);
    Ok(statements[0].to_string())
}

struct ReplaceDataSource;

impl VisitorMut for ReplaceDataSource {
    type Break = ();

    /// Invoked for any queries that appear in the AST before visiting children
    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let sqlparser::ast::SetExpr::Select(ref mut select) = *query.body {
            let source = TableWithJoins {
                relation: TableFactor::Table {
                    name: ObjectName(vec![Ident::new("tbl")]),
                    alias: None,
                    args: None,
                    with_hints: vec![],
                    version: None,
                    partitions: vec![],
                },
                joins: vec![],
            };
            select.from = vec![source];
        }
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_distinct_rewrite_phase1() {
        let sql = [
            "SELECT COUNT(DISTINCT a) FROM tbl where a > 3 limit 10",
            "SELECT COUNT(DISTINCT(a)) FROM tbl where a > 3 limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, COUNT(DISTINCT c) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, COUNT(b) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, MAX(b) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT date_bin(interval '1 day', to_timestamp_micros('2001-01-01T00:00:00'), to_timestamp('2001-01-01T00:00:00')) as x_axis_1, count(distinct(userid)) as y_axis_1  FROM segment WHERE event IN ('OpenObserve - heartbeat') GROUP BY x_axis_1 ORDER BY x_axis_1 ASC LIMIT 15",
        ];

        let excepts = [
            "SELECT DISTINCT a FROM tbl WHERE a > 3",
            "SELECT DISTINCT a FROM tbl WHERE a > 3",
            "SELECT DISTINCT a, b FROM tbl WHERE a > 3",
            "SELECT DISTINCT a, b, c FROM tbl WHERE a > 3",
            "SELECT a, b FROM tbl WHERE a > 3",
            "SELECT DISTINCT a, b FROM tbl WHERE a > 3",
            "SELECT DISTINCT date_bin(INTERVAL '1 day', to_timestamp_micros('2001-01-01T00:00:00'), to_timestamp('2001-01-01T00:00:00')) AS x_axis_1, userid FROM segment WHERE event IN ('OpenObserve - heartbeat')",
        ];
        for (sql, except) in sql.iter().zip(excepts.iter()) {
            let new_sql = rewrite_count_distinct_sql(sql, true).unwrap();
            assert_eq!(new_sql, **except);
        }
    }

    #[test]
    fn test_count_distinct_rewrite_phase2() {
        let sql = [
            "SELECT COUNT(DISTINCT a) FROM tbl where a > 3 limit 10",
            "SELECT COUNT(DISTINCT(a)) FROM tbl where a > 3 limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, COUNT(DISTINCT c) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, COUNT(b) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, MAX(b) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT date_bin(interval '1 day', to_timestamp_micros('2001-01-01T00:00:00'), to_timestamp('2001-01-01T00:00:00')) as x_axis_1, count(distinct(userid)) as y_axis_1  FROM segment WHERE event IN ('OpenObserve - heartbeat') GROUP BY x_axis_1 ORDER BY x_axis_1 ASC LIMIT 15",
        ];

        let excepts = [
            "SELECT DISTINCT a FROM tbl",
            "SELECT DISTINCT a FROM tbl",
            "SELECT DISTINCT a, b FROM tbl",
            "SELECT DISTINCT a, b, c FROM tbl",
            "SELECT a, b FROM tbl",
            "SELECT DISTINCT a, b FROM tbl",
            "SELECT DISTINCT x_axis_1, userid FROM segment",
        ];
        for (sql, except) in sql.iter().zip(excepts.iter()) {
            let new_sql = rewrite_count_distinct_sql(sql, false).unwrap();
            assert_eq!(new_sql, **except);
        }
    }

    #[test]
    fn test_count_distinct_rewrite_phase3() {
        let sql = [
            "SELECT COUNT(DISTINCT a) FROM tbl where a > 3 limit 10",
            "SELECT COUNT(DISTINCT(a)) FROM tbl where a > 3 limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, COUNT(DISTINCT c) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, COUNT(b) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT a, COUNT(DISTINCT b) as cnt, MAX(b) FROM tbl where a > 3 group by a having cnt > 1 order by cnt limit 10",
            "SELECT date_bin(interval '1 day', to_timestamp_micros('2001-01-01T00:00:00'), to_timestamp('2001-01-01T00:00:00')) as x_axis_1, count(distinct(userid)) as y_axis_1  FROM segment WHERE event IN ('OpenObserve - heartbeat') GROUP BY x_axis_1 ORDER BY x_axis_1 ASC LIMIT 15",
        ];

        let excepts = [
            "SELECT COUNT(DISTINCT a) FROM tbl LIMIT 10",
            "SELECT COUNT(DISTINCT (a)) FROM tbl LIMIT 10",
            "SELECT a, COUNT(DISTINCT b) AS cnt FROM tbl GROUP BY a HAVING cnt > 1 ORDER BY cnt LIMIT 10",
            "SELECT a, COUNT(DISTINCT b) AS cnt, COUNT(DISTINCT c) FROM tbl GROUP BY a HAVING cnt > 1 ORDER BY cnt LIMIT 10",
            "SELECT a, COUNT(DISTINCT b) AS cnt, COUNT(b) FROM tbl GROUP BY a HAVING cnt > 1 ORDER BY cnt LIMIT 10",
            "SELECT a, COUNT(DISTINCT b) AS cnt, MAX(b) FROM tbl GROUP BY a HAVING cnt > 1 ORDER BY cnt LIMIT 10",
            "SELECT x_axis_1, count(DISTINCT (userid)) AS y_axis_1 FROM segment GROUP BY x_axis_1 ORDER BY x_axis_1 ASC LIMIT 15",
        ];
        for (sql, except) in sql.iter().zip(excepts.iter()) {
            let new_sql = rewrite_count_distinct_merge_sql(sql).unwrap();
            assert_eq!(new_sql, **except);
        }
    }

    #[test]
    fn test_add_group_by() {
        let sql = [
            "SELECT count(*) as cnt FROM default group by k8s_namespace_name",
            "SELECT count(*) FROM default group by k8s_namespace_name",
            "SELECT k8s_namespace_name as k8s, count(*) FROM default group by k8s",
            "SELECT k8s_namespace_name, count(*) FROM default group by k8s_namespace_name",
            "SELECT * FROM default ORDER BY k8s_namespace_name",
            "SELECT log FROM default ORDER BY k8s_namespace_name",
            "SELECT count(*) as cnt FROM default GROUP BY log ORDER BY log",
            "SELECT log, k8s_namespace_name FROM default ORDER BY k8s_namespace_name",
            "SELECT * FROM default where a = b",
            "SELECT a, b, c FROM default",
            "SELECT avg(resource_duration / 1000000) AS y_axis_1, SPLIT_PART(resource_url, '?', 1) AS x_axis_1 FROM tbl WHERE (_timestamp >= 1710404419324000 AND _timestamp < 1710490819324000) AND (resource_duration >= 0) GROUP BY x_axis_1 ORDER BY x_axis_1 LIMIT 10",
            "SELECT kubernetes_namespace_name, count(*) FROM default GROUP BY kubernetes_namespace_name, stream order by kubernetes_namespace_name, stream ",
        ];

        let excepts = [
            "SELECT count(*) AS cnt, k8s_namespace_name FROM default GROUP BY k8s_namespace_name",
            "SELECT count(*), k8s_namespace_name FROM default GROUP BY k8s_namespace_name",
            "SELECT k8s_namespace_name AS k8s, count(*) FROM default GROUP BY k8s",
            "SELECT k8s_namespace_name, count(*) FROM default GROUP BY k8s_namespace_name",
            "SELECT * FROM default ORDER BY k8s_namespace_name",
            "SELECT log, k8s_namespace_name FROM default ORDER BY k8s_namespace_name",
            "SELECT count(*) AS cnt, log FROM default GROUP BY log ORDER BY log",
            "SELECT log, k8s_namespace_name FROM default ORDER BY k8s_namespace_name",
            "SELECT * FROM default WHERE a = b",
            "SELECT a, b, c FROM default",
            "SELECT avg(resource_duration / 1000000) AS y_axis_1, SPLIT_PART(resource_url, '?', 1) AS x_axis_1 FROM tbl WHERE (_timestamp >= 1710404419324000 AND _timestamp < 1710490819324000) AND (resource_duration >= 0) GROUP BY x_axis_1 ORDER BY x_axis_1 LIMIT 10",
            "SELECT kubernetes_namespace_name, count(*), stream FROM default GROUP BY kubernetes_namespace_name, stream ORDER BY kubernetes_namespace_name, stream",
        ];
        for (sql, except) in sql.iter().zip(excepts.iter()) {
            let new_sql = add_group_by_order_by_field_to_select(sql).unwrap();
            assert_eq!(new_sql, except.to_string());
        }
    }
}
