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

use config::{meta::stream::FileMeta, RwHashMap, RwHashSet};
use dashmap::{DashMap, DashSet};
use infra::{cache, file_list};
use once_cell::sync::Lazy;

pub mod broadcast;
pub mod local;
pub mod remote;

pub static DEPULICATE_FILES: Lazy<RwHashSet<String>> =
    Lazy::new(|| DashSet::with_capacity_and_hasher(1024, Default::default()));

pub static DELETED_FILES: Lazy<RwHashMap<String, FileMeta>> =
    Lazy::new(|| DashMap::with_capacity_and_hasher(64, Default::default()));

pub static BLOCKED_ORGS: Lazy<Vec<String>> = Lazy::new(|| {
    config::get_config()
        .compact
        .blocked_orgs
        .split(',')
        .map(|x| x.to_string())
        .collect()
});

pub async fn progress(
    key: &str,
    data: Option<&FileMeta>,
    delete: bool,
) -> Result<(), anyhow::Error> {
    if delete {
        if let Err(e) = file_list::remove(key).await {
            log::error!(
                "service:db:file_list: delete {}, set_file_to_cache error: {}",
                key,
                e
            );
        }
    } else {
        if let Err(e) = file_list::add(key, data.unwrap()).await {
            log::error!(
                "service:db:file_list: add {}, set_file_to_cache error: {}",
                key,
                e
            );
        }
        // update stream stats realtime
        if config::get_config().common.local_mode {
            if let Err(e) = cache::stats::incr_stream_stats(key, data.unwrap()) {
                log::error!(
                    "service:db:file_list: add {}, incr_stream_stats error: {}",
                    key,
                    e
                );
            }
        }
    }

    Ok(())
}
