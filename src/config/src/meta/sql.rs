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

use chrono::DateTime;
use regex::Regex;
use serde::Serialize;
use sqlparser::{
    ast::{
        BinaryOperator, Expr as SqlExpr, Function, FunctionArg, FunctionArgExpr, GroupByExpr,
        Offset as SqlOffset, OrderByExpr, Select, SelectItem, SetExpr, Statement, TableFactor,
        TableWithJoins, Value,
    },
    parser::Parser,
};

use crate::get_config;

const MAX_LIMIT: i64 = 100000;
const MAX_OFFSET: i64 = 100000;

/// parsed sql
#[derive(Clone, Debug, Serialize)]
pub struct Sql {
    pub fields: Vec<String>,           // projection, select, fields
    pub selection: Option<SqlExpr>,    // where
    pub source: String,                // table
    pub order_by: Vec<(String, bool)>, // desc: true / false
    pub group_by: Vec<String>,         // field
    pub having: bool,
    pub offset: i64,
    pub limit: i64,
    pub time_range: Option<(i64, i64)>,
    pub quick_text: Vec<(String, String, SqlOperator)>, // use text line quick filter
    pub field_alias: Vec<(String, String)>,             // alias for select field
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum SqlOperator {
    And,
    Or,
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    Nop,
}

#[derive(Clone, Debug, Serialize)]
pub enum SqlValue {
    String(String),
    Number(i64),
}

pub struct Projection<'a>(pub &'a Vec<SelectItem>);
pub struct Quicktext<'a>(pub &'a Option<SqlExpr>);
pub struct Timerange<'a>(pub &'a Option<SqlExpr>);
pub struct Source<'a>(pub &'a [TableWithJoins]);
pub struct Order<'a>(pub &'a OrderByExpr);
pub struct Group<'a>(pub &'a SqlExpr);
pub struct Offset<'a>(pub &'a SqlOffset);
pub struct Limit<'a>(pub &'a SqlExpr);
pub struct Where<'a>(pub &'a Option<SqlExpr>);

impl Sql {
    pub fn new(sql: &str) -> Result<Sql, anyhow::Error> {
        if sql.is_empty() {
            return Err(anyhow::anyhow!("SQL is empty"));
        }
        let dialect = sqlparser::dialect::GenericDialect {};
        let statement = Parser::parse_sql(&dialect, sql);
        if statement.is_err() {
            return Err(anyhow::anyhow!(statement.err().unwrap()));
        }
        let statement = statement.unwrap();
        if statement.is_empty() {
            return Err(anyhow::anyhow!("SQL is empty"));
        }
        let statement = &statement[0];
        let sql: Result<Sql, anyhow::Error> = statement.try_into();
        if sql.is_err() {
            return Err(sql.err().unwrap());
        }

        Ok(sql.unwrap())
    }
}

impl TryFrom<&Statement> for Sql {
    type Error = anyhow::Error;

    fn try_from(sql: &Statement) -> Result<Self, Self::Error> {
        match sql {
            // just take case of: query (select ... from ... where ...)
            Statement::Query(q) => {
                let offset = q.offset.as_ref();
                let limit = q.limit.as_ref();
                let orders = &q.order_by;
                let Select {
                    from: table_with_joins,
                    selection,
                    projection,
                    group_by: groups,
                    having,
                    ..
                } = match &q.body.as_ref() {
                    SetExpr::Select(statement) => statement.as_ref(),
                    _ => {
                        return Err(anyhow::anyhow!(
                            "We only support Select Query at the moment"
                        ));
                    }
                };

                let source = Source(table_with_joins).try_into()?;

                let mut order_by = Vec::new();
                for expr in orders {
                    order_by.push(Order(expr).try_into()?);
                }

                // TODO: support Group by all
                // https://docs.snowflake.com/en/sql-reference/constructs/group-by#label-group-by-all-columns
                let mut group_by = Vec::new();
                if let GroupByExpr::Expressions(exprs) = groups {
                    for expr in exprs {
                        group_by.push(Group(expr).try_into()?);
                    }
                }

                let offset = offset.map_or(0, |v| Offset(v).into());
                let limit = limit.map_or(0, |v| Limit(v).into());

                let mut fields: Vec<String> = Projection(projection).try_into()?;
                let selection = selection.as_ref().cloned();
                let field_alias: Vec<(String, String)> = Projection(projection).try_into()?;
                let time_range: Option<(i64, i64)> = Timerange(&selection).try_into()?;
                let quick_text: Vec<(String, String, SqlOperator)> =
                    Quicktext(&selection).try_into()?;
                let where_fields: Vec<String> = Where(&selection).try_into()?;

                fields.extend(where_fields);
                fields.sort();
                fields.dedup();

                Ok(Sql {
                    fields,
                    selection,
                    source,
                    order_by,
                    group_by,
                    having: having.is_some(),
                    offset,
                    limit,
                    time_range,
                    quick_text,
                    field_alias,
                })
            }
            _ => Err(anyhow::anyhow!("We only support Query at the moment")),
        }
    }
}

impl std::fmt::Display for SqlValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlValue::String(s) => write!(f, "{s}"),
            SqlValue::Number(n) => write!(f, "{n}"),
        }
    }
}

impl<'a> From<Offset<'a>> for i64 {
    fn from(offset: Offset) -> Self {
        match offset.0 {
            SqlOffset {
                value: SqlExpr::Value(Value::Number(v, _b)),
                ..
            } => {
                let mut v: i64 = v.parse().unwrap_or(0);
                if v > MAX_OFFSET {
                    v = MAX_OFFSET;
                }
                v
            }
            _ => 0,
        }
    }
}

impl<'a> From<Limit<'a>> for i64 {
    fn from(l: Limit<'a>) -> Self {
        match l.0 {
            SqlExpr::Value(Value::Number(v, _b)) => {
                let mut v: i64 = v.parse().unwrap_or(0);
                if v > MAX_LIMIT {
                    v = MAX_LIMIT;
                }
                v
            }
            _ => 0,
        }
    }
}

impl<'a> TryFrom<Source<'a>> for String {
    type Error = anyhow::Error;

    fn try_from(source: Source<'a>) -> Result<Self, Self::Error> {
        if source.0.len() != 1 {
            return Err(anyhow::anyhow!(
                "We only support single data source at the moment"
            ));
        }

        let table = &source.0[0];
        if !table.joins.is_empty() {
            return Err(anyhow::anyhow!(
                "We do not support joint data source at the moment"
            ));
        }

        match &table.relation {
            TableFactor::Table { name, .. } => Ok(name.0.first().unwrap().value.clone()),
            _ => Err(anyhow::anyhow!("We only support table")),
        }
    }
}

impl<'a> TryFrom<Order<'a>> for (String, bool) {
    type Error = anyhow::Error;

    fn try_from(order: Order) -> Result<Self, Self::Error> {
        match &order.0.expr {
            SqlExpr::Identifier(id) => Ok((id.to_string(), !order.0.asc.unwrap_or(true))),
            expr => Err(anyhow::anyhow!(
                "We only support identifier for order by, got {expr}"
            )),
        }
    }
}

impl<'a> TryFrom<Group<'a>> for String {
    type Error = anyhow::Error;

    fn try_from(g: Group) -> Result<Self, Self::Error> {
        match &g.0 {
            SqlExpr::Identifier(id) => Ok(id.to_string()),
            expr => Err(anyhow::anyhow!(
                "We only support identifier for order by, got {expr}"
            )),
        }
    }
}

impl<'a> TryFrom<Projection<'a>> for Vec<String> {
    type Error = anyhow::Error;

    fn try_from(projection: Projection<'a>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        for item in projection.0 {
            let field = match item {
                SelectItem::UnnamedExpr(expr) => get_field_name_from_expr(expr),
                SelectItem::ExprWithAlias { expr, alias: _ } => get_field_name_from_expr(expr),
                _ => None,
            };
            if let Some(field) = field {
                let field = field
                    .into_iter()
                    .map(|v| v.trim_matches(|v| v == '\'' || v == '"').to_string());
                fields.extend(field);
            }
        }
        Ok(fields)
    }
}

impl<'a> TryFrom<Projection<'a>> for Vec<(String, String)> {
    type Error = anyhow::Error;

    fn try_from(projection: Projection<'a>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        for item in projection.0 {
            if let SelectItem::ExprWithAlias { expr, alias } = item {
                fields.push((expr.to_string(), alias.to_string().replace('"', "")))
            }
        }
        Ok(fields)
    }
}

impl<'a> TryFrom<Timerange<'a>> for Option<(i64, i64)> {
    type Error = anyhow::Error;

    fn try_from(selection: Timerange<'a>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        match selection.0 {
            Some(expr) => parse_expr_for_field(
                expr,
                &SqlOperator::And,
                &get_config().common.column_timestamp,
                &mut fields,
            )?,
            None => {}
        }

        let mut time_min = Vec::new();
        for (_field, value, op, _operator) in fields.iter() {
            match op {
                SqlOperator::Gt => match parse_timestamp(value) {
                    Ok(v) => time_min.push(v.unwrap_or_default()),
                    Err(e) => return Err(e),
                },
                SqlOperator::Gte => match parse_timestamp(value) {
                    Ok(v) => time_min.push(v.unwrap_or_default()),
                    Err(e) => return Err(e),
                },
                _ => {}
            }
        }

        let mut time_max = Vec::new();
        for (_field, value, op, _operator) in fields.iter() {
            match op {
                SqlOperator::Lt => match parse_timestamp(value) {
                    Ok(v) => time_max.push(v.unwrap_or_default()),
                    Err(e) => return Err(e),
                },
                SqlOperator::Lte => match parse_timestamp(value) {
                    Ok(v) => time_max.push(v.unwrap_or_default()),
                    Err(e) => return Err(e),
                },
                _ => {}
            }
        }

        let time_min = {
            if !time_min.is_empty() {
                time_min.iter().min().unwrap().to_owned()
            } else {
                0
            }
        };
        let time_max = {
            if !time_max.is_empty() {
                time_max.iter().max().unwrap().to_owned()
            } else {
                0
            }
        };
        Ok(Some((time_min, time_max)))
    }
}

impl<'a> TryFrom<Quicktext<'a>> for Vec<(String, String, SqlOperator)> {
    type Error = anyhow::Error;

    fn try_from(selection: Quicktext<'a>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        match selection.0 {
            Some(expr) => parse_expr_for_field(expr, &SqlOperator::And, "*", &mut fields)?,
            None => {}
        }
        let fields = fields
            .iter()
            .filter_map(|(field, value, op, operator)| {
                if op == &SqlOperator::Eq || op == &SqlOperator::Like {
                    Some((
                        field.to_string(),
                        value.to_owned().to_string(),
                        operator.to_owned(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        Ok(fields)
    }
}

impl<'a> TryFrom<Where<'a>> for Vec<String> {
    type Error = anyhow::Error;

    fn try_from(selection: Where<'a>) -> Result<Self, Self::Error> {
        let mut fields = Vec::new();
        match selection.0 {
            Some(expr) => fields.extend(get_field_name_from_expr(expr).unwrap_or_default()),
            None => {}
        }
        Ok(fields)
    }
}

fn parse_timestamp(s: &SqlValue) -> Result<Option<i64>, anyhow::Error> {
    match s {
        SqlValue::String(s) => {
            let s = s.to_lowercase();
            let mut s = s.as_str();
            if s.starts_with("to_timestamp") {
                if s.starts_with("to_timestamp_seconds(") {
                    s = s.strip_prefix("to_timestamp_seconds(").unwrap();
                } else if s.starts_with("to_timestamp_micros(") {
                    s = s.strip_prefix("to_timestamp_micros(").unwrap();
                } else if s.starts_with("to_timestamp_millis(") {
                    s = s.strip_prefix("to_timestamp_millis(").unwrap();
                } else if s.starts_with("to_timestamp(") {
                    s = s.strip_prefix("to_timestamp(").unwrap();
                } else {
                    return Err(anyhow::anyhow!(
                        "Only support timestamp functions [to_timestamp|to_timestamp_millis|to_timestamp_micros|to_timestamp_seconds]"
                    ));
                }
                s = s.strip_suffix(')').unwrap();
                s = s.trim_matches(|v| v == '\'' || v == '"');
            }
            let v = DateTime::parse_from_rfc3339(s)?;
            Ok(Some(v.timestamp_micros()))
        }
        SqlValue::Number(n) => {
            if *n == 0 {
                Ok(None)
            } else if *n > (1e18 as i64) {
                Ok(Some(*n / 1000))
            } else if *n > (1e15 as i64) {
                Ok(Some(*n))
            } else if *n > (1e12 as i64) {
                Ok(Some(*n * 1000))
            } else if *n > (1e9 as i64) {
                Ok(Some(*n * 1000 * 1000))
            } else {
                Err(anyhow::anyhow!("Invalid timestamp: {}", n))
            }
        }
    }
}

fn parse_expr_for_field(
    expr: &SqlExpr,
    expr_op: &SqlOperator,
    field: &str,
    fields: &mut Vec<(String, SqlValue, SqlOperator, SqlOperator)>,
) -> Result<(), anyhow::Error> {
    // println!("! parse_expr -> {:?}", expr);
    match expr {
        SqlExpr::Nested(e) => parse_expr_for_field(e, expr_op, field, fields)?,
        SqlExpr::BinaryOp { left, op, right } => {
            let next_op: SqlOperator = op.try_into()?;
            if let SqlExpr::Identifier(ident) = &**left {
                let eq = parse_expr_check_field_name(&ident.value, field);
                if ident.value == field || (eq && next_op == SqlOperator::Eq) {
                    let val = get_value_from_expr(right);
                    if val.is_none() {
                        return Err(anyhow::anyhow!(
                            "SqlExpr::Identifier: We only support Identifier at the moment"
                        ));
                    }
                    fields.push((ident.value.to_string(), val.unwrap(), next_op, *expr_op));
                }
            } else {
                parse_expr_for_field(left, &next_op, field, fields)?;
                parse_expr_for_field(right, expr_op, field, fields)?;
            }
        }
        SqlExpr::Like {
            negated,
            expr,
            pattern,
            escape_char,
        } => {
            parse_expr_like(negated, expr, pattern, escape_char, expr_op, field, fields).unwrap();
        }
        SqlExpr::InList {
            expr,
            list,
            negated,
        } => {
            parse_expr_in_list(expr, list, negated, expr_op, field, fields).unwrap();
        }
        SqlExpr::Between {
            expr,
            negated,
            low,
            high,
        } => {
            let ret = parse_expr_between(expr, negated, low, high, field, fields);
            if ret.is_err() {
                return Err(anyhow::anyhow!("{:?}", ret.err()));
            }
        }
        SqlExpr::Function(f) => {
            let ret = parse_expr_function(f, field, fields);
            if ret.is_err() {
                return Err(anyhow::anyhow!("{:?}", ret.err()));
            }
        }
        SqlExpr::IsNull(expr) => {
            if let SqlExpr::Identifier(ident) = expr.as_ref() {
                if parse_expr_check_field_name(&ident.value, field) {
                    fields.push((
                        ident.value.to_string(),
                        SqlValue::String("".to_string()),
                        SqlOperator::Eq,
                        *expr_op,
                    ));
                }
            }
        }
        SqlExpr::IsNotNull(expr) => {
            if let SqlExpr::Identifier(ident) = expr.as_ref() {
                if parse_expr_check_field_name(&ident.value, field) {
                    fields.push((
                        ident.value.to_string(),
                        SqlValue::String("".to_string()),
                        SqlOperator::Eq,
                        *expr_op,
                    ));
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_expr_check_field_name(s: &str, field: &str) -> bool {
    if s == field {
        return true;
    }
    if field == "*" && s != "_all" && s != get_config().common.column_timestamp.clone() {
        return true;
    }

    // check function, like: to_timestamp_micros("field")
    let re = Regex::new(&format!(r#"(?i)\(['"]?{field}['"]?\)"#)).unwrap();
    re.is_match(s)
}

fn parse_expr_like(
    _negated: &bool,
    expr: &SqlExpr,
    pattern: &SqlExpr,
    _escape_char: &Option<char>,
    next_op: &SqlOperator,
    field: &str,
    fields: &mut Vec<(String, SqlValue, SqlOperator, SqlOperator)>,
) -> Result<(), anyhow::Error> {
    if let SqlExpr::Identifier(ident) = expr {
        if parse_expr_check_field_name(&ident.value, field) {
            let val = get_value_from_expr(pattern);
            if val.is_none() {
                return Err(anyhow::anyhow!(
                    "SqlExpr::Like: We only support Identifier at the moment"
                ));
            }
            fields.push((
                ident.value.to_string(),
                val.unwrap(),
                SqlOperator::Like,
                *next_op,
            ));
        }
    }
    Ok(())
}

fn parse_expr_in_list(
    expr: &SqlExpr,
    list: &[SqlExpr],
    negated: &bool,
    next_op: &SqlOperator,
    field: &str,
    fields: &mut Vec<(String, SqlValue, SqlOperator, SqlOperator)>,
) -> Result<(), anyhow::Error> {
    if *negated {
        return Ok(());
    }
    if list.is_empty() {
        return Ok(());
    }
    let field_name = get_value_from_expr(expr).unwrap().to_string();
    if !parse_expr_check_field_name(&field_name, field) {
        return Ok(());
    }
    let exprs_len = list.len();
    for (i, item) in list.iter().enumerate() {
        let op = if i + 1 == exprs_len {
            *next_op
        } else {
            SqlOperator::Or
        };
        if let Some(val) = get_value_from_expr(item) {
            fields.push((field_name.to_string(), val, SqlOperator::Eq, op));
        }
    }
    Ok(())
}

fn parse_expr_between(
    expr: &SqlExpr,
    negated: &bool,
    low: &SqlExpr,
    high: &SqlExpr,
    field: &str,
    fields: &mut Vec<(String, SqlValue, SqlOperator, SqlOperator)>,
) -> Result<(), anyhow::Error> {
    if *negated {
        return Ok(());
    }
    let f_name = get_value_from_expr(expr).unwrap().to_string();
    if parse_expr_check_field_name(&f_name, field) {
        let min = get_value_from_expr(low).unwrap();
        let max = get_value_from_expr(high).unwrap();
        fields.push((field.to_string(), min, SqlOperator::Gte, SqlOperator::And));
        fields.push((field.to_string(), max, SqlOperator::Lt, SqlOperator::And));
    }
    Ok(())
}

fn parse_expr_function(
    f: &Function,
    field: &str,
    fields: &mut Vec<(String, SqlValue, SqlOperator, SqlOperator)>,
) -> Result<(), anyhow::Error> {
    let f_name = f.name.to_string().to_lowercase();
    if ![
        "strpos",
        "contains",
        "match",
        "time_range",
        "to_timestamp",
        "to_timestamp_millis",
        "to_timestamp_micros",
        "to_timestamp_seconds",
    ]
    .contains(&f_name.as_str())
    {
        return Ok(());
    }

    // Hack time_range
    if f_name == "time_range" {
        return parse_expr_fun_time_range(f, field, fields);
    }

    if f.args.len() < 2 {
        return Ok(());
    }

    let nop = SqlOperator::And;
    let next_op = SqlOperator::And;
    let field_name = f.args.first().unwrap().to_string();
    let field_name = field_name.trim_matches(|c: char| c == '\'' || c == '"');
    if parse_expr_check_field_name(field_name, field) {
        match f.args.get(1).unwrap() {
            FunctionArg::Named {
                name: _name,
                arg,
                operator: _operator,
            } => match arg {
                FunctionArgExpr::Expr(expr) => {
                    let val = get_value_from_expr(expr);
                    if val.is_none() {
                        return Err(anyhow::anyhow!(
                            "SqlExpr::Function<Named>: We only support Identifier at the moment"
                        ));
                    }
                    fields.push((field.to_string(), val.unwrap(), nop, next_op));
                }
                _ => return Err(anyhow::anyhow!("We only support String at the moment")),
            },
            FunctionArg::Unnamed(arg) => match arg {
                FunctionArgExpr::Expr(expr) => {
                    let val = get_value_from_expr(expr);
                    if val.is_none() {
                        return Err(anyhow::anyhow!(
                            "SqlExpr::Function<Unnamed>: We only support Identifier at the moment"
                        ));
                    }
                    fields.push((field.to_string(), val.unwrap(), nop, next_op));
                }
                _ => return Err(anyhow::anyhow!("We only support String at the moment")),
            },
        }
    }

    Ok(())
}

fn parse_expr_fun_time_range(
    f: &Function,
    field: &str,
    fields: &mut Vec<(String, SqlValue, SqlOperator, SqlOperator)>,
) -> Result<(), anyhow::Error> {
    if f.args.len() != 3 {
        return Err(anyhow::anyhow!(
            "SqlExpr::Function: time_range function must have 3 arguments"
        ));
    }

    let next_op = SqlOperator::And;
    let field_name = f.args.first().unwrap().to_string();
    let field_name = field_name.trim_matches(|c: char| c == '\'' || c == '"');
    if parse_expr_check_field_name(field_name, field) {
        let mut vals = Vec::new();
        for arg in f.args.iter() {
            let val = match arg {
                FunctionArg::Named {
                    name: _name,
                    arg,
                    operator: _operator,
                } => match arg {
                    FunctionArgExpr::Expr(expr) => {
                        let val = get_value_from_expr(expr);
                        if val.is_none() {
                            return Err(anyhow::anyhow!(
                                "SqlExpr::Function<Named>: We only support Identifier at the moment"
                            ));
                        }
                        val.unwrap()
                    }
                    _ => return Err(anyhow::anyhow!("We only support String at the moment")),
                },
                FunctionArg::Unnamed(arg) => match arg {
                    FunctionArgExpr::Expr(expr) => {
                        let val = get_value_from_expr(expr);
                        if val.is_none() {
                            return Err(anyhow::anyhow!(
                                "SqlExpr::Function<Unnamed>: We only support Identifier at the moment"
                            ));
                        }
                        val.unwrap()
                    }
                    _ => return Err(anyhow::anyhow!("We only support String at the moment")),
                },
            };
            vals.push(val);
        }

        fields.push((
            field_name.to_string(),
            vals.get(1).unwrap().to_owned(),
            SqlOperator::Gte,
            next_op,
        ));
        fields.push((
            field_name.to_string(),
            vals.get(2).unwrap().to_owned(),
            SqlOperator::Lt,
            next_op,
        ));
    }

    Ok(())
}

fn get_value_from_expr(expr: &SqlExpr) -> Option<SqlValue> {
    match expr {
        SqlExpr::Identifier(ident) => Some(SqlValue::String(ident.value.to_string())),
        SqlExpr::Value(value) => match value {
            Value::SingleQuotedString(s) => Some(SqlValue::String(s.to_string())),
            Value::DoubleQuotedString(s) => Some(SqlValue::String(s.to_string())),
            Value::Number(s, _) => Some(SqlValue::Number(s.parse::<i64>().unwrap())),
            _ => None,
        },
        SqlExpr::Function(f) => Some(SqlValue::String(f.to_string())),
        _ => None,
    }
}

fn get_field_name_from_expr(expr: &SqlExpr) -> Option<Vec<String>> {
    match expr {
        SqlExpr::Identifier(ident) => Some(vec![ident.value.to_string()]),
        SqlExpr::BinaryOp { left, op: _, right } => {
            let mut fields = Vec::new();
            if let Some(v) = get_field_name_from_expr(left) {
                fields.extend(v);
            }
            if let Some(v) = get_field_name_from_expr(right) {
                fields.extend(v);
            }
            (!fields.is_empty()).then_some(fields)
        }
        SqlExpr::Function(f) => {
            let mut fields = Vec::with_capacity(f.args.len());
            for arg in f.args.iter() {
                match arg {
                    FunctionArg::Named {
                        name: _name,
                        arg: FunctionArgExpr::Expr(expr),
                        operator: _operator,
                    } => {
                        if let Some(v) = get_field_name_from_expr(expr) {
                            fields.extend(v);
                        }
                    }
                    FunctionArg::Unnamed(FunctionArgExpr::Expr(expr)) => {
                        if let Some(v) = get_field_name_from_expr(expr) {
                            fields.extend(v);
                        }
                    }
                    _ => {}
                }
            }
            (!fields.is_empty()).then_some(fields)
        }
        SqlExpr::Nested(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsFalse(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsNotFalse(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsTrue(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsNotTrue(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsNull(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsNotNull(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsUnknown(expr) => get_field_name_from_expr(expr),
        SqlExpr::IsNotUnknown(expr) => get_field_name_from_expr(expr),
        SqlExpr::InList { expr, .. } => get_field_name_from_expr(expr),
        SqlExpr::Between { expr, .. } => get_field_name_from_expr(expr),
        SqlExpr::Like { expr, pattern, .. } | SqlExpr::ILike { expr, pattern, .. } => {
            let mut fields = Vec::new();
            if let Some(expr) = get_field_name_from_expr(expr) {
                fields.extend(expr);
            }
            if let Some(pattern) = get_field_name_from_expr(pattern) {
                fields.extend(pattern);
            }
            (!fields.is_empty()).then_some(fields)
        }
        SqlExpr::Cast { expr, .. } | SqlExpr::TryCast { expr, .. } => {
            get_field_name_from_expr(expr)
        }
        SqlExpr::Case {
            operand: _,
            conditions,
            results: _,
            else_result: _,
        } => {
            let mut fields = Vec::new();
            for expr in conditions.iter() {
                if let Some(v) = get_field_name_from_expr(expr) {
                    fields.extend(v);
                }
            }
            (!fields.is_empty()).then_some(fields)
        }
        SqlExpr::AtTimeZone { timestamp, .. } => get_field_name_from_expr(timestamp),
        SqlExpr::Extract { expr, .. } => get_field_name_from_expr(expr),
        SqlExpr::MapAccess { column, .. } => get_field_name_from_expr(column),
        SqlExpr::ArrayIndex { obj, .. } => get_field_name_from_expr(obj),
        _ => None,
    }
}

impl TryFrom<&BinaryOperator> for SqlOperator {
    type Error = anyhow::Error;
    fn try_from(value: &BinaryOperator) -> Result<Self, Self::Error> {
        match value {
            BinaryOperator::And => Ok(SqlOperator::And),
            BinaryOperator::Or => Ok(SqlOperator::Or),
            BinaryOperator::Eq => Ok(SqlOperator::Eq),
            BinaryOperator::NotEq => Ok(SqlOperator::Neq),
            BinaryOperator::Gt => Ok(SqlOperator::Gt),
            BinaryOperator::GtEq => Ok(SqlOperator::Gte),
            BinaryOperator::Lt => Ok(SqlOperator::Lt),
            BinaryOperator::LtEq => Ok(SqlOperator::Lte),
            _ => Err(anyhow::anyhow!(
                "We only support BinaryOperator at the moment"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sql_works() {
        let table = "index.1.2022";
        let sql = format!(
            "select a, b, c from \"{}\" where a=1 and b=1 or c=1 order by c desc limit 5 offset 10",
            table
        );
        let dialect = sqlparser::dialect::GenericDialect {};
        let statement = &Parser::parse_sql(&dialect, sql.as_ref()).unwrap()[0];
        let sql: Sql = statement.try_into().unwrap();
        assert_eq!(sql.source, table);
        assert_eq!(sql.limit, 5);
        assert_eq!(sql.offset, 10);
        assert_eq!(sql.order_by, vec![("c".into(), true)]);
        assert_eq!(sql.fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_sql_new() {
        let table = "index.1.2022";
        let sql = format!(
            "select a, b, c from \"{}\" where a=1 and b=1 or c=1 order by c desc limit 5 offset 10",
            table
        );
        let local_sql: Sql = Sql::new(sql.as_str()).unwrap();
        assert_eq!(local_sql.source, table);
        assert_eq!(local_sql.limit, 5);
        assert_eq!(local_sql.offset, 10);
        assert_eq!(local_sql.order_by, vec![("c".into(), true)]);
        assert_eq!(local_sql.fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_sql_parse() {
        let sqls = [
            ("select * from table1", true),
            ("select * from table1 where a=1", true),
            ("select * from table1 where a='b'", true),
            ("select * from table1 where a='b' limit 10 offset 10", true),
            ("select * from table1 where a='b' group by abc", true),
            (
                "select * from table1 where a='b' group by abc having count(*) > 19",
                true,
            ),
            ("select * from table1, table2 where a='b'", false),
            (
                "select * from table1 left join table2 on table1.a=table2.b where a='b'",
                false,
            ),
            (
                "select * from table1 union select * from table2 where a='b'",
                false,
            ),
        ];
        for (sql, ok) in sqls {
            let ret = Sql::new(sql);
            assert_eq!(ret.is_ok(), ok);
        }
    }

    #[test]
    fn test_sql_parse_timestamp() {
        let val = 1666093521151350;
        let ts_val = SqlValue::Number(val);
        let ts = parse_timestamp(&ts_val).unwrap().unwrap();
        let ts_str_val = SqlValue::String("to_timestamp1()".to_string());
        let ts_str = parse_timestamp(&ts_str_val);
        assert_eq!(ts, val);
        assert!(ts_str.is_err());
    }

    #[test]
    fn test_sql_parse_timerange() {
        let samples = [
            ("select * from tbl where ts in (1, 2, 3)", (0,0)),
            ("select * from tbl where _timestamp >= 1666093521151350", (1666093521151350,0)),
            ("select * from tbl where _timestamp >= 1666093521151350 AND _timestamp < 1666093521151351", (1666093521151350,1666093521151351)),
            ("select * from tbl where a=1 AND _timestamp>=1666093521151350 AND _timestamp < 1666093521151351", (1666093521151350,1666093521151351)),
            ("select * from tbl where a=1 AND b = 2 AND _timestamp>=1666093521151350 AND _timestamp < 1666093521151351", (1666093521151350,1666093521151351)),
            (r#"select * from tbl where "a"=1 AND b = 2 AND (_timestamp>=1666093521151350 AND _timestamp < 1666093521151351)"#, (1666093521151350,1666093521151351)),
            ("select * from tbl where b = 2 AND (_timestamp>=1666093521151350 AND _timestamp < 1666093521151351)", (1666093521151350,1666093521151351)),
            ("select * from tbl where b = 2 AND _timestamp>=1666093521151350 AND _timestamp < 1666093521151351", (1666093521151350,1666093521151351)),
            ("select * from tbl where (_timestamp>=1666093521151350 AND _timestamp < 1666093521151351)", (1666093521151350,1666093521151351)),
            ("select * from tbl where _timestamp>=1666093521151350 AND _timestamp < 1666093521151351", (1666093521151350,1666093521151351)),
            ("select * from tbl where a=1 AND b = 2 AND (_timestamp BETWEEN 1666093521151350 AND 1666093521151351)", (1666093521151350,1666093521151351)),
            ("select * from tbl where b = 2 AND (_timestamp BETWEEN 1666093521151350 AND 1666093521151351)", (1666093521151350,1666093521151351)),
            ("select * from tbl where (_timestamp BETWEEN 1666093521151350 AND 1666093521151351)", (1666093521151350,1666093521151351)),
            ("select * from tbl where _timestamp BETWEEN 1666093521151350 AND 1666093521151351", (1666093521151350,1666093521151351)),
            (r#"select * from tbl where time_range("_timestamp", '2022-10-19T15:19:24.587Z','2022-10-19T15:34:24.587Z')"#,(1666192764587000,1666193664587000))].to_vec();

        for (sql, (expected_t1, expected_t2)) in samples {
            let (actual_t1, actual_t2) = Sql::new(sql).unwrap().time_range.unwrap();
            assert_eq!(actual_t1, expected_t1);
            if expected_t2 != 0 {
                assert_eq!(actual_t2, expected_t2);
            }
        }
    }

    #[test]
    fn test_sql_parse_fields() {
        let samples = [
            ("select * FROM tbl", vec![]),
            ("select a, b, c FROM tbl", vec!["a", "b", "c"]),
            ("select a, avg(b) FROM tbl where c=1", vec!["a", "b", "c"]),
            ("select a, a + b FROM tbl where c=1", vec!["a", "b", "c"]),
            ("select a, b + 1 FROM tbl where c=1", vec!["a", "b", "c"]),
            (
                "select a, (a + b) as d FROM tbl where c=1",
                vec!["a", "b", "c"],
            ),
            ("select a, COALESCE(b, c) FROM tbl", vec!["a", "b", "c"]),
            ("select a, COALESCE(b, 'c') FROM tbl", vec!["a", "b"]),
            ("select a, COALESCE(b, \"c\") FROM tbl", vec!["a", "b", "c"]),
            ("select a, b + 1 FROM tbl where c>1", vec!["a", "b", "c"]),
            (
                "select a, b FROM tbl  where (a >= 3 AND a < 10) AND (c='abc' AND d='abcd' AND str_match(b, 'Error')) order by a desc LIMIT 250",
                vec!["a", "b", "c", "d"],
            ),
            (
                "select _timestamp, message FROM tbl  where (_timestamp >= 17139560000000 AND _timestamp < 171395076000000) AND (pid='2fs93s' AND stream_id='asdf834sdf2' AND str_match(message, 'Error')) order by _timestamp desc LIMIT 250",
                vec!["_timestamp", "message", "pid", "stream_id"],
            ),
            ("SELECT a FROM tbl WHERE b IS FALSE", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS NOT FALSE", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS TRUE", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS NOT TRUE", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS NULL", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS NOT NULL", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS UNKNOWN", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IS NOT UNKNOWN", vec!["a", "b"]),
            ("SELECT a FROM tbl WHERE b IN (1, 2, 3)", vec!["a", "b"]),
            (
                "SELECT a FROM tbl WHERE b BETWEEN 10 AND 20",
                vec!["a", "b"],
            ),
            ("SELECT a FROM tbl WHERE b LIKE '%pattern%'", vec!["a", "b"]),
            (
                "SELECT a FROM tbl WHERE b ILIKE '%pattern%'",
                vec!["a", "b"],
            ),
            ("SELECT CAST(a AS INTEGER) FROM tbl", vec!["a"]),
            ("SELECT TRY_CAST(a AS INTEGER) FROM tbl", vec!["a"]),
            ("SELECT a AT TIME ZONE 'UTC' FROM tbl", vec!["a"]),
            ("SELECT EXTRACT(YEAR FROM a) FROM tbl", vec!["a"]),
            ("SELECT map['key'] from tbl", vec!["map"]),
            (
                "SELECT a FROM tbl WHERE c IS NOT NULL AND (b IS FALSE OR d > 3)",
                vec!["a", "b", "c", "d"],
            ),
            (
                "select _timestamp, message FROM tbl where  (pid='2fs93s' or stream_id='asdf834sdf2') AND str_match(new_message, 'Error')",
                vec!["_timestamp", "message", "new_message", "pid", "stream_id"],
            ),
            (
                "SELECT COUNT(CASE WHEN k8s_namespace_name IS NULL THEN 0 ELSE 1 END) AS null_count FROM default1",
                vec!["k8s_namespace_name"],
            ),
            (
                "SELECT COUNT(CASE WHEN k8s_namespace_name IS NULL THEN 0 ELSE 1 END) AS null_count FROM default1 WHERE a=1",
                vec!["a", "k8s_namespace_name"],
            ),
        ];
        for (sql, fields) in samples {
            let actual = Sql::new(sql).unwrap().fields;
            assert_eq!(actual, fields);
        }
    }
}
