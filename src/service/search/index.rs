use std::collections::HashSet;

use config::INDEX_FIELD_NAME_FOR_ALL;
use serde::{Deserialize, Serialize};
use sqlparser::ast::{BinaryOperator, Expr, FunctionArguments};
use tantivy::{
    query::{BooleanQuery, Occur, Query, RegexQuery, TermQuery},
    schema::{Field, IndexRecordOption, Schema},
    Term,
};

use super::utils::{conjunction, is_field, is_value, split_conjunction, trim_quotes};

pub fn get_index_condition_from_expr(
    index_fields: &HashSet<String>,
    expr: &Expr,
) -> (IndexCondition, Option<Expr>) {
    let mut other_expr = Vec::new();
    let expr_list = split_conjunction(expr);
    let mut index_condition = IndexCondition::default();
    for e in expr_list {
        if !is_expr_valid_for_index(e, index_fields) {
            other_expr.push(e);
            continue;
        }

        let multi_condition = Condition::from_expr(e);
        index_condition.add_condition(multi_condition);
    }

    let new_expr = conjunction(other_expr);
    (index_condition, new_expr)
}

// note the condition in IndexCondition is connection by AND operator
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IndexCondition {
    pub conditions: Vec<Condition>,
}

impl IndexCondition {
    pub fn new() -> Self {
        IndexCondition {
            conditions: Vec::new(),
        }
    }

    pub fn add_condition(&mut self, condition: Condition) {
        self.conditions.push(condition);
    }
}

// single condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    // field, value
    Equal(String, String),
    MatchAll(String),
    Or(Box<Condition>, Box<Condition>),
    And(Box<Condition>, Box<Condition>),
}

impl IndexCondition {
    // this only use for display the query
    pub fn to_query(&self) -> String {
        self.conditions
            .iter()
            .map(|condition| condition.to_query())
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    pub fn to_tantivy_query(&self, schema: Schema, default_fields: Field) -> Box<dyn Query> {
        let queries = self
            .conditions
            .iter()
            .map(|condition| {
                (
                    Occur::Must,
                    condition.to_tantivy_query(&schema, default_fields),
                )
            })
            .collect::<Vec<_>>();
        Box::new(BooleanQuery::from(queries))
    }

    pub fn get_fields(&self) -> HashSet<String> {
        self.conditions
            .iter()
            .fold(HashSet::new(), |mut acc, condition| {
                acc.extend(condition.get_fields());
                acc
            })
    }
}

impl Condition {
    // this only use for display the query
    pub fn to_query(&self) -> String {
        let cfg = config::get_config();
        let (prefix, suffix) = match cfg.common.full_text_search_type.as_str() {
            "eq" => ("", ""),
            "contains" => ("*", "*"),
            _ => ("", "*"),
        };
        match self {
            Condition::Equal(field, value) => format!("{}:{}", field, value),
            Condition::MatchAll(value) => format!("{}{}{}", prefix, value, suffix),
            Condition::Or(left, right) => format!("({} OR {})", left.to_query(), right.to_query()),
            Condition::And(left, right) => {
                format!("({} AND {})", left.to_query(), right.to_query())
            }
        }
    }

    pub fn from_expr(expr: &Expr) -> Self {
        match expr {
            Expr::BinaryOp {
                left,
                op: BinaryOperator::Eq,
                right,
            } => {
                let (field, value) = if is_value(left) && is_field(right) {
                    (get_field_name(right), get_value(left))
                } else if is_value(right) && is_field(left) {
                    (get_field_name(left), get_value(right))
                } else {
                    unreachable!()
                };
                Condition::Equal(field, value)
            }
            Expr::Function(func) => {
                if !func
                    .name
                    .to_string()
                    .to_lowercase()
                    .starts_with("match_all")
                {
                    unreachable!()
                }
                if let FunctionArguments::List(list) = &func.args {
                    if list.args.len() != 1 {
                        unreachable!()
                    }
                    Condition::MatchAll(trim_quotes(list.args[0].to_string().as_str()))
                } else {
                    unreachable!()
                }
            }
            Expr::BinaryOp {
                left,
                op: BinaryOperator::Or,
                right,
            } => Condition::Or(
                Box::new(Condition::from_expr(left)),
                Box::new(Condition::from_expr(right)),
            ),
            Expr::BinaryOp {
                left,
                op: BinaryOperator::And,
                right,
            } => Condition::And(
                Box::new(Condition::from_expr(left)),
                Box::new(Condition::from_expr(right)),
            ),
            Expr::Nested(expr) => Condition::from_expr(expr),
            _ => unreachable!(),
        }
    }

    pub fn to_tantivy_query(&self, schema: &Schema, default_fields: Field) -> Box<dyn Query> {
        match self {
            Condition::Equal(field, value) => {
                let field = schema.get_field(field).unwrap();
                let term = Term::from_field_text(field, value);
                Box::new(TermQuery::new(term, IndexRecordOption::Basic))
            }
            Condition::MatchAll(value) => {
                let cfg = config::get_config();
                match cfg.common.full_text_search_type.as_str() {
                    "eq" => {
                        let term = Term::from_field_text(default_fields, value);
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic))
                    }
                    "contains" => Box::new(
                        RegexQuery::from_pattern(&format!(".*{}.*", value), default_fields)
                            .unwrap(),
                    ),
                    // default to prefix search
                    _ => {
                        let term = tantivy::Term::from_field_text(default_fields, value);
                        Box::new(tantivy::query::PhrasePrefixQuery::new_with_offset(vec![(
                            0, term,
                        )]))
                    }
                }
            }
            Condition::Or(left, right) => {
                let left_query = left.to_tantivy_query(schema, default_fields);
                let right_query = right.to_tantivy_query(schema, default_fields);
                Box::new(tantivy::query::BooleanQuery::new(vec![
                    (tantivy::query::Occur::Should, left_query),
                    (tantivy::query::Occur::Should, right_query),
                ]))
            }
            Condition::And(left, right) => {
                let left_query = left.to_tantivy_query(schema, default_fields);
                let right_query = right.to_tantivy_query(schema, default_fields);
                Box::new(tantivy::query::BooleanQuery::new(vec![
                    (tantivy::query::Occur::Must, left_query),
                    (tantivy::query::Occur::Must, right_query),
                ]))
            }
        }
    }

    pub fn get_fields(&self) -> HashSet<String> {
        let mut fields = HashSet::new();
        match self {
            Condition::Equal(field, _) => {
                fields.insert(field.clone());
            }
            Condition::MatchAll(_) => {
                fields.insert(INDEX_FIELD_NAME_FOR_ALL.to_string());
            }
            Condition::Or(left, right) | Condition::And(left, right) => {
                fields.extend(left.get_fields());
                fields.extend(right.get_fields());
            }
        }
        fields
    }
}

// check if function is match_all and only have one argument
// check if binary operator is equal and one side is field and the other side is value
// and the field is in the index_fields
fn is_expr_valid_for_index(expr: &Expr, index_fields: &HashSet<String>) -> bool {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::Eq,
            right,
        } => {
            let field = if is_value(left) && is_field(right) {
                right
            } else if is_value(right) && is_field(left) {
                left
            } else {
                return false;
            };

            if !index_fields.contains(&get_field_name(field)) {
                return false;
            }
        }
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And | BinaryOperator::Or,
            right,
        } => {
            return is_expr_valid_for_index(left, index_fields)
                && is_expr_valid_for_index(right, index_fields);
        }
        Expr::Function(func) => {
            if !func
                .name
                .to_string()
                .to_lowercase()
                .starts_with("match_all")
            {
                return false;
            }
            if let FunctionArguments::List(list) = &func.args {
                if list.args.len() != 1 {
                    return false;
                }
            } else {
                return false;
            }
        }
        Expr::Nested(expr) => return is_expr_valid_for_index(expr, index_fields),
        _ => return false,
    }
    true
}

// Note: the expr should be Identifier or CompoundIdentifier
fn get_field_name(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(ident) => trim_quotes(ident.to_string().as_str()),
        Expr::CompoundIdentifier(ident) => trim_quotes(ident[1].to_string().as_str()),
        _ => unreachable!(),
    }
}

fn get_value(expr: &Expr) -> String {
    match expr {
        Expr::Value(value) => trim_quotes(value.to_string().as_str()),
        _ => unreachable!(),
    }
}
