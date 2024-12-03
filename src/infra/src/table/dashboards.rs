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

use config::meta::dashboards::{
    v1::Dashboard as DashboardV1, v2::Dashboard as DashboardV2, v3::Dashboard as DashboardV3,
    v4::Dashboard as DashboardV4, v5::Dashboard as DashboardV5, Dashboard, ListDashboardsParams,
};
use sea_orm::{
    prelude::Expr, sea_query::Func, ActiveModelTrait, ActiveValue::NotSet, ColumnTrait,
    DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, QueryOrder, Set, TryIntoModel,
};
use serde_json::Value as JsonValue;

use super::{
    entity::{dashboards, folders},
    folders::FolderType,
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors::{self, GetDashboardError, PutDashboardError},
};

impl TryFrom<dashboards::Model> for Dashboard {
    type Error = errors::Error;

    fn try_from(mut value: dashboards::Model) -> Result<Self, Self::Error> {
        if let Some(obj) = value.data.as_object_mut() {
            // The domain model JSON deserialization logic for v1-v5 expects
            // some or all these fields to be present in the JSON even though we
            // store them in DB columns. Therefore we add these values back into
            // the JSON object so that deserializing the JSON can succeed.
            obj.insert("dashboardId".to_owned(), value.dashboard_id.into());
            obj.insert("version".to_owned(), value.version.into());
            obj.insert("owner".to_owned(), value.owner.into());
            obj.insert("role".to_owned(), value.role.unwrap_or_default().into());
            obj.insert("title".to_owned(), value.title.into());
            obj.insert(
                "description".to_owned(),
                value.description.unwrap_or_default().into(),
            );
        }

        match value.version {
            1 => {
                let inner: DashboardV1 = serde_json::from_value(value.data)?;
                let dash = inner.into();
                Ok(dash)
            }
            2 => {
                let inner: DashboardV2 = serde_json::from_value(value.data)?;
                let dash = inner.into();
                Ok(dash)
            }
            3 => {
                let inner: DashboardV3 = serde_json::from_value(value.data)?;
                let dash = inner.into();
                Ok(dash)
            }
            4 => {
                let inner: DashboardV4 = serde_json::from_value(value.data)?;
                let dash = inner.into();
                Ok(dash)
            }
            5 => {
                let inner: DashboardV5 = serde_json::from_value(value.data)?;
                let dash = inner.into();
                Ok(dash)
            }
            _ => Err(GetDashboardError::UnsupportedVersion(value.version).into()),
        }
    }
}

/// Gets a dashboard.
pub async fn get(
    org_id: &str,
    folder_id: &str,
    dashboard_id: &str,
) -> Result<Option<Dashboard>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let model = get_model(client, org_id, folder_id, dashboard_id)
        .await?
        .and_then(|(_folder, maybe_dash)| maybe_dash);

    if let Some(model) = model {
        let dash = model.try_into()?;
        Ok(Some(dash))
    } else {
        Ok(None)
    }
}

/// Lists all dashboards belonging to the given org and folder.
pub async fn list(params: ListDashboardsParams) -> Result<Vec<Dashboard>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let dashboards = list_models(client, params)
        .await?
        .into_iter()
        .map(Dashboard::try_from)
        .collect::<Result<_, _>>()?;
    Ok(dashboards)
}

/// Lists all dashboards belonging to the given org and folder.
pub async fn list_all() -> Result<Vec<(String, Dashboard)>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let dashboards = list_all_models(client)
        .await?
        .into_iter()
        .map(|(org, db)| Ok((org, Dashboard::try_from(db)?)))
        .collect::<Result<_, errors::Error>>()?;
    Ok(dashboards)
}

/// Creates a new dashboard or updates an existing dashboard in the database.
/// Returns the new or updated dashboard.
pub async fn put(
    org_id: &str,
    folder_id: &str,
    dashboard: Dashboard,
) -> Result<Dashboard, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    // Get the fields that will be inserted into or updated in the database.
    let dashboard_id = dashboard
        .dashboard_id()
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .ok_or(errors::PutDashboardError::MissingDashboardId)?
        .to_owned();
    let owner = dashboard
        .owner()
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .ok_or(errors::PutDashboardError::MissingOwner)?
        .to_owned();
    let title = dashboard
        .title()
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .ok_or(errors::PutDashboardError::MissingTitle)?
        .to_owned();
    let role = dashboard
        .role()
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .map(|r| r.to_owned());
    let description = dashboard
        .description()
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
        .map(|d| d.to_owned());
    let version = dashboard.version;
    let created_at_depricated = dashboard.created_at_deprecated();

    let data = inner_data_as_json(dashboard)?;

    let (folder_m, mut dash_am) = match get_model(client, org_id, folder_id, &dashboard_id).await? {
        None => {
            // Destination folder does not exist so the dashboard can neither be
            // created nor updated.
            Err(errors::PutDashboardError::FolderDoesNotExist)
        }
        Some((folder_m, Some(dash_m))) => {
            // Destination folder exists and dashboard already exists, so
            // convert the dashboard model to an active model that will be
            // updated.
            Ok((folder_m, dash_m.into()))
        }
        Some((folder_m, None)) => {
            // Destination folder exists but dashboard does not exist, so create
            // a new dashboard active model that will be inserted.
            let created_at_unix: i64 = if let Some(created_at_tz) = created_at_depricated {
                created_at_tz.timestamp()
            } else {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| PutDashboardError::ConvertingCreatedTimestamp)?
                    .as_secs()
                    .try_into()
                    .map_err(|_| PutDashboardError::ConvertingCreatedTimestamp)?
            };

            let dash_am = dashboards::ActiveModel {
                id: NotSet, // Set by DB.
                dashboard_id: Set(dashboard_id.to_owned()),
                folder_id: NotSet,   // Can be updated, so it is set below.
                owner: NotSet,       // Can be updated, so it is set below.
                role: NotSet,        // Can be updated, so it is set below.
                title: NotSet,       // Can be updated, so it is set below.
                description: NotSet, // Can be updated, so it is set below.
                data: NotSet,        // Can be updated, so it is set below.
                version: NotSet,     // Can be updated, so it is set below.
                created_at: Set(created_at_unix),
            };
            Ok((folder_m, dash_am))
        }
    }?;

    // All of the following fields will be set on creation or updated.
    dash_am.folder_id = Set(folder_m.id);
    dash_am.owner = Set(owner);
    dash_am.role = Set(role);
    dash_am.title = Set(title);
    dash_am.description = Set(description);
    dash_am.data = Set(data);
    dash_am.version = Set(version);

    let model: dashboards::Model = dash_am.save(client).await?.try_into_model()?;
    let dash = model.try_into()?;
    Ok(dash)
}

/// Deletes a dashboard with the given `folder_id` and `dashboard_id` surrogate
/// keys.
pub async fn delete(
    org_id: &str,
    folder_id: &str,
    dashboard_id: &str,
) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let model = get_model(client, org_id, folder_id, dashboard_id)
        .await?
        .and_then(|(_folder, maybe_dash)| maybe_dash);

    if let Some(model) = model {
        let _ = model.delete(client).await?;
    }

    Ok(())
}

/// Deletes all dashboards.
pub async fn delete_all() -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    dashboards::Entity::delete_many().exec(client).await?;
    Ok(())
}

/// Tries to get a dashboard ORM entity and its parent folder ORM entity.
async fn get_model(
    db: &DatabaseConnection,
    org_id: &str,
    folder_id: &str,
    dashboard_id: &str,
) -> Result<Option<(folders::Model, Option<dashboards::Model>)>, sea_orm::DbErr> {
    let select_folders = folders::Entity::find()
        .filter(folders::Column::Org.eq(org_id))
        .filter(folders::Column::Type.eq::<i16>(FolderType::Dashboards.into()))
        .filter(folders::Column::FolderId.eq(folder_id));

    let Some(folder) = select_folders.one(db).await? else {
        return Ok(None);
    };

    let maybe_dashboard = folder
        .find_related(dashboards::Entity)
        .filter(dashboards::Column::DashboardId.eq(dashboard_id))
        .one(db)
        .await?;

    Ok(Some((folder, maybe_dashboard)))
}

/// Lists all dashboard ORM models that belong to the given org and folder.
async fn list_models(
    db: &DatabaseConnection,
    params: ListDashboardsParams,
) -> Result<Vec<dashboards::Model>, sea_orm::DbErr> {
    let query = folders::Entity::find()
        .filter(folders::Column::Org.eq(params.org_id))
        .filter(folders::Column::Type.eq::<i16>(FolderType::Dashboards.into()));

    // Apply the optional folder_id filter.
    let query = if let Some(folder_id) = &params.folder_id {
        query.filter(folders::Column::FolderId.eq(folder_id))
    } else {
        query
    };

    // Apply ordering. Confusingly, it is necessary to apply the ordering BEFORE
    // adding a join to the query builder. If we don't do this then Sea ORM will
    // always sort on the join key (folder.id) before any other sorting
    // conditions.
    let query = query.order_by(dashboards::Column::Title, sea_orm::Order::Asc);

    // Left join on dashboards table.
    let query = query.find_with_related(dashboards::Entity);

    // Apply the optional title substring filter.
    let title_pat = params
        .title_pat
        .and_then(|p| if p.is_empty() { None } else { Some(p) });
    let query = if let Some(title_pat) = title_pat {
        let pattern = format!("%{}%", title_pat.to_lowercase());
        query.filter(Expr::expr(Func::lower(Expr::col(dashboards::Column::Title))).like(pattern))
    } else {
        query
    };

    let dashboards = query
        .all(db)
        .await?
        .into_iter()
        .flat_map(|(_, ds)| ds)
        .collect();
    Ok(dashboards)
}

/// Lists all dashboard ORM models that belong to the given org and folder.
async fn list_all_models(
    db: &DatabaseConnection,
) -> Result<Vec<(String, dashboards::Model)>, sea_orm::DbErr> {
    let query = folders::Entity::find()
        .filter(folders::Column::Type.eq::<i16>(FolderType::Dashboards.into()));

    // Apply ordering. Confusingly, it is necessary to apply the ordering BEFORE
    // adding a join to the query builder. If we don't do this then Sea ORM will
    // always sort on the join key (folder.id) before any other sorting
    // conditions.
    let query = query.order_by(dashboards::Column::Title, sea_orm::Order::Asc);

    // Left join on dashboards table.
    let query = query.find_with_related(dashboards::Entity);

    let dashboards = query
        .all(db)
        .await?
        .into_iter()
        .flat_map(|(folder, ds)| ds.into_iter().map(move |d| (folder.org.clone(), d)))
        .collect();
    Ok(dashboards)
}

/// Converts the [Dashboard] into the JSON represention of the inner data that
/// will be stored in the data column of the database.
///
/// The returned JSON object should exclude any field that will be stored in
/// database columns. The schema of the JSON object will vary depending on the
/// dashboard version.
fn inner_data_as_json(dashboard: Dashboard) -> Result<JsonValue, errors::Error> {
    let mut data: JsonValue = match dashboard {
        Dashboard {
            version: 1,
            v1: Some(inner),
            ..
        } => serde_json::to_value(inner).map_err(errors::Error::SerdeJsonError),
        Dashboard {
            version: 2,
            v2: Some(inner),
            ..
        } => serde_json::to_value(inner).map_err(errors::Error::SerdeJsonError),
        Dashboard {
            version: 3,
            v3: Some(inner),
            ..
        } => serde_json::to_value(inner).map_err(errors::Error::SerdeJsonError),
        Dashboard {
            version: 4,
            v4: Some(inner),
            ..
        } => serde_json::to_value(inner).map_err(errors::Error::SerdeJsonError),
        Dashboard {
            version: 5,
            v5: Some(inner),
            ..
        } => serde_json::to_value(inner).map_err(errors::Error::SerdeJsonError),
        Dashboard { version: v, .. } => Err(errors::PutDashboardError::MissingInnerData(v).into()),
    }?;

    // Remove fields that will be saved in DB columns from the JSON blob so that
    // we are not storing the same value in two different places and risking
    // desynchronization.
    if let Some(obj) = data.as_object_mut() {
        obj.remove("dashboard_id");
        obj.remove("version");
        obj.remove("owner");
        obj.remove("role");
        obj.remove("title");
        obj.remove("description");
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use sea_orm::{entity::prelude::*, DatabaseBackend, MockDatabase, Transaction};

    use super::*;

    #[tokio::test]
    async fn list_models_psql() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([Vec::<dashboards::Model>::new()])
            .into_connection();
        let params = ListDashboardsParams {
            org_id: "orgId".to_owned(),
            folder_id: Some("folderId".to_owned()),
            title_pat: Some("tItLePat".to_owned()),
        };
        list_models(&db, params).await?;
        assert_eq!(
            db.into_transaction_log(),
            vec![Transaction::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"SELECT "folders"."id" AS "A_id", "folders"."org" AS "A_org", "folders"."folder_id" AS "A_folder_id", "folders"."name" AS "A_name", "folders"."description" AS "A_description", "folders"."type" AS "A_type", "dashboards"."id" AS "B_id", "dashboards"."dashboard_id" AS "B_dashboard_id", "dashboards"."folder_id" AS "B_folder_id", "dashboards"."owner" AS "B_owner", "dashboards"."role" AS "B_role", "dashboards"."title" AS "B_title", "dashboards"."description" AS "B_description", "dashboards"."data" AS "B_data", "dashboards"."version" AS "B_version", "dashboards"."created_at" AS "B_created_at" FROM "folders" LEFT JOIN "dashboards" ON "folders"."id" = "dashboards"."folder_id" WHERE "folders"."org" = $1 AND "folders"."type" = $2 AND "folders"."folder_id" = $3 AND LOWER("title") LIKE $4 ORDER BY "dashboards"."title" ASC, "folders"."id" ASC"#,
                [
                    "orgId".into(),
                    0i16.into(),
                    "folderId".into(),
                    "%titlepat%".into()
                ]
            )]
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_models_mysql() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::MySql)
            .append_query_results([Vec::<dashboards::Model>::new()])
            .into_connection();
        let params = ListDashboardsParams {
            org_id: "orgId".to_owned(),
            folder_id: Some("folderId".to_owned()),
            title_pat: Some("tItLePat".to_owned()),
        };
        list_models(&db, params).await?;
        assert_eq!(
            db.into_transaction_log(),
            vec![Transaction::from_sql_and_values(
                DatabaseBackend::MySql,
                r#"SELECT `folders`.`id` AS `A_id`, `folders`.`org` AS `A_org`, `folders`.`folder_id` AS `A_folder_id`, `folders`.`name` AS `A_name`, `folders`.`description` AS `A_description`, `folders`.`type` AS `A_type`, `dashboards`.`id` AS `B_id`, `dashboards`.`dashboard_id` AS `B_dashboard_id`, `dashboards`.`folder_id` AS `B_folder_id`, `dashboards`.`owner` AS `B_owner`, `dashboards`.`role` AS `B_role`, `dashboards`.`title` AS `B_title`, `dashboards`.`description` AS `B_description`, `dashboards`.`data` AS `B_data`, `dashboards`.`version` AS `B_version`, `dashboards`.`created_at` AS `B_created_at` FROM `folders` LEFT JOIN `dashboards` ON `folders`.`id` = `dashboards`.`folder_id` WHERE `folders`.`org` = ? AND `folders`.`type` = ? AND `folders`.`folder_id` = ? AND LOWER(`title`) LIKE ? ORDER BY `dashboards`.`title` ASC, `folders`.`id` ASC"#,
                [
                    "orgId".into(),
                    0i16.into(),
                    "folderId".into(),
                    "%titlepat%".into()
                ]
            )]
        );
        Ok(())
    }

    #[tokio::test]
    async fn list_models_sqlite() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results([Vec::<dashboards::Model>::new()])
            .into_connection();
        let params = ListDashboardsParams {
            org_id: "orgId".to_owned(),
            folder_id: Some("folderId".to_owned()),
            title_pat: Some("tItLePat".to_owned()),
        };
        list_models(&db, params).await?;
        assert_eq!(
            db.into_transaction_log(),
            vec![Transaction::from_sql_and_values(
                DatabaseBackend::Sqlite,
                r#"SELECT "folders"."id" AS "A_id", "folders"."org" AS "A_org", "folders"."folder_id" AS "A_folder_id", "folders"."name" AS "A_name", "folders"."description" AS "A_description", "folders"."type" AS "A_type", "dashboards"."id" AS "B_id", "dashboards"."dashboard_id" AS "B_dashboard_id", "dashboards"."folder_id" AS "B_folder_id", "dashboards"."owner" AS "B_owner", "dashboards"."role" AS "B_role", "dashboards"."title" AS "B_title", "dashboards"."description" AS "B_description", "dashboards"."data" AS "B_data", "dashboards"."version" AS "B_version", "dashboards"."created_at" AS "B_created_at" FROM "folders" LEFT JOIN "dashboards" ON "folders"."id" = "dashboards"."folder_id" WHERE "folders"."org" = ? AND "folders"."type" = ? AND "folders"."folder_id" = ? AND LOWER("title") LIKE ? ORDER BY "dashboards"."title" ASC, "folders"."id" ASC"#,
                [
                    "orgId".into(),
                    0i16.into(),
                    "folderId".into(),
                    "%titlepat%".into()
                ]
            )]
        );
        Ok(())
    }
}
