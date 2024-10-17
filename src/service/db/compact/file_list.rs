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

use crate::service::db;

pub async fn get_offset() -> Result<i64, anyhow::Error> {
    let key = "/compact/file_list/offset";
    let value = match db::get(key).await {
        Ok(ret) => String::from_utf8_lossy(&ret).to_string(),
        Err(_) => String::from("0"),
    };
    let offset: i64 = value.parse().unwrap();
    Ok(offset)
}

pub async fn set_offset(offset: i64) -> Result<(), anyhow::Error> {
    let key = "/compact/file_list/offset";
    db::put(key, offset.to_string().into(), db::NO_NEED_WATCH, None).await?;
    Ok(())
}

pub async fn set_delete(key: &str) -> Result<(), anyhow::Error> {
    let key = format!("/compact/file_list/delete/{key}");
    db::put(&key, "OK".into(), db::NO_NEED_WATCH, None).await?;
    Ok(())
}

pub async fn del_delete(key: &str) -> Result<(), anyhow::Error> {
    let key = format!("/compact/file_list/delete/{key}");
    db::delete_if_exists(&key, false, db::NO_NEED_WATCH)
        .await
        .map_err(Into::into)
}

pub async fn list_delete() -> Result<Vec<String>, anyhow::Error> {
    let mut items = Vec::new();
    let key = "/compact/file_list/delete/";
    for (item_key, _item_value) in db::list(key).await? {
        let item_key = item_key.strip_prefix(key).unwrap();
        items.push(item_key.to_string());
    }
    Ok(items)
}

pub async fn get_process(offset: i64) -> String {
    let key = format!("/compact/file_list/process/{offset}");
    match db::get(&key).await {
        Ok(ret) => String::from_utf8_lossy(&ret).to_string(),
        Err(_) => String::from(""),
    }
}

pub async fn set_process(offset: i64, node: &str) -> Result<(), anyhow::Error> {
    let key = format!("/compact/file_list/process/{offset}");
    db::put(&key, node.to_string().into(), db::NO_NEED_WATCH, None).await?;
    Ok(())
}

pub async fn del_process(offset: i64) -> Result<(), anyhow::Error> {
    let key = format!("/compact/file_list/process/{offset}");
    db::delete_if_exists(&key, false, db::NO_NEED_WATCH)
        .await
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_list_offset() {
        const OFFSET: i64 = 100;

        set_offset(OFFSET).await.unwrap();
        assert_eq!(get_offset().await.unwrap(), OFFSET);

        let delete_day = "2023-03-03";
        set_delete(delete_day).await.unwrap();
        assert_eq!(list_delete().await.unwrap(), vec![delete_day.to_string()]);
    }

    #[tokio::test]
    async fn test_file_list_process_offset() {
        const OFFSET: i64 = 100;
        const NODE: &str = "node1";

        set_process(OFFSET, NODE).await.unwrap();
        assert_eq!(get_process(OFFSET).await, NODE);
    }
}
