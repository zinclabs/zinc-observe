// Copyright 2024 OpenObserve Inc.
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
    hash::{Hash, Hasher},
};

use arrow_schema::{Field, Schema};
use hashbrown::HashSet;

const HASH_MAP_SIZE: usize = std::mem::size_of::<HashMap<String, String>>();

/// SchemaExt helper...
pub trait SchemaExt {
    fn to_cloned_fields(&self) -> Vec<Field>;
    fn cloned_from(&self, schema: &Schema) -> Schema;
    fn retain(&self, fields: HashSet<String>) -> Schema;
    fn hash_key(&self) -> String;
    fn size(&self) -> usize;
    fn simple_fields(&self) -> Vec<(String, String)>;
}

impl SchemaExt for Schema {
    fn to_cloned_fields(&self) -> Vec<Field> {
        self.fields.iter().map(|x| (**x).clone()).collect()
    }

    // ensure schema is compatible
    fn cloned_from(&self, schema: &Schema) -> Schema {
        let schema_latest_map: HashMap<_, _> = schema
            .fields()
            .iter()
            .map(|field| (field.name(), field))
            .collect();
        let fields = self
            .fields()
            .iter()
            .map(|f| match schema_latest_map.get(f.name()) {
                Some(f) => (*f).clone(),
                None => f.clone(),
            })
            .collect::<Vec<_>>();
        Schema::new(fields)
    }

    // create a new schema with only the fields in the set and keep the order
    fn retain(&self, fields: HashSet<String>) -> Schema {
        let fields = self
            .fields()
            .iter()
            .filter(|f| fields.contains(f.name()))
            .cloned()
            .collect::<Vec<_>>();
        Schema::new(fields)
    }

    #[cfg(feature = "gxhash")]
    fn hash_key(&self) -> String {
        let mut hasher = gxhash::GxHasher::with_seed(0);
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    #[cfg(not(feature = "gxhash"))]
    fn hash_key(&self) -> String {
        let mut hasher = std::hash::DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn size(&self) -> usize {
        let mut size = self.fields.iter().fold(0, |acc, field| acc + field.size()) + HASH_MAP_SIZE;
        size += std::mem::size_of::<HashMap<String, String>>();
        for (key, val) in self.metadata.iter() {
            size += std::mem::size_of::<String>() * 2;
            size += key.len() + val.len();
        }
        size
    }

    fn simple_fields(&self) -> Vec<(String, String)> {
        self.fields
            .iter()
            .map(|x| (x.name().to_string(), x.data_type().to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use arrow_schema::DataType;

    use super::*;

    #[test]
    fn test_schema_hash() {
        let schema1 = Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("address", DataType::Utf8, false),
            Field::new("priority", DataType::UInt8, false),
        ]);
        let schema2 = Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("address", DataType::Utf8, false),
            Field::new("priority", DataType::UInt8, false),
            Field::new("flag", DataType::UInt8, false),
        ]);
        assert_ne!(schema1.hash_key(), schema2.hash_key());
    }
}
