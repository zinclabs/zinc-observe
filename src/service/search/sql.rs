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

use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use chrono::Duration;
use config::{
    get_config,
    meta::{
        sql::{Sql as MetaSql, SqlOperator},
        stream::{FileKey, StreamPartition, StreamType},
    },
    QUICK_MODEL_FIELDS, SQL_FULL_TEXT_SEARCH_FIELDS,
};
use datafusion::arrow::datatypes::{DataType, Schema};
use hashbrown::HashSet;
use infra::{
    errors::{Error, ErrorCodes},
    schema::{get_stream_setting_fts_fields, STREAM_SCHEMAS_FIELDS},
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use proto::cluster_rpc;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{common::meta::stream::StreamParams, service::search::match_source};

const SQL_DELIMITERS: [u8; 12] = [
    b' ', b'*', b'(', b')', b'<', b'>', b',', b';', b'=', b'!', b'\r', b'\n',
];

pub static RE_ONLY_SELECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)select[ ]+\*").unwrap());
static RE_ONLY_GROUPBY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i) group[ ]+by[ ]+([a-zA-Z0-9'"._-]+)"#).unwrap());
static RE_SELECT_FIELD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)select (.*) from[ ]+query").unwrap());
static RE_SELECT_FROM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)SELECT (.*) FROM").unwrap());
static RE_WHERE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i) where (.*)").unwrap());

static RE_ONLY_WHERE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i) where ").unwrap());
static RE_ONLY_FROM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i) from[ ]+query").unwrap());

static RE_HISTOGRAM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)histogram\(([^\)]*)\)").unwrap());
static RE_MATCH_ALL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)match_all\('([^']*)'\)").unwrap());
static RE_MATCH_ALL_IGNORE_CASE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)match_all_ignore_case\('([^']*)'\)").unwrap());
static RE_MATCH_ALL_INDEXED: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)match_all_indexed\('([^']*)'\)").unwrap());
static RE_MATCH_ALL_INDEXED_IGNORE_CASE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)match_all_indexed_ignore_case\('([^']*)'\)").unwrap());

#[derive(Clone, Debug, Serialize)]
pub struct Sql {
    pub origin_sql: String,
    pub rewrite_sql: String,
    pub org_id: String,
    pub stream_type: StreamType,
    pub stream_name: String,
    pub meta: MetaSql,
    pub fulltext: Vec<(String, String)>,
    pub aggs: hashbrown::HashMap<String, (String, MetaSql)>,
    pub sql_mode: SqlMode,
    pub fast_mode: bool, /* there is no where, no group by, no aggregatioin, we can just get
                          * data from the latest file */
    pub schema: Schema,
    pub query_context: String,
    pub uses_zo_fn: bool,
    pub query_fn: Option<String>,
    pub fts_terms: Vec<String>,
    pub histogram_interval: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SqlMode {
    Context,
    Full,
}

impl From<&str> for SqlMode {
    fn from(mode: &str) -> Self {
        match mode.to_lowercase().as_str() {
            "full" => SqlMode::Full,
            "context" => SqlMode::Context,
            _ => SqlMode::Context,
        }
    }
}

impl Display for SqlMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlMode::Context => write!(f, "context"),
            SqlMode::Full => write!(f, "full"),
        }
    }
}

impl Sql {
    pub async fn new(req: &cluster_rpc::SearchRequest) -> Result<Sql, Error> {
        let req_query = req.query.as_ref().unwrap();
        let org_id = req.org_id.clone();
        let stream_type = StreamType::from(req.stream_type.as_str());

        let mut req_time_range = (req_query.start_time, req_query.end_time);
        if req_time_range.1 == 0 {
            req_time_range.1 = chrono::Utc::now().timestamp_micros();
        }

        // parse sql
        let mut origin_sql = req_query.sql.clone();
        let mut rewrite_sql = req_query.sql.clone();
        // log::info!("origin_sql: {:?}", origin_sql);
        origin_sql = origin_sql.replace('\n', " ");
        origin_sql = origin_sql.trim().to_string();
        if origin_sql.ends_with(';') {
            origin_sql.pop();
        }
        origin_sql = split_sql_token(&origin_sql).join("");
        let mut meta = match MetaSql::new(&origin_sql) {
            Ok(meta) => meta,
            Err(err) => {
                log::error!("parse sql error: {}, sql: {}", err, origin_sql);
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(origin_sql)));
            }
        };

        let cfg = get_config();
        // need check some things:
        // 1. no where
        // 2. no aggregation
        // 3. no group by
        let mut fast_mode = meta.selection.is_none()
            && meta.group_by.is_empty()
            && (meta.order_by.is_empty() || meta.order_by[0].0 == cfg.common.column_timestamp)
            && !meta.fields.iter().any(|f| f.contains('('))
            && !meta.field_alias.iter().any(|f| f.0.contains('('))
            && !origin_sql.to_lowercase().contains("distinct");

        // check sql_mode
        let sql_mode: SqlMode = req_query.sql_mode.as_str().into();
        let track_total_hits = req_query.track_total_hits && meta.limit == 0;

        // check SQL limitation
        // in context mode, disallow, [limit|offset|group by|having|join|union]
        // in full    mode, disallow, [join|union]
        if sql_mode.eq(&SqlMode::Context)
            && (meta.offset > 0 || meta.limit > 0 || !meta.group_by.is_empty())
        {
            return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                "sql_mode=context, Query SQL does not supported [limit|offset|group by|having|join|union]".to_string()
            )));
        }

        // check Agg SQL
        // 1. must from query
        // 2. disallow select *
        // 3. must select group by field
        let mut req_aggs = HashMap::new();
        for agg in req.aggs.iter() {
            req_aggs.insert(agg.name.to_string(), agg.sql.to_string());
        }
        if sql_mode.eq(&SqlMode::Full) && !req_aggs.is_empty() {
            return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                "sql_mode=full, Query not supported aggs".to_string(),
            )));
        }

        // check aggs
        for sql in req_aggs.values() {
            if !RE_ONLY_FROM.is_match(sql) {
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                    "Aggregation SQL only support 'from query' as context".to_string(),
                )));
            }
            if RE_ONLY_SELECT.is_match(sql) {
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                    "Aggregation SQL is not supported 'select *' please specify the fields"
                        .to_string(),
                )));
            }
            if RE_ONLY_GROUPBY.is_match(sql) {
                let caps = RE_ONLY_GROUPBY.captures(sql).unwrap();
                let group_by = caps
                    .get(1)
                    .unwrap()
                    .as_str()
                    .trim_matches(|v| v == '\'' || v == '"');
                let select_caps = match RE_SELECT_FIELD.captures(sql) {
                    Some(caps) => caps,
                    None => {
                        return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                            sql.to_owned(),
                        )));
                    }
                };
                if !select_caps.get(1).unwrap().as_str().contains(group_by) {
                    return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(format!(
                        "Aggregation SQL used [group by] you should select the field [{group_by}]"
                    ))));
                }
            }
        }

        // Hack for table name
        // DataFusion disallow use `k8s-logs-2022.09.11` as table name
        let stream_name = meta.source.clone();
        let re = Regex::new(&format!(r#"(?i) from[ '"]+{stream_name}[ '"]?"#)).unwrap();
        let caps = match re.captures(origin_sql.as_str()) {
            Some(caps) => caps,
            None => {
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(origin_sql)));
            }
        };
        origin_sql = origin_sql.replace(caps.get(0).unwrap().as_str(), " FROM tbl ");

        // Hack select for _timestamp
        if !sql_mode.eq(&SqlMode::Full) && meta.order_by.is_empty() && !origin_sql.contains('*') {
            let caps = RE_SELECT_FROM.captures(origin_sql.as_str()).unwrap();
            let cap_str = caps.get(1).unwrap().as_str();
            if !cap_str.contains(&cfg.common.column_timestamp) {
                origin_sql = origin_sql.replace(
                    cap_str,
                    &format!("{}, {}", &cfg.common.column_timestamp, cap_str),
                );
            }
        }

        // check time_range values
        if req_time_range.0 > 0
            && req_time_range.0
                < Duration::try_seconds(1)
                    .unwrap()
                    .num_microseconds()
                    .unwrap()
        {
            return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                "Query SQL time_range start_time should be microseconds".to_string(),
            )));
        }
        if req_time_range.1 > 0
            && req_time_range.1
                < Duration::try_seconds(1)
                    .unwrap()
                    .num_microseconds()
                    .unwrap()
        {
            return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                "Query SQL time_range start_time should be microseconds".to_string(),
            )));
        }
        if req_time_range.0 > 0 && req_time_range.1 > 0 && req_time_range.1 < req_time_range.0 {
            return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                "Query SQL time_range start_time should be less than end_time".to_string(),
            )));
        }

        // Hack time_range for sql
        let meta_time_range_is_empty = meta.time_range.is_none() || meta.time_range == Some((0, 0));
        if meta_time_range_is_empty && (req_time_range.0 > 0 || req_time_range.1 > 0) {
            meta.time_range = Some(req_time_range); // update meta
        };
        if let Some(time_range) = meta.time_range {
            let time_range_sql = if time_range.0 > 0 && time_range.1 > 0 {
                format!(
                    "({} >= {} AND {} < {})",
                    cfg.common.column_timestamp,
                    time_range.0,
                    cfg.common.column_timestamp,
                    time_range.1
                )
            } else if time_range.0 > 0 {
                format!("{} >= {}", cfg.common.column_timestamp, time_range.0)
            } else if time_range.1 > 0 {
                format!("{} < {}", cfg.common.column_timestamp, time_range.1)
            } else {
                "".to_string()
            };
            if !time_range_sql.is_empty() && meta_time_range_is_empty {
                match RE_WHERE.captures(origin_sql.as_str()) {
                    Some(caps) => {
                        let mut where_str = caps.get(1).unwrap().as_str().to_string();
                        if !meta.group_by.is_empty() {
                            where_str = where_str
                                [0..where_str.to_lowercase().rfind(" group ").unwrap()]
                                .to_string();
                        } else if meta.having {
                            where_str = where_str
                                [0..where_str.to_lowercase().rfind(" having ").unwrap()]
                                .to_string();
                        } else if !meta.order_by.is_empty() {
                            where_str = where_str
                                [0..where_str.to_lowercase().rfind(" order ").unwrap()]
                                .to_string();
                        } else if meta.limit > 0 {
                            where_str = where_str
                                [0..where_str.to_lowercase().rfind(" limit ").unwrap()]
                                .to_string();
                        } else if meta.offset > 0 {
                            where_str = where_str
                                [0..where_str.to_lowercase().rfind(" offset ").unwrap()]
                                .to_string();
                        }
                        let pos_start = origin_sql.find(where_str.as_str()).unwrap();
                        let pos_end = pos_start + where_str.len();
                        origin_sql = format!(
                            "{}{} AND ({}){}",
                            &origin_sql[0..pos_start],
                            time_range_sql,
                            where_str,
                            &origin_sql[pos_end..]
                        );
                    }
                    None => {
                        origin_sql = origin_sql
                            .replace(" FROM tbl", &format!(" FROM tbl WHERE {time_range_sql}"));
                    }
                };
            }
        }

        // Hack offset limit and sort by for sql
        if meta.limit == 0 {
            meta.offset = req_query.from as i64;
            // If `size` is negative, use the backend's default limit setting
            meta.limit = if req_query.size >= 0 {
                req_query.size as i64
            } else {
                cfg.limit.query_default_limit
            };
            origin_sql = if meta.order_by.is_empty()
                && (!sql_mode.eq(&SqlMode::Full)
                    || (meta.group_by.is_empty()
                        && !origin_sql
                            .to_lowercase()
                            .split(" from ")
                            .next()
                            .unwrap_or_default()
                            .contains('(')))
            {
                let sort_by = if req_query.sort_by.is_empty() {
                    meta.order_by = vec![(cfg.common.column_timestamp.to_string(), true)];
                    format!("{} DESC", cfg.common.column_timestamp)
                } else {
                    if req_query.sort_by.to_uppercase().ends_with(" DESC") {
                        meta.order_by = vec![(
                            req_query.sort_by[0..req_query.sort_by.len() - 5].to_string(),
                            true,
                        )];
                    } else if req_query.sort_by.to_uppercase().ends_with(" ASC") {
                        meta.order_by = vec![(
                            req_query.sort_by[0..req_query.sort_by.len() - 4].to_string(),
                            false,
                        )];
                    }
                    req_query.sort_by.clone()
                };
                format!(
                    "{} ORDER BY {} LIMIT {}",
                    origin_sql,
                    sort_by,
                    meta.offset + meta.limit
                )
            } else {
                format!("{} LIMIT {}", origin_sql, meta.offset + meta.limit)
            };
        }

        // fetch schema
        let schema = match infra::schema::get(&org_id, &meta.source, stream_type).await {
            Ok(schema) => schema,
            Err(_) => Schema::empty(),
        };
        let schema_fields = schema.fields().to_vec();

        // fetch fts fields
        let mut fts_terms = HashSet::new();
        let fts_fields = get_stream_setting_fts_fields(&schema).unwrap();
        let match_all_fields = if !fts_fields.is_empty() {
            fts_fields.iter().map(|v| v.to_lowercase()).collect()
        } else {
            SQL_FULL_TEXT_SEARCH_FIELDS
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
        };

        // Hack for quick_mode
        // replace `select *` to `select f1,f2,f3`
        if req_query.quick_mode
            && schema_fields.len() > cfg.limit.quick_mode_num_fields
            && RE_ONLY_SELECT.is_match(&origin_sql)
        {
            let stream_key = format!("{}/{}/{}", org_id, stream_type, meta.source);
            let cached_fields: Option<Vec<String>> = if cfg.limit.quick_mode_file_list_enabled {
                STREAM_SCHEMAS_FIELDS
                    .read()
                    .await
                    .get(&stream_key)
                    .map(|v| v.1.clone())
            } else {
                None
            };
            let fields = generate_quick_mode_fields(&schema, cached_fields, &match_all_fields);
            let select_fields = "SELECT ".to_string() + &fields.join(",");
            origin_sql = RE_ONLY_SELECT
                .replace(origin_sql.as_str(), &select_fields)
                .to_string();
            // rewrite distribution sql
            rewrite_sql = RE_ONLY_SELECT
                .replace(rewrite_sql.as_str(), &select_fields)
                .to_string();
            // reset meta fields
            meta.fields.extend(fields);
        }

        // get sql where tokens
        let where_tokens = split_sql_token(&origin_sql);
        let where_pos = where_tokens
            .iter()
            .position(|x| x.to_lowercase() == "where");
        let mut where_tokens = if let Some(v) = where_pos {
            where_tokens[v + 1..].to_vec()
        } else {
            Vec::new()
        };

        // HACK full text search
        let mut fulltext = Vec::new();
        let mut indexed_text = Vec::new();
        for token in &where_tokens {
            let tokens = split_sql_token_unwrap_brace(token);
            for token in &tokens {
                if !token.to_lowercase().starts_with("match_all") {
                    continue;
                }
                for cap in RE_MATCH_ALL.captures_iter(token) {
                    fulltext.push((cap[0].to_string(), cap[1].to_string()));
                }
                for cap in RE_MATCH_ALL_IGNORE_CASE.captures_iter(token) {
                    fulltext.push((cap[0].to_string(), cap[1].to_lowercase()));
                }
                for cap in RE_MATCH_ALL_INDEXED.captures_iter(token) {
                    indexed_text.push((cap[0].to_string(), cap[1].to_string()));
                }
                for cap in RE_MATCH_ALL_INDEXED_IGNORE_CASE.captures_iter(token) {
                    indexed_text.push((cap[0].to_string(), cap[1].to_lowercase()));
                }
            }
        }

        // Iterator for indexed texts only
        for item in indexed_text.iter() {
            let mut indexed_search = Vec::new();
            for field in &schema_fields {
                if !match_all_fields.contains(&field.name().to_lowercase()) {
                    continue;
                }
                if !field.data_type().eq(&DataType::Utf8) || field.name().starts_with('@') {
                    continue;
                }
                let mut func = "LIKE";
                if item.0.to_lowercase().contains("_ignore_case") {
                    func = "ILIKE";
                }
                indexed_search.push(format!("\"{}\" {} '%{}%'", field.name(), func, item.1));

                // add full text field to meta fields
                meta.fields.push(field.name().to_string());

                fts_terms.insert(item.1.clone());
            }
            if indexed_search.is_empty() {
                return Err(Error::ErrorCode(ErrorCodes::FullTextSearchFieldNotFound));
            }
            let indexed_search = format!("({})", indexed_search.join(" OR "));
            origin_sql = origin_sql.replace(item.0.as_str(), &indexed_search);
        }

        for item in fulltext.iter() {
            let mut fulltext_search = Vec::new();
            for field in &schema_fields {
                if !match_all_fields.contains(&field.name().to_lowercase()) {
                    continue;
                }
                if !field.data_type().eq(&DataType::Utf8) || field.name().starts_with('@') {
                    continue;
                }
                let mut func = "LIKE";
                if item.0.to_lowercase().contains("_ignore_case") {
                    func = "ILIKE";
                }
                fulltext_search.push(format!("\"{}\" {} '%{}%'", field.name(), func, item.1));
                // add full text field to meta fields
                meta.fields.push(field.name().to_string());
            }
            if fulltext_search.is_empty() {
                return Err(Error::ErrorCode(ErrorCodes::FullTextSearchFieldNotFound));
            }
            let fulltext_search = format!("({})", fulltext_search.join(" OR "));
            origin_sql = origin_sql.replace(item.0.as_str(), &fulltext_search);
        }

        // Hack: str_match
        for key in [
            "match",
            "match_ignore_case",
            // "str_match"             // use UDF will get better result
            // "str_match_ignore_case" // use UDF will get better result
        ] {
            let re_str_match = Regex::new(&format!(r"(?i)\b{key}\b\(([^\)]*)\)")).unwrap();
            let re_fn = if key == "match" || key == "str_match" {
                "LIKE"
            } else {
                "ILIKE"
            };
            for token in &where_tokens {
                if !token.to_lowercase().starts_with("match")
                    && !token.to_lowercase().starts_with("str_match")
                {
                    continue;
                }
                for cap in re_str_match.captures_iter(token.as_str()) {
                    let attrs = cap
                        .get(1)
                        .unwrap()
                        .as_str()
                        .splitn(2, ',')
                        .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
                        .collect::<Vec<&str>>();
                    let field = attrs.first().unwrap();
                    let value = attrs.last().unwrap();
                    origin_sql = origin_sql.replace(
                        cap.get(0).unwrap().as_str(),
                        &format!("\"{field}\" {re_fn} '%{value}%'"),
                    );
                    fts_terms.insert(value.to_string());
                }
            }
        }

        // Hack for histogram
        let mut histogram_interval = None;
        let from_pos = origin_sql.to_lowercase().find(" from ").unwrap();
        let select_str = origin_sql[0..from_pos].to_string();
        for cap in RE_HISTOGRAM.captures_iter(select_str.as_str()) {
            let attrs = cap
                .get(1)
                .unwrap()
                .as_str()
                .split(',')
                .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
                .collect::<Vec<&str>>();
            let field = attrs.first().unwrap();
            let interval = match attrs.get(1) {
                Some(v) => match v.parse::<u16>() {
                    Ok(v) => generate_histogram_interval(meta.time_range, v),
                    Err(_) => v.to_string(),
                },
                None => generate_histogram_interval(meta.time_range, 0),
            };
            origin_sql = origin_sql.replace(
                cap.get(0).unwrap().as_str(),
                &format!(
                    "date_bin(interval '{interval}', to_timestamp_micros(\"{field}\"), to_timestamp('2001-01-01T00:00:00'))",
                )
            );
            if histogram_interval.is_none() {
                histogram_interval = Some(convert_histogram_interval_to_seconds(&interval)?);
            }
        }

        // pickup where
        let mut where_str = match RE_WHERE.captures(&origin_sql) {
            Some(caps) => caps[1].to_string(),
            None => "".to_string(),
        };
        if !where_str.is_empty() {
            let mut where_str_lower = where_str.to_lowercase();
            for key in ["group", "order", "offset", "limit"].iter() {
                if !where_tokens.iter().any(|x| x.to_lowercase().eq(key)) {
                    continue;
                }
                let where_pos = where_tokens
                    .iter()
                    .position(|x| x.to_lowercase().eq(key))
                    .unwrap();
                where_tokens = where_tokens[..where_pos].to_vec();
                if let Some(pos) = where_str_lower.rfind(key) {
                    where_str = where_str[..pos].to_string();
                    where_str_lower = where_str.to_lowercase();
                }
            }
        }

        // Hack for aggregation
        if track_total_hits {
            req_aggs.insert(
                "_count".to_string(),
                String::from("SELECT COUNT(*) as num from query"),
            );
        }
        let mut aggs = hashbrown::HashMap::new();
        for (key, sql) in &req_aggs {
            let mut sql = sql.to_string();
            if let Some(caps) = RE_ONLY_FROM.captures(&sql) {
                sql = sql.replace(&caps[0].to_string(), " FROM tbl ");
            }
            if !where_str.is_empty() {
                match RE_ONLY_WHERE.captures(&sql) {
                    Some(caps) => {
                        sql = sql
                            .replace(&caps[0].to_string(), &format!(" WHERE ({where_str}) AND "));
                    }
                    None => {
                        sql = sql.replace(
                            &" FROM tbl ".to_string(),
                            &format!(" FROM tbl WHERE ({where_str}) "),
                        );
                    }
                }
            }
            let sql_meta = MetaSql::new(sql.clone().as_str());
            if sql_meta.is_err() {
                log::error!("parse sql error: {}, sql: {}", sql_meta.err().unwrap(), sql);
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(sql)));
            }
            let sql_meta = sql_meta.unwrap();
            for cap in RE_HISTOGRAM.captures_iter(sql.clone().as_str()) {
                let attrs = cap
                    .get(1)
                    .unwrap()
                    .as_str()
                    .split(',')
                    .map(|v| v.trim().trim_matches(|v| v == '\'' || v == '"'))
                    .collect::<Vec<&str>>();
                let field = attrs.first().unwrap();
                let interval = attrs.get(1).unwrap();
                sql = sql.replace(
                    cap.get(0).unwrap().as_str(),
                    &format!(
                        "date_bin(interval '{interval}', to_timestamp_micros(\"{field}\"), to_timestamp('2001-01-01T00:00:00'))"
                    )
                );
            }

            if !(sql_meta.group_by.is_empty()
                || (sql_meta.field_alias.len() == 2
                    && sql_meta.field_alias[0].1 == "zo_sql_key"
                    && sql_meta.field_alias[1].1 == "zo_sql_num"))
            {
                fast_mode = false;
            }
            aggs.insert(key.clone(), (sql, sql_meta));
        }

        let sql_meta = MetaSql::new(origin_sql.clone().as_str());

        match &sql_meta {
            Ok(sql_meta) => {
                let mut used_fns = vec![];
                for fn_name in
                    crate::common::utils::functions::get_all_transform_keys(&org_id).await
                {
                    let str_re = format!(r"(?i){}[ ]*\(.*\)", fn_name);

                    if let Ok(re1) = Regex::new(&str_re) {
                        let cap = re1.captures(&origin_sql);
                        if cap.is_some() {
                            for _ in 0..cap.unwrap().len() {
                                used_fns.push(fn_name.clone());
                            }
                        }
                    }
                }
                if sql_meta.field_alias.len() < used_fns.len() {
                    return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                        "Please use alias for function used in query.".to_string(),
                    )));
                }
            }
            Err(e) => {
                log::error!("parse sql error: {}, sql: {}", e, origin_sql);
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(origin_sql)));
            }
        }

        let query_fn = if req_query.query_fn.is_empty() {
            None
        } else {
            Some(req_query.query_fn.clone())
        };

        Ok(Sql {
            origin_sql,
            rewrite_sql,
            org_id,
            stream_type,
            stream_name,
            meta,
            fulltext,
            aggs,
            sql_mode,
            fast_mode,
            schema,
            query_context: req_query.query_context.clone(),
            uses_zo_fn: req_query.uses_zo_fn,
            query_fn,
            fts_terms: fts_terms.into_iter().collect(),
            histogram_interval,
        })
    }

    /// match a source is a valid file or not
    pub async fn match_source(
        &self,
        source: &FileKey,
        match_min_ts_only: bool,
        is_wal: bool,
        stream_type: StreamType,
        partition_keys: &[StreamPartition],
    ) -> bool {
        let mut filters = generate_filter_from_quick_text(&self.meta.quick_text);
        // rewrite partition filters
        let partition_keys: HashMap<&str, &StreamPartition> = partition_keys
            .iter()
            .map(|v| (v.field.as_str(), v))
            .collect();
        for entry in filters.iter_mut() {
            if let Some(partition_key) = partition_keys.get(entry.0) {
                for val in entry.1.iter_mut() {
                    *val = partition_key.get_partition_value(val);
                }
            }
        }
        match_source(
            StreamParams::new(&self.org_id, &self.stream_name, stream_type),
            self.meta.time_range,
            filters.as_slice(),
            source,
            is_wal,
            match_min_ts_only,
        )
        .await
    }
}

pub fn generate_filter_from_quick_text(
    data: &[(String, String, SqlOperator)],
) -> Vec<(&str, Vec<String>)> {
    let quick_text_len = data.len();
    let mut filters = HashMap::with_capacity(quick_text_len);
    for i in 0..quick_text_len {
        let (k, v, op) = &data[i];
        if op == &SqlOperator::And
            || (op == &SqlOperator::Or && (i + 1 == quick_text_len || k == &data[i + 1].0))
        {
            let entry = filters.entry(k.as_str()).or_insert_with(Vec::new);
            entry.push(v.to_string());
        } else {
            filters.clear();
            break;
        }
    }
    filters.into_iter().collect::<Vec<(_, _)>>()
}

pub(crate) fn generate_quick_mode_fields(
    schema: &Schema,
    cached_fields: Option<Vec<String>>,
    fts_fields: &[String],
) -> Vec<String> {
    let cfg = get_config();
    let strategy = cfg.limit.quick_mode_strategy.to_lowercase();
    let schema_fields = match cached_fields {
        Some(v) => v,
        None => schema
            .fields()
            .iter()
            .map(|f| f.name().to_string())
            .collect(),
    };
    let mut fields = match strategy.as_str() {
        "last" => {
            let skip = std::cmp::max(0, schema_fields.len() - cfg.limit.quick_mode_num_fields);
            schema_fields.into_iter().skip(skip).collect()
        }
        "both" => {
            let need_num = std::cmp::min(schema_fields.len(), cfg.limit.quick_mode_num_fields);
            let mut inner_fields = schema_fields
                .iter()
                .take(need_num / 2)
                .map(|f| f.to_string())
                .collect::<Vec<_>>();
            if schema_fields.len() > inner_fields.len() {
                let skip = std::cmp::max(0, schema_fields.len() + inner_fields.len() - need_num);
                inner_fields.extend(schema_fields.iter().skip(skip).map(|f| f.to_string()));
            }
            inner_fields
        }
        _ => {
            // default is first mode
            schema_fields
                .into_iter()
                .take(cfg.limit.quick_mode_num_fields)
                .collect()
        }
    };
    // check _timestamp
    if !fields.contains(&cfg.common.column_timestamp) {
        fields.push(cfg.common.column_timestamp.to_string());
    }
    // check fts fields
    for field in fts_fields {
        if !fields.contains(field) && schema.field_with_name(field).is_ok() {
            fields.push(field.to_string());
        }
    }
    // check quick mode fields
    for field in QUICK_MODEL_FIELDS.iter() {
        if !fields.contains(field) && schema.field_with_name(field).is_ok() {
            fields.push(field.to_string());
        }
    }
    fields
}

fn generate_histogram_interval(time_range: Option<(i64, i64)>, num: u16) -> String {
    if time_range.is_none() || time_range.unwrap().eq(&(0, 0)) {
        return "1 hour".to_string();
    }
    let time_range = time_range.unwrap();
    if num > 0 {
        return format!(
            "{} second",
            std::cmp::max(
                (time_range.1 - time_range.0)
                    / Duration::try_seconds(1)
                        .unwrap()
                        .num_microseconds()
                        .unwrap()
                    / num as i64,
                1
            )
        );
    }

    let intervals = [
        (
            Duration::try_hours(24 * 30)
                .unwrap()
                .num_microseconds()
                .unwrap(),
            "1 day",
        ),
        (
            Duration::try_hours(24 * 7)
                .unwrap()
                .num_microseconds()
                .unwrap(),
            "1 hour",
        ),
        (
            Duration::try_hours(24).unwrap().num_microseconds().unwrap(),
            "30 minute",
        ),
        (
            Duration::try_hours(6).unwrap().num_microseconds().unwrap(),
            "5 minute",
        ),
        (
            Duration::try_hours(2).unwrap().num_microseconds().unwrap(),
            "1 minute",
        ),
        (
            Duration::try_hours(1).unwrap().num_microseconds().unwrap(),
            "30 second",
        ),
        (
            Duration::try_minutes(30)
                .unwrap()
                .num_microseconds()
                .unwrap(),
            "15 second",
        ),
        (
            Duration::try_minutes(15)
                .unwrap()
                .num_microseconds()
                .unwrap(),
            "10 second",
        ),
    ];
    for interval in intervals.iter() {
        if (time_range.1 - time_range.0) >= interval.0 {
            return interval.1.to_string();
        }
    }
    "10 second".to_string()
}

fn convert_histogram_interval_to_seconds(interval: &str) -> Result<i64, Error> {
    let Some((num, unit)) = interval.splitn(2, ' ').collect_tuple() else {
        return Err(Error::Message("Invalid interval format".to_string()));
    };
    let seconds = match unit.to_lowercase().as_str() {
        "second" | "seconds" => num.parse::<i64>(),
        "minute" | "minutes" => num.parse::<i64>().map(|n| n * 60),
        "hour" | "hours" => num.parse::<i64>().map(|n| n * 3600),
        "day" | "days" => num.parse::<i64>().map(|n| n * 86400),
        _ => {
            return Err(Error::Message(
                "Unsupported histogram interval unit".to_string(),
            ));
        }
    };
    seconds.map_err(|_| Error::Message("Invalid number format".to_string()))
}

fn split_sql_token_unwrap_brace(token: &str) -> Vec<String> {
    if token.is_empty() {
        return vec![];
    }
    if token.starts_with('(') && token.ends_with(')') {
        return split_sql_token_unwrap_brace(&token[1..token.len() - 1]);
    }
    let tokens = split_sql_token(token);
    let mut fin_tokens = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.starts_with('(') && token.ends_with(')') {
            fin_tokens.extend(split_sql_token_unwrap_brace(&token[1..token.len() - 1]));
        } else {
            fin_tokens.push(token);
        }
    }
    fin_tokens
}

fn split_sql_token(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let text_chars = text.chars().collect::<Vec<char>>();
    let text_chars_len = text_chars.len();
    let mut start_pos = 0;
    let mut in_word = false;
    let mut bracket = 0;
    let mut in_quote = false;
    let mut quote = ' ';
    for i in 0..text_chars_len {
        let c = text_chars.get(i).unwrap();
        if !in_quote && *c == '(' {
            bracket += 1;
            continue;
        }
        if !in_quote && *c == ')' {
            bracket -= 1;
            continue;
        }
        if *c == '\'' || *c == '"' {
            if in_quote {
                if quote == *c {
                    in_quote = false;
                }
            } else {
                in_quote = true;
                quote = *c;
            }
        }
        if SQL_DELIMITERS.contains(&(*c as u8)) {
            if bracket > 0 || in_quote {
                continue;
            }
            if in_word {
                let token = text_chars[start_pos..i].iter().collect::<String>();
                tokens.push(token);
            }
            tokens.push(String::from_utf8(vec![*c as u8]).unwrap());
            in_word = false;
            start_pos = i + 1;
            continue;
        }
        if in_word {
            continue;
        }
        in_word = true;
    }
    if start_pos != text_chars_len {
        let token = text_chars[start_pos..text_chars_len]
            .iter()
            .collect::<String>();
        tokens.push(token);
    }

    // filter tokens by break line
    for token in tokens.iter_mut() {
        if token.eq(&"\r\n") || token.eq(&"\n") {
            *token = " ".to_string();
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sql_works() {
        let org_id = "test_org";
        let col = "_timestamp";
        let table = "default";
        let query = config::meta::search::Query {
            sql: format!("select {} from {} ", col, table),
            from: 0,
            size: 100,
            sql_mode: "full".to_owned(),
            quick_mode: false,
            query_type: "".to_owned(),
            start_time: 1667978895416,
            end_time: 1667978900217,
            sort_by: None,
            track_total_hits: false,
            query_context: None,
            uses_zo_fn: false,
            query_fn: None,
            skip_wal: false,
        };

        let req: config::meta::search::Request = config::meta::search::Request {
            query,
            aggs: HashMap::new(),
            encoding: config::meta::search::RequestEncoding::Empty,
            regions: vec![],
            clusters: vec![],
            timeout: 0,
        };

        let mut rpc_req: cluster_rpc::SearchRequest = req.to_owned().into();
        rpc_req.org_id = org_id.to_string();

        let resp = Sql::new(&rpc_req).await.unwrap();
        assert_eq!(resp.stream_name, table);
        assert_eq!(resp.org_id, org_id);
        assert!(resp.meta.fields.contains(&col.to_string()));
    }

    #[tokio::test]
    async fn test_sql_contexts() {
        let sqls = [
            ("select * from table1", true, (0, 0)),
            ("select * from table1 where a=1", true, (0, 0)),
            ("select * from table1 where a='b'", true, (0, 0)),
            (
                "select * from table1 where a='b' limit 10 offset 10",
                false,
                (0, 0),
            ),
            (
                "select * from table1 where a='b' group by abc",
                false,
                (0, 0),
            ),
            (
                "select * from table1 where a='b' group by abc having count(*) > 19",
                false,
                (0, 0),
            ),
            ("select * from table1, table2 where a='b'", false, (0, 0)),
            (
                "select * from table1 left join table2 on table1.a=table2.b where a='b'",
                false,
                (0, 0),
            ),
            (
                "select * from table1 union select * from table2 where a='b'",
                false,
                (0, 0),
            ),
            (
                "select * from table1 where log='[2023-03-19T05:23:14Z INFO  openobserve::service::search::datafusion::exec] Query sql: select * FROM tbl WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000)   ORDER BY _timestamp DESC LIMIT 150'",
                true,
                (0, 0),
            ),
            (
                "select * from table1 where log='[2023-03-19T05:23:14Z INFO  openobserve::service::search::datafusion::exec] Query sql: select * FROM tbl WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000)   ORDER BY _timestamp DESC LIMIT 150' order by _timestamp desc limit 10 offset 10",
                false,
                (0, 0),
            ),
            (
                "select * from table1 where log='[2023-03-19T05:23:14Z INFO  openobserve::service::search::datafusion::exec] Query sql: select * FROM tbl WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000)   ORDER BY _timestamp DESC LIMIT 150' AND time_range(_timestamp, 1679202494333000, 1679203394333000) order by _timestamp desc",
                true,
                (1679202494333000, 1679203394333000),
            ),
            (
                "select * from table1 WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000 AND str_match(log, 's')) order by _timestamp desc",
                true,
                (1679202494333000, 1679203394333000),
            ),
            (
                "select * from table1 WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000 AND str_match(log, 's') AND str_match_IGNORE_CASE(log, 's')) order by _timestamp desc",
                true,
                (1679202494333000, 1679203394333000),
            ),
            (
                "select * from table1 where match_all('abc') order by _timestamp desc limit 10 offset 10",
                false,
                (0, 0),
            ),
            (
                "select * from table1 where match_all('abc') and str_match(log,'abc') order by _timestamp desc",
                false,
                (0, 0),
            ),
            (
                "select abc, count(*) as cnt from table1 where match_all('abc') and str_match(log,'abc') group by abc having cnt > 1 order by _timestamp desc limit 10",
                false,
                (0, 0),
            ),
        ];

        let org_id = "test_org";
        for (sql, ok, time_range) in sqls {
            let query = config::meta::search::Query {
                sql: sql.to_string(),
                from: 0,
                size: 100,
                sql_mode: "context".to_owned(),
                quick_mode: false,
                query_type: "".to_owned(),
                start_time: 1667978895416,
                end_time: 1667978900217,
                sort_by: None,
                track_total_hits: true,
                query_context: None,
                uses_zo_fn: false,
                query_fn: None,
                skip_wal: false,
            };
            let req = config::meta::search::Request {
                query: query.clone(),
                aggs: HashMap::new(),
                encoding: config::meta::search::RequestEncoding::Empty,
                regions: vec![],
                clusters: vec![],
                timeout: 0,
            };
            let mut rpc_req: cluster_rpc::SearchRequest = req.to_owned().into();
            rpc_req.org_id = org_id.to_string();

            let resp = Sql::new(&rpc_req).await;
            assert_eq!(resp.is_ok(), ok);
            if ok {
                let resp = resp.unwrap();
                assert_eq!(resp.stream_name, "table1");
                assert_eq!(resp.org_id, org_id);
                if time_range.0 > 0 {
                    assert_eq!(resp.meta.time_range, Some((time_range.0, time_range.1)));
                } else {
                    assert_eq!(
                        resp.meta.time_range,
                        Some((query.start_time, query.end_time))
                    );
                }
                assert_eq!(resp.meta.limit, query.size);
            }
        }
    }

    #[tokio::test]
    async fn test_sql_full() {
        let sqls = [
            ("select * from table1", true, 0, (0, 0)),
            ("select * from table1 where a=1", true, 0, (0, 0)),
            ("select * from table1 where a='b'", true, 0, (0, 0)),
            (
                "select * from table1 where a='b' limit 10 offset 10",
                true,
                10,
                (0, 0),
            ),
            (
                "select * from table1 where a='b' group by abc",
                true,
                0,
                (0, 0),
            ),
            (
                "select * from table1 where a='b' group by abc having count(*) > 19",
                true,
                0,
                (0, 0),
            ),
            ("select * from table1, table2 where a='b'", false, 0, (0, 0)),
            (
                "select * from table1 left join table2 on table1.a=table2.b where a='b'",
                false,
                0,
                (0, 0),
            ),
            (
                "select * from table1 union select * from table2 where a='b'",
                false,
                0,
                (0, 0),
            ),
            (
                "select * from table1 where log='[2023-03-19T05:23:14Z INFO  openobserve::service::search::datafusion::exec] Query sql: select * FROM tbl WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000)   ORDER BY _timestamp DESC LIMIT 150'",
                true,
                0,
                (0, 0),
            ),
            (
                "select * from table1 where log='[2023-03-19T05:23:14Z INFO  openobserve::service::search::datafusion::exec] Query sql: select * FROM tbl WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000)   ORDER BY _timestamp DESC LIMIT 150' order by _timestamp desc limit 10 offset 10",
                true,
                10,
                (0, 0),
            ),
            (
                "select * from table1 where log='[2023-03-19T05:23:14Z INFO  openobserve::service::search::datafusion::exec] Query sql: select * FROM tbl WHERE (_timestamp >= 1679202494333000 AND _timestamp < 1679203394333000)   ORDER BY _timestamp DESC LIMIT 150' AND time_range(_timestamp, 1679202494333000, 1679203394333000) order by _timestamp desc",
                true,
                0,
                (1679202494333000, 1679203394333000),
            ),
            (
                "select histogram(_timestamp, '5 second') AS zo_sql_key, count(*) AS zo_sql_num from table1 GROUP BY zo_sql_key ORDER BY zo_sql_key",
                true,
                0,
                (0, 0),
            ),
            (
                "select DISTINCT field1, field2, field3 FROM table1",
                true,
                0,
                (0, 0),
            ),
            (
                "SELECT trace_id, MIN(start_time) AS start_time FROM table1 WHERE service_name ='APISIX-B' GROUP BY trace_id ORDER BY start_time DESC LIMIT 10",
                true,
                10,
                (0, 0),
            ),
        ];

        let org_id = "test_org";
        for (sql, ok, limit, time_range) in sqls {
            let query = config::meta::search::Query {
                sql: sql.to_string(),
                from: 0,
                size: 100,
                sql_mode: "full".to_owned(),
                quick_mode: false,
                query_type: "".to_owned(),
                start_time: 1667978895416,
                end_time: 1667978900217,
                sort_by: None,
                track_total_hits: true,
                query_context: None,
                uses_zo_fn: false,
                query_fn: None,
                skip_wal: false,
            };
            let req = config::meta::search::Request {
                query: query.clone(),
                aggs: HashMap::new(),
                encoding: config::meta::search::RequestEncoding::Empty,
                regions: vec![],
                clusters: vec![],
                timeout: 0,
            };
            let mut rpc_req: cluster_rpc::SearchRequest = req.to_owned().into();
            rpc_req.org_id = org_id.to_string();

            let resp = Sql::new(&rpc_req).await;
            assert_eq!(resp.is_ok(), ok);
            if ok {
                let resp = resp.unwrap();
                assert_eq!(resp.stream_name, "table1");
                assert_eq!(resp.org_id, org_id);
                if time_range.0 > 0 {
                    assert_eq!(resp.meta.time_range, Some((time_range.0, time_range.1)));
                } else {
                    assert_eq!(
                        resp.meta.time_range,
                        Some((query.start_time, query.end_time))
                    );
                }
                if limit > 0 {
                    assert_eq!(resp.meta.limit, limit);
                } else {
                    assert_eq!(resp.meta.limit, query.size);
                }
            }
        }
    }
}
