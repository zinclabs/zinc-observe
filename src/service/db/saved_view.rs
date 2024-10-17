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

use config::utils::json;
use infra::errors::Error;

use crate::{
    common::meta::saved_view::{
        CreateViewRequest, UpdateViewRequest, View, ViewWithoutData, ViewsWithoutData,
    },
    service::db,
};

pub const SAVED_VIEWS_KEY_PREFIX: &str = "/organization/savedviews";

pub async fn set_view(org_id: &str, view: &CreateViewRequest) -> Result<View, Error> {
    let view_id = config::ider::uuid();
    let view = View {
        org_id: org_id.into(),
        view_id: view_id.clone(),
        data: view.data.clone(),
        view_name: view.view_name.clone(),
    };
    let key = format!("{}/{}/{}", SAVED_VIEWS_KEY_PREFIX, org_id, view_id);
    db::put(
        &key,
        json::to_vec(&view).unwrap().into(),
        db::NO_NEED_WATCH,
        None,
    )
    .await?;
    Ok(view)
}

/// Update the given view
pub async fn update_view(
    org_id: &str,
    view_id: &str,
    view: &UpdateViewRequest,
) -> Result<View, Error> {
    let key = format!("{}/{}/{}", SAVED_VIEWS_KEY_PREFIX, org_id, view_id);
    let updated_view = match get_view(org_id, view_id).await {
        Ok(original_view) => View {
            data: view.data.clone(),
            view_name: view.view_name.clone(),
            ..original_view
        },
        Err(e) => return Err(e),
    };
    db::put(
        &key,
        json::to_vec(&updated_view).unwrap().into(),
        db::NO_NEED_WATCH,
        None,
    )
    .await?;
    Ok(updated_view)
}

/// Get the saved view id associated with an org_id
pub async fn get_view(org_id: &str, view_id: &str) -> Result<View, Error> {
    let key = format!("{}/{}/{}", SAVED_VIEWS_KEY_PREFIX, org_id, view_id);
    let ret = db::get(&key).await?;
    let view = json::from_slice(&ret).unwrap();
    Ok(view)
}

/// Return all the saved views but query limited data only, associated with a
/// provided org_id This will not contain the payload.
pub async fn get_views_list_only(org_id: &str) -> Result<ViewsWithoutData, Error> {
    let key = format!("{}/{}", SAVED_VIEWS_KEY_PREFIX, org_id);
    let ret = db::list_values(&key).await?;
    let mut views: Vec<ViewWithoutData> = ret
        .iter()
        .map(|view| json::from_slice(view).unwrap())
        .collect();
    views.sort_by_key(|v| v.view_name.clone());

    Ok(ViewsWithoutData { views })
}

/// Delete a saved view id associated with an org-id
// pub async fn delete_view(org_id: &str, view_id: &str) -> Result<View, Error>
// {
pub async fn delete_view(org_id: &str, view_id: &str) -> Result<(), Error> {
    let key = format!("{}/{}/{}", SAVED_VIEWS_KEY_PREFIX, org_id, view_id);
    db::delete(&key, false, db::NO_NEED_WATCH, None).await?;
    Ok(())
}
