use std::{
    collections::HashSet,
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use config::{utils::tantivy::tokenizer::o2_collect_tokens, INDEX_FIELD_NAME_FOR_ALL};
use datafusion::{
    arrow::datatypes::{DataType, SchemaRef},
    logical_expr::Operator,
    physical_expr::ScalarFunctionExpr,
    physical_plan::{
        expressions::{BinaryExpr, Column, InListExpr, LikeExpr, Literal},
        PhysicalExpr,
    },
    scalar::ScalarValue,
};
use serde::{Deserialize, Serialize};
use sqlparser::ast::{BinaryOperator, Expr, FunctionArguments};
use tantivy::{
    query::{BooleanQuery, FuzzyTermQuery, Occur, PhrasePrefixQuery, Query, RegexQuery, TermQuery},
    schema::{Field, IndexRecordOption, Schema},
    Term,
};

use super::{
    datafusion::udf::fuzzy_match_udf,
    utils::{is_field, is_value, split_conjunction, trim_quotes},
};

pub fn get_index_condition_from_expr(
    index_fields: &HashSet<String>,
    expr: &Expr,
) -> (Option<IndexCondition>, Option<Expr>) {
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

    let new_expr = super::utils::conjunction(other_expr);
    if index_condition.conditions.is_empty() {
        (None, new_expr)
    } else {
        (Some(index_condition), new_expr)
    }
}

// note the condition in IndexCondition is connection by AND operator
#[derive(Default, Clone, Serialize, Deserialize)]
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

impl Debug for IndexCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_query())
    }
}

// single condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    // field, value
    Equal(String, String),
    In(String, Vec<String>),
    MatchAll(String),
    FuzzyMatchAll(String, u8),
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

    pub fn to_tantivy_query(
        &self,
        schema: Schema,
        default_field: Option<Field>,
    ) -> anyhow::Result<Box<dyn Query>> {
        let queries = self
            .conditions
            .iter()
            .map(|condition| {
                condition
                    .to_tantivy_query(&schema, default_field)
                    .map(|condition| (Occur::Must, condition))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Box::new(BooleanQuery::from(queries)))
    }

    pub fn get_tantivy_fields(&self) -> HashSet<String> {
        self.conditions
            .iter()
            .fold(HashSet::new(), |mut acc, condition| {
                acc.extend(condition.get_tantivy_fields());
                acc
            })
    }

    pub fn get_schema_fields(&self, fst_fields: &[String]) -> HashSet<String> {
        self.conditions
            .iter()
            .fold(HashSet::new(), |mut acc, condition| {
                acc.extend(condition.get_schema_fields(fst_fields));
                acc
            })
    }

    pub fn get_schema_projection(&self, schema: SchemaRef, fst_fields: &[String]) -> Vec<usize> {
        let fields = self.get_schema_fields(fst_fields);
        let mut projection = Vec::with_capacity(fields.len());
        for field in fields.iter() {
            if let Ok(index) = schema.index_of(field) {
                projection.push(index);
            }
        }
        projection
    }

    pub fn to_physical_expr(
        &self,
        schema: &arrow_schema::Schema,
        fst_fields: &[String],
    ) -> Result<Arc<dyn PhysicalExpr>, anyhow::Error> {
        Ok(conjunction(
            self.conditions
                .iter()
                .map(|condition| condition.to_physical_expr(schema, fst_fields))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl Condition {
    // this only use for display the query
    pub fn to_query(&self) -> String {
        match self {
            Condition::Equal(field, value) => format!("{}={}", field, value),
            Condition::In(field, values) => format!("{} IN ({})", field, values.join(",")),
            Condition::MatchAll(value) => format!("{}:{}", INDEX_FIELD_NAME_FOR_ALL, value),
            Condition::FuzzyMatchAll(value, distance) => format!(
                "{}:fuzzy({}, {})",
                INDEX_FIELD_NAME_FOR_ALL, value, distance
            ),
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
            Expr::InList {
                expr,
                list,
                negated: _,
            } => {
                let field = get_field_name(expr);
                let values = list.iter().map(get_value).collect();
                Condition::In(field, values)
            }
            Expr::Function(func) => {
                let fn_name = func.name.to_string().to_lowercase();
                if fn_name == "match_all" {
                    if let FunctionArguments::List(list) = &func.args {
                        if list.args.len() != 1 {
                            unreachable!()
                        }
                        Condition::MatchAll(trim_quotes(list.args[0].to_string().as_str()))
                    } else {
                        unreachable!()
                    }
                } else if fn_name == "fuzzy_match_all" {
                    if let FunctionArguments::List(list) = &func.args {
                        if list.args.len() != 2 {
                            unreachable!()
                        }
                        let value = trim_quotes(list.args[0].to_string().as_str());
                        let distance = trim_quotes(list.args[1].to_string().as_str())
                            .parse()
                            .unwrap_or(1);
                        Condition::FuzzyMatchAll(value, distance)
                    } else {
                        unreachable!()
                    }
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

    pub fn to_tantivy_query(
        &self,
        schema: &Schema,
        default_field: Option<Field>,
    ) -> anyhow::Result<Box<dyn Query>> {
        Ok(match self {
            Condition::Equal(field, value) => {
                let field = schema.get_field(field)?;
                let term = Term::from_field_text(field, value);
                Box::new(TermQuery::new(term, IndexRecordOption::Basic))
            }
            Condition::In(field, values) => {
                let field = schema.get_field(field)?;
                let terms: Vec<Box<dyn Query>> = values
                    .iter()
                    .map(|value| {
                        let term = Term::from_field_text(field, value);
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as _
                    })
                    .collect();
                Box::new(BooleanQuery::union(terms))
            }
            Condition::MatchAll(value) => {
                let default_field = default_field.ok_or_else(|| {
                    anyhow::anyhow!("There's no FullTextSearch field for match_all() function")
                })?;
                if value.starts_with("*") && value.ends_with("*") {
                    let value = format!(".*{}.*", value.trim_matches('*'));
                    Box::new(RegexQuery::from_pattern(&value, default_field)?)
                } else if value.to_lowercase().starts_with("re:") {
                    let value = value[3..].trim();
                    Box::new(RegexQuery::from_pattern(value, default_field)?)
                } else if value.ends_with("*") {
                    let value = value.strip_suffix("*").unwrap();
                    Box::new(PhrasePrefixQuery::new_with_offset(vec![(
                        0,
                        Term::from_field_text(default_field, value),
                    )]))
                } else {
                    if value.is_empty() {
                        return Err(anyhow::anyhow!(
                            "The value of match_all() function can't be empty"
                        ));
                    }
                    let mut terms: Vec<Box<dyn Query>> = o2_collect_tokens(value)
                        .into_iter()
                        .map(|value| {
                            let term = Term::from_field_text(default_field, &value);
                            Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as _
                        })
                        .collect();
                    if terms.len() > 1 {
                        Box::new(BooleanQuery::intersection(terms))
                    } else {
                        terms.remove(0)
                    }
                }
            }
            Condition::FuzzyMatchAll(value, distance) => {
                let default_field = default_field.ok_or_else(|| {
                    anyhow::anyhow!(
                        "There's no FullTextSearch field for fuzzy_match_all() function"
                    )
                })?;
                if value.is_empty() {
                    return Err(anyhow::anyhow!(
                        "The value of fuzzy_match_all() function can't be empty"
                    ));
                }
                let term = Term::from_field_text(default_field, value);
                Box::new(FuzzyTermQuery::new(term, *distance, false))
            }
            Condition::Or(left, right) => {
                let left_query = left.to_tantivy_query(schema, default_field)?;
                let right_query = right.to_tantivy_query(schema, default_field)?;
                Box::new(BooleanQuery::union(vec![left_query, right_query]))
            }
            Condition::And(left, right) => {
                let left_query = left.to_tantivy_query(schema, default_field)?;
                let right_query = right.to_tantivy_query(schema, default_field)?;
                Box::new(BooleanQuery::intersection(vec![left_query, right_query]))
            }
        })
    }

    pub fn get_tantivy_fields(&self) -> HashSet<String> {
        let mut fields = HashSet::new();
        match self {
            Condition::Equal(field, _) => {
                fields.insert(field.clone());
            }
            Condition::In(field, _) => {
                fields.insert(field.clone());
            }
            Condition::MatchAll(_) => {
                fields.insert(INDEX_FIELD_NAME_FOR_ALL.to_string());
            }
            Condition::FuzzyMatchAll(..) => {
                fields.insert(INDEX_FIELD_NAME_FOR_ALL.to_string());
            }
            Condition::Or(left, right) | Condition::And(left, right) => {
                fields.extend(left.get_tantivy_fields());
                fields.extend(right.get_tantivy_fields());
            }
        }
        fields
    }

    pub fn get_schema_fields(&self, fst_fields: &[String]) -> HashSet<String> {
        let mut fields = HashSet::new();
        match self {
            Condition::Equal(field, _) => {
                fields.insert(field.clone());
            }
            Condition::In(field, _) => {
                fields.insert(field.clone());
            }
            Condition::MatchAll(_) => {
                fields.extend(fst_fields.iter().cloned());
            }
            Condition::FuzzyMatchAll(..) => {
                fields.extend(fst_fields.iter().cloned());
            }
            Condition::Or(left, right) | Condition::And(left, right) => {
                fields.extend(left.get_schema_fields(fst_fields));
                fields.extend(right.get_schema_fields(fst_fields));
            }
        }
        fields
    }

    pub fn to_physical_expr(
        &self,
        schema: &arrow_schema::Schema,
        fst_fields: &[String],
    ) -> Result<Arc<dyn PhysicalExpr>, anyhow::Error> {
        match self {
            Condition::Equal(name, value) => {
                let index = schema.index_of(name).unwrap();
                let left = Arc::new(Column::new(name, index));
                let field = schema.field(index);
                let right = get_scalar_value(value, field.data_type())?;
                Ok(Arc::new(BinaryExpr::new(left, Operator::Eq, right)))
            }
            Condition::In(name, values) => {
                let index = schema.index_of(name).unwrap();
                let left = Arc::new(Column::new(name, index));
                let field = schema.field(index);
                let values: Vec<Arc<dyn PhysicalExpr>> = values
                    .iter()
                    .map(|value| get_scalar_value(value, field.data_type()).map(|v| v as _))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Arc::new(InListExpr::new(left, values, false, None)))
            }
            Condition::MatchAll(value) => {
                let value = value
                    .trim_start_matches("re:") // regex
                    .trim_start_matches('*') // contains
                    .trim_end_matches('*') // prefix or contains
                    .to_string();
                let term = Arc::new(Literal::new(ScalarValue::Utf8(Some(format!("%{value}%")))));
                let mut expr_list: Vec<Arc<dyn PhysicalExpr>> =
                    Vec::with_capacity(fst_fields.len());
                for field in fst_fields.iter() {
                    let new_expr = Arc::new(LikeExpr::new(
                        false,
                        true,
                        Arc::new(Column::new(field, schema.index_of(field).unwrap())),
                        term.clone(),
                    ));
                    expr_list.push(new_expr);
                }
                if expr_list.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Using match_all() function in a stream that don't have full text search field"
                    )); // already check this in sql.rs
                }
                Ok(disjunction(expr_list))
            }
            Condition::FuzzyMatchAll(value, distance) => {
                let fuzzy_expr = Arc::new(fuzzy_match_udf::FUZZY_MATCH_UDF.clone());
                let term = Arc::new(Literal::new(ScalarValue::Utf8(Some(value.clone()))));
                let distance = Arc::new(Literal::new(ScalarValue::Int64(Some(*distance as i64))));
                let mut expr_list: Vec<Arc<dyn PhysicalExpr>> =
                    Vec::with_capacity(fst_fields.len());
                for field in fst_fields.iter() {
                    let new_expr = Arc::new(ScalarFunctionExpr::new(
                        fuzzy_expr.name(),
                        fuzzy_expr.clone(),
                        vec![
                            Arc::new(Column::new(field, schema.index_of(field).unwrap())),
                            term.clone(),
                            distance.clone(),
                        ],
                        DataType::Boolean,
                    ));
                    expr_list.push(new_expr);
                }
                if expr_list.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Using fuzzy_match_all() function in a stream that don't have full text search field"
                    )); // already check this in sql.rs
                }
                Ok(disjunction(expr_list))
            }
            Condition::Or(left, right) => {
                let left = left.to_physical_expr(schema, fst_fields)?;
                let right = right.to_physical_expr(schema, fst_fields)?;
                Ok(Arc::new(BinaryExpr::new(left, Operator::Or, right)))
            }
            Condition::And(left, right) => {
                let left = left.to_physical_expr(schema, fst_fields)?;
                let right = right.to_physical_expr(schema, fst_fields)?;
                Ok(Arc::new(BinaryExpr::new(left, Operator::And, right)))
            }
        }
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
        Expr::InList {
            expr,
            list,
            negated,
        } => {
            if *negated {
                return false;
            }
            if !is_field(expr) || !index_fields.contains(&get_field_name(expr)) {
                return false;
            }
            for value in list {
                if !is_value(value) {
                    return false;
                }
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
            let fn_name = func.name.to_string().to_lowercase();
            if fn_name == "match_all" {
                if let FunctionArguments::List(list) = &func.args {
                    if list.args.len() != 1 {
                        return false;
                    }
                } else {
                    return false;
                }
            } else if fn_name == "fuzzy_match_all" {
                if let FunctionArguments::List(list) = &func.args {
                    if list.args.len() != 2 {
                        return false;
                    }
                } else {
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

fn disjunction(exprs: Vec<Arc<dyn PhysicalExpr>>) -> Arc<dyn PhysicalExpr> {
    if exprs.len() == 1 {
        exprs[0].clone()
    } else {
        // conjuction all expr in exprs
        let mut expr = exprs[0].clone();
        for e in exprs.into_iter().skip(1) {
            expr = Arc::new(BinaryExpr::new(expr, Operator::Or, e));
        }
        expr
    }
}

fn conjunction(exprs: Vec<Arc<dyn PhysicalExpr>>) -> Arc<dyn PhysicalExpr> {
    if exprs.len() == 1 {
        exprs[0].clone()
    } else {
        // conjuction all expr in exprs
        let mut expr = exprs[0].clone();
        for e in exprs.into_iter().skip(1) {
            expr = Arc::new(BinaryExpr::new(expr, Operator::And, e));
        }
        expr
    }
}

fn get_scalar_value(value: &str, data_type: &DataType) -> Result<Arc<Literal>, anyhow::Error> {
    Ok(match data_type {
        DataType::Boolean => Arc::new(Literal::new(ScalarValue::Boolean(Some(value.parse()?)))),
        DataType::Int64 => Arc::new(Literal::new(ScalarValue::Int64(Some(value.parse()?)))),
        DataType::UInt64 => Arc::new(Literal::new(ScalarValue::UInt64(Some(value.parse()?)))),
        DataType::Float64 => Arc::new(Literal::new(ScalarValue::Float64(Some(value.parse()?)))),
        DataType::Utf8 => Arc::new(Literal::new(ScalarValue::Utf8(Some(value.to_string())))),
        DataType::Binary => Arc::new(Literal::new(ScalarValue::Binary(Some(
            value.as_bytes().to_vec(),
        )))),
        _ => unimplemented!(),
    })
}
