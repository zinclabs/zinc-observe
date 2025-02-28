// Copyright 2025 OpenObserve Inc.
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

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MetaStore {
    Sqlite,
    Etcd,
    Nats,
    MySQL,
    PostgreSQL,
}

impl From<&str> for MetaStore {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sqlite" => MetaStore::Sqlite,
            "etcd" => MetaStore::Etcd,
            "nats" => MetaStore::Nats,
            "mysql" => MetaStore::MySQL,
            "postgres" | "postgresql" => MetaStore::PostgreSQL,
            _ => MetaStore::Sqlite,
        }
    }
}

impl From<String> for MetaStore {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "sqlite" => MetaStore::Sqlite,
            "etcd" => MetaStore::Etcd,
            "nats" => MetaStore::Nats,
            "mysql" => MetaStore::MySQL,
            "postgres" | "postgresql" => MetaStore::PostgreSQL,
            _ => MetaStore::Sqlite,
        }
    }
}

impl std::fmt::Display for MetaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MetaStore::Sqlite => write!(f, "sqlite"),
            MetaStore::Etcd => write!(f, "etcd"),
            MetaStore::Nats => write!(f, "nats"),
            MetaStore::MySQL => write!(f, "mysql"),
            MetaStore::PostgreSQL => write!(f, "postgresql"),
        }
    }
}
