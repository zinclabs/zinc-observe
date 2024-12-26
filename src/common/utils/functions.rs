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

use std::collections::HashMap;

use config::{
    meta::function::VRLCompilerConfig, GEO_IP_ASN_ENRICHMENT_TABLE, GEO_IP_CITY_ENRICHMENT_TABLE,
};
use vector_enrichment::{Table, TableRegistry};

use crate::common::{
    infra::config::{ENRICHMENT_TABLES, GEOIP_ASN_TABLE, GEOIP_CITY_TABLE},
    meta::organization::DEFAULT_ORG,
};

pub async fn get_all_transform_keys(org_id: &str) -> Vec<String> {
    let org_key = &format!("{}/", org_id);

    crate::common::infra::config::QUERY_FUNCTIONS
        .clone()
        .iter()
        .filter_map(|transform| {
            let key = transform.key();
            key.strip_prefix(org_key).map(|x| x.to_string())
        })
        .collect()
}

pub fn init_vrl_runtime() -> vrl::compiler::runtime::Runtime {
    vrl::compiler::runtime::Runtime::new(vrl::prelude::state::RuntimeState::default())
}

pub fn get_vrl_compiler_config(org_id: &str) -> VRLCompilerConfig {
    let en_tables = ENRICHMENT_TABLES.clone();
    let mut functions = vrl::stdlib::all();
    functions.append(&mut vector_enrichment::vrl_functions());
    let registry = TableRegistry::default();
    let mut tables: HashMap<String, Box<dyn Table + Send + Sync>> = HashMap::new();

    for table in en_tables.iter() {
        if table.org_id == org_id || table.org_id == DEFAULT_ORG {
            tables.insert(
                table.stream_name.to_owned(),
                Box::new(table.value().clone()),
            );
        }
    }
    if GEOIP_CITY_TABLE.read().is_some() {
        tables.insert(
            GEO_IP_CITY_ENRICHMENT_TABLE.to_owned(),
            Box::new(GEOIP_CITY_TABLE.read().as_ref().unwrap().clone()),
        );
    }
    if GEOIP_ASN_TABLE.read().is_some() {
        tables.insert(
            GEO_IP_ASN_ENRICHMENT_TABLE.to_owned(),
            Box::new(GEOIP_ASN_TABLE.read().as_ref().unwrap().clone()),
        );
    }
    #[cfg(feature = "enterprise")]
    if o2_enterprise::enterprise::common::infra::config::get_config()
        .common
        .enable_enterprise_mmdb
    {
        tables.insert(
            o2_enterprise::enterprise::common::infra::config::GEO_IP_ENTERPRISE_ENRICHMENT_TABLE
                .to_owned(),
            Box::new(
                crate::common::infra::config::GEOIP_ENT_TABLE
                    .read()
                    .as_ref()
                    .unwrap()
                    .clone(),
            ),
        );
    };

    registry.load(tables);
    let mut config = vrl::compiler::CompileConfig::default();
    config.set_custom(registry);
    VRLCompilerConfig { config, functions }
}
