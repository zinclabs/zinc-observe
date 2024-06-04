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

use std::str::FromStr;

use crate::common::meta::functions::ZoFunction;

mod date_format_udf;
pub mod exec;
pub mod match_udf;
pub mod regexp_udf;
mod rewrite;
pub mod storage;
pub mod string_to_array_v2_udf;
mod time_range_udf;
mod transform_udf;

#[derive(PartialEq, Debug)]
pub enum MemoryPoolType {
    Greedy,
    Fair,
    None,
}

impl FromStr for MemoryPoolType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "greedy" => Ok(MemoryPoolType::Greedy),
            "fair" | "" => Ok(MemoryPoolType::Fair), // default is fair
            "none" | "off" => Ok(MemoryPoolType::None),
            _ => Err(format!("Invalid memory pool type '{}'", s)),
        }
    }
}

/// The name of the match UDF given to DataFusion.
pub const MATCH_UDF_NAME: &str = "str_match";
/// The name of the match_ignore_case UDF given to DataFusion.
pub const MATCH_UDF_IGNORE_CASE_NAME: &str = "str_match_ignore_case";
/// The name of the regex_match UDF given to DataFusion.
pub const REGEX_MATCH_UDF_NAME: &str = "re_match";
/// The name of the not_regex_match UDF given to DataFusion.
pub const REGEX_NOT_MATCH_UDF_NAME: &str = "re_not_match";

pub const DEFAULT_FUNCTIONS: [ZoFunction; 8] = [
    ZoFunction {
        name: "match_all",
        text: "match_all('v')",
    },
    ZoFunction {
        name: "match_all_ignore_case",
        text: "match_all_ignore_case('v')",
    },
    ZoFunction {
        name: "match_all_indexed",
        text: "match_all_indexed('v')",
    },
    ZoFunction {
        name: "match_all_indexed_ignore_case",
        text: "match_all_indexed_ignore_case('v')",
    },
    ZoFunction {
        name: MATCH_UDF_NAME,
        text: "match_all('v')",
    },
    ZoFunction {
        name: MATCH_UDF_IGNORE_CASE_NAME,
        text: "match_all_ignore_case('v')",
    },
    ZoFunction {
        name: REGEX_MATCH_UDF_NAME,
        text: "re_match(field, 'pattern')",
    },
    ZoFunction {
        name: REGEX_NOT_MATCH_UDF_NAME,
        text: "re_not_match(field, 'pattern')",
    },
];
