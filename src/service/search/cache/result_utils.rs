use std::io::{self, Read, Write};

use config::utils::{base64, json};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
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
        Expr::Function(Function { name, args: _, .. }) => {
            AGGREGATE_UDF_LIST.contains(&name.to_string().to_lowercase().as_str())
        }

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

pub fn encode_sql_to_foldername(sql_query: &str) -> io::Result<String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(sql_query.as_bytes())?;
    let compressed_bytes = encoder.finish()?;

    Ok(base64::encode(json::from_slice(&compressed_bytes).unwrap()))
}

pub fn decode_foldername_to_sql(folder_name: &str) -> io::Result<String> {
    let compressed_bytes = base64::decode(folder_name)?;
    let mut decoder = ZlibDecoder::new(compressed_bytes.as_bytes());
    let mut decompressed_bytes = Vec::new();
    decoder.read_to_end(&mut decompressed_bytes)?;

    Ok(String::from_utf8(decompressed_bytes).unwrap())
}
