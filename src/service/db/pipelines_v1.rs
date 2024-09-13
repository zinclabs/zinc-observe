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

use config::{meta::stream::StreamType, utils::json};

use crate::{common::meta::pipelines::PipeLine, service::db};

pub async fn set(org_id: &str, pipeline: &PipeLine) -> Result<(), anyhow::Error> {
    let key = format!(
        "/pipeline/{org_id}/{}/{}/{}",
        pipeline.stream_type, pipeline.stream_name, pipeline.name
    );
    match db::put(
        &key,
        json::to_vec(pipeline).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error saving pipeline: {}", e);
            return Err(anyhow::anyhow!("Error saving pipeline: {}", e));
        }
    }

    Ok(())
}

pub async fn get(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<PipeLine, anyhow::Error> {
    let val = db::get(&format!(
        "/pipeline/{org_id}/{stream_type}/{stream_name}/{name}"
    ))
    .await?;
    Ok(json::from_slice(&val).unwrap())
}

pub async fn delete(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<(), anyhow::Error> {
    let key = format!("/pipeline/{org_id}/{stream_type}/{stream_name}/{name}");
    match db::delete(&key, false, db::NEED_WATCH, None).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("Error deleting pipeline: {}", e);
            return Err(anyhow::anyhow!("Error deleting pipeline: {}", e));
        }
    }
    Ok(())
}

pub async fn list(org_id: &str) -> Result<Vec<PipeLine>, anyhow::Error> {
    Ok(db::list(&format!("/pipeline/{org_id}/"))
        .await?
        .values()
        .map(|val| json::from_slice(val).unwrap())
        .collect())
}
