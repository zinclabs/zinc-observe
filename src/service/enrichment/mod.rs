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

use async_trait::async_trait;
use config::utils::time::parse_str_to_time;
use vector_enrichment::{Case, IndexHandle, Table};
use vrl::value::{ObjectMap, Value};

#[derive(Clone)]
pub struct StreamTableConfig {}

#[derive(Debug, Clone)]
pub struct StreamTable {
    pub org_id: String,
    pub stream_name: String,
    pub data: Vec<vrl::value::Value>,
}
impl StreamTable {}

#[async_trait]
impl Table for StreamTable {
    fn find_table_row(
        &self,
        case: vector_enrichment::Case,
        conditions: &[vector_enrichment::Condition],
        select: Option<&[String]>,
        _index: Option<vector_enrichment::IndexHandle>,
    ) -> Result<ObjectMap, String> {
        let resp = get_data(self, conditions, select, case);
        let record = if resp.is_empty() {
            ObjectMap::new()
        } else {
            resp.first().unwrap().clone()
        };

        Ok(record)
    }

    fn find_table_rows(
        &self,
        case: vector_enrichment::Case,
        conditions: &[vector_enrichment::Condition],
        select: Option<&[String]>,
        _index: Option<vector_enrichment::IndexHandle>,
    ) -> Result<Vec<ObjectMap>, String> {
        let resp = get_data(self, conditions, select, case);
        Ok(resp)
    }

    fn add_index(
        &mut self,
        _case: vector_enrichment::Case,
        _fields: &[&str],
    ) -> Result<vector_enrichment::IndexHandle, String> {
        Ok(IndexHandle(1))
    }

    fn index_fields(&self) -> Vec<(vector_enrichment::Case, Vec<String>)> {
        Vec::new()
    }

    fn needs_reload(&self) -> bool {
        false
    }
}

fn get_data(
    table: &StreamTable,
    condition: &[vector_enrichment::Condition],
    select: Option<&[String]>,
    case: vector_enrichment::Case,
) -> Vec<ObjectMap> {
    let mut resp = vec![];
    let filtered: Vec<&vrl::value::Value> = table
        .data
        .iter()
        .filter(|v| {
            if let vrl::value::Value::Object(map) = v {
                for cond in condition {
                    match cond {
                        vector_enrichment::Condition::Equals { field, value } => match case {
                            Case::Insensitive => {
                                if let Some(Value::Bytes(bytes1)) = map.get(field.to_owned()) {
                                    if let Value::Bytes(bytes2) = value {
                                        match (
                                            std::str::from_utf8(bytes1),
                                            std::str::from_utf8(bytes2),
                                        ) {
                                            (Ok(s1), Ok(s2)) => {
                                                return s1.eq_ignore_ascii_case(s2);
                                            }
                                            (Err(_), Err(_)) => return bytes1 == bytes2,
                                            _ => return false,
                                        }
                                    }
                                }
                            }
                            Case::Sensitive => {
                                if let Some(v) = map.get(field.to_owned()) {
                                    if v.clone() == value.clone() {
                                        return true;
                                    }
                                }
                            }
                        },
                        vector_enrichment::Condition::BetweenDates { field, from, to } => {
                            if let Some(v) = map.get(field.to_owned()) {
                                if let Some(v) = v.as_str() {
                                    if let Ok(v) = parse_str_to_time(&v) {
                                        if v >= *from && v <= *to {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            false
        })
        .collect();

    match select {
        Some(val) => {
            for value in filtered {
                if let Some(map) = value.as_object() {
                    let mut btree_map = ObjectMap::new();
                    for field in val {
                        if let Some(v) = map.get(field.as_str()) {
                            btree_map.insert(field.to_owned().into(), v.clone());
                        }
                    }
                    resp.push(btree_map);
                };
            }
        }
        None => {
            for value in filtered {
                if let Value::Object(map) = value {
                    let btree_map: ObjectMap = map
                        .iter()
                        .map(|(k, v)| (k.to_owned(), v.clone()))
                        .collect::<ObjectMap>();
                    resp.push(btree_map);
                };
            }
        }
    };

    resp
}
