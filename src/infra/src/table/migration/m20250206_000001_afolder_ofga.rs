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

use std::collections::HashSet;

// TODO: use flag to import the below crate
use o2_openfga::{authorizer, config::get_config as get_o2_config, meta::mapping::OFGA_MODELS};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !get_o2_config().openfga.enabled {
            return Ok(());
        }
        let db = manager.get_connection();
        let txn = db.begin().await?;
        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        let mut len = 0;
        let mut alert_pages = alerts::Entity::find()
            .order_by_asc(alerts::Column::Id)
            .paginate(&txn, 100);
        let mut alerts_set = HashSet::new();

        // First migrate the alert folders of every organizations
        let mut folder_pages = folders::Entity::find()
            .filter(folders::Column::Type.eq(1))
            .order_by_asc(folders::Column::Id)
            .paginate(&txn, 100);

        let alert_folders_ofga_type = OFGA_MODELS.get("alert_folders").unwrap().key;
        let alerts_ofga_type = OFGA_MODELS.get("alerts").unwrap().key;

        while let Some(folders) = folder_pages.fetch_and_next().await? {
            let folders_len = folders.len();
            len += folders_len;
            log::debug!("Processing {} records", folders_len);
            let mut tuples = vec![];
            for folder in folders {
                log::debug!(
                    "Processing record -> id: {}, org: {}",
                    folder.id,
                    folder.org,
                );
                let org_id = folder.org;
                let folder_id = folder.id;
                let obj_str = format!("{}:{}", alert_folders_ofga_type, folder_id);
                authorizer::authz::set_ownership(&org_id, &obj_str, "", "").await;
            }
        }
        log::info!("Processed {} folders for ofga migrations", len);

        // Next migrate all the alerts of every organizations
        len = 0;

        let mut orgs = HashSet::new();
        while let Some(alerts) = alert_pages.fetch_and_next().await? {
            let alerts_len = alerts.len();
            len += alerts_len;
            log::debug!("Processing {} records", alerts_len);
            let mut tuples = vec![];
            for alert in alerts {
                let org_id = alert.org;
                orgs.insert(org_id.clone());
                // Use the alert id
                let alert_id = alert.id;
                let alert_folder_id = alert.folder_id;
                let object = authorizer::authz::get_ownership_tuple(
                    &org_id,
                    "alerts",
                    &alert_id,
                    &mut tuples,
                );
                authorizer::authz::get_parent_tuple(
                    &alert_folder_id,
                    &alert_folders_ofga_type,
                    &mut tuples,
                );
            }
            if !tuples.is_empty() {
                match authorizer::authz::update_tuples(tuples, vec![]).await {
                    Ok(_) => {
                        log::debug!("{} alerts migrated to openfga", alerts_len);
                    }
                    Err(e) => {
                        log::error!("Error migrating alerts in openfga: {}", e);
                    }
                }
            }
        }

        log::info!("Processed {} alerts for ofga migrations", len);

        // 1. Get all the roles for every org
        for org in orgs.iter() {
            let roles = match authorizer::roles::get_all_roles(org, None).await {
                Ok(roles) => roles,
                Err(e) => {
                    log::error!("Error openfga getting roles for org: {}", e);
                    continue;
                }
            };
            // 2. Get all the alerts assigned to the roles
            for role in roles.iter() {
                let mut add_roles = vec![];
                let alerts =
                    match authorizer::roles::get_role_permissions(org, &role, &alerts_ofga_type)
                        .await
                    {
                        Ok(alerts) => alerts,
                        Err(e) => {
                            log::error!("Error openfga getting alerts for role: {}", e);
                            continue;
                        }
                    };
                // 3. Add the role assignment tuples for the alerts
                for mut alert in alerts.iter() {
                    // Get the alert id from the alert name
                    // TODO: Optimize this workflow

                    let alert_name = alert.object.split(':').last() else {
                        log::error!("Error openfga getting alert id from alert name");
                        continue;
                    };
                    if alert_name.starts_with("_all_") {
                        continue;
                    }

                    // Get the alert id from the alert name.
                    // There could be multiple alerts with the same name.
                    // Since we don't have a way to uniquely identify the alert,
                    // we are using all the alerts with the same name.
                    // TODO: Think of a better solution if possible.
                    let db_alerts = alerts::Entity::find()
                        .filter(alerts::Column::Name.eq(alert_name))
                        .filter(alerts::Column::Org.eq(org))
                        .all(&txn)
                        .await?;

                    for db_alert in db_alerts {
                        alert.object = format!("{}:{}", alerts_ofga_type, db_alert.id);
                        add_roles.push(alert);
                    }
                }
                if !add_roles.is_empty() {
                    match authorizer::roles::update_role(&org, &role, add_roles, vec![], None, None)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("{} roles added to openfga", add_roles.len());
                        }
                        Err(e) => {
                            log::error!("Error adding roles in openfga: {}", e);
                        }
                    }
                }
            }
        }

        // 4. Add the role assignment tuples for the alerts
        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

/// Representation of the meta table at the time this migration executes.
mod alerts {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "alerts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub org: String,
        pub folder_id: String,
        pub name: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod folders {

    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "folders")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub org: String,
        pub folder_id: String,
        pub name: String,
        pub r#type: i16,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
