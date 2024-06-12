use sqlparser::{
    ast::{Expr, Function, Query, SelectItem, SetExpr, Statement},
    dialect::GenericDialect,
    parser::Parser,
};

pub fn is_aggregate_query(query: &str) -> Result<bool, sqlparser::parser::ParserError> {
    let ast = Parser::parse_sql(&GenericDialect {}, query)?;

    for statement in ast {
        if let Statement::Query(query) = statement {
            if is_aggregate_in_select(query) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn is_aggregate_in_select(query: Box<Query>) -> bool {
    if let SetExpr::Select(ref select) = *query.body {
        for select_item in &select.projection {
            if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, alias: _ } =
                select_item
            {
                if is_aggregate_expression(expr) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_aggregate_expression(expr: &Expr) -> bool {
    match expr {
        Expr::Function(Function {
            name,
            args: _,
            distinct: _,
            ..
        }) => AGGREGATE_UDF_LIST.contains(&name.to_string().to_lowercase().as_str()),

        _ => false,
    }
}

const AGGREGATE_UDF_LIST: [&str; 7] = [
    "min",
    "max",
    "count",
    "avg",
    "sum",
    "array_agg",
    "approx_percentile_cont",
];
