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

use crate::service::db::kv;

pub async fn get(org_id: &str, key: &str) -> Result<bytes::Bytes, anyhow::Error> {
    let val = kv::get(org_id, key).await?;
    Ok(val)
}

pub async fn set(org_id: &str, key: &str, val: bytes::Bytes) -> Result<(), anyhow::Error> {
    kv::set(org_id, key, val).await?;
    Ok(())
}

pub async fn delete(org_id: &str, key: &str) -> Result<(), anyhow::Error> {
    kv::delete(org_id, key).await?;
    Ok(())
}

pub async fn list(org_id: &str, prefix: &str) -> Result<Vec<String>, anyhow::Error> {
    let items = kv::list(org_id, prefix).await?;
    Ok(items)
}
