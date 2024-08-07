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

pub mod exec;
pub mod file_type;
pub mod plan;
pub mod storage;
pub mod table_provider;
pub mod udf;

/// Extension setting for query limit
pub(crate) struct ExtLimit(usize);

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
