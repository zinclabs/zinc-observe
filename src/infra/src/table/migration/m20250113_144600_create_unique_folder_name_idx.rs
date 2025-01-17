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

//! Folder names within an individual organization and within a specific folder
//! type should be unique.
//!
//! This migration renames any folders that have the same nane, organization,
//! and folder type to ensure that they have unique names. It then creates a
//! unique constraint on the composite key of `org`, `type`, and `name`.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, FromQueryResult, IntoActiveModel, QueryFilter,
    QuerySelect, Select, Set, TransactionTrait,
};
use sea_orm_migration::prelude::*;
use svix_ksuid::KsuidLike;

#[derive(DeriveMigrationName)]
pub struct Migration;

const FOLDERS_ORG_TYPE_NAME_IDX: &str = "folders_org_type_name_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;
        rename_duplicates(&txn).await?;
        txn.commit().await?;

        manager
            .create_index(create_new_folders_org_type_name_idx_stmnt())
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // This only reverse the creation of the new index. It does not rename
        // the folders back to their original names.
        manager
            .drop_index(
                Index::drop()
                    .name(FOLDERS_ORG_TYPE_NAME_IDX)
                    .table(Folders::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

/// Renames any folders that have the same nane, organization, and folder type
/// by appending a KSUID to the folder name to ensure that folder names are
/// unique.
async fn rename_duplicates<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
    let duplicates = FolderCount::select_all(conn)
        .await?
        .into_iter()
        .filter(|f| f.count > 1);

    for d in duplicates {
        let folders = folders::Entity::find()
            .filter(folders::Column::Org.eq(d.org))
            .filter(folders::Column::Type.eq(d.r#type))
            .filter(folders::Column::Name.eq(d.name))
            .all(conn)
            .await?;

        for f in folders {
            let name = f.name.clone();
            let ksuid = svix_ksuid::Ksuid::new(None, None);
            let mut am = f.into_active_model();
            am.name = Set(format!("{} {}", name, ksuid));
            am.update(conn).await?;
        }
    }

    Ok(())
}

/// Result of querying for folders that have the same nane, organization, and
/// folder type.
#[derive(FromQueryResult)]
struct FolderCount {
    org: String,
    r#type: i16,
    name: String,

    /// The count of folders that have this `org`, `type`, and `name`.
    count: i64,
}

impl FolderCount {
    /// Selects all folders grouped by their `org`, `type`, and `name.`
    async fn select_all<C: ConnectionTrait>(conn: &C) -> Result<Vec<Self>, DbErr> {
        Self::select_statement()
            .into_model::<Self>()
            .all(conn)
            .await
    }

    /// Returns a statement to select folders grouped by their `org`, `type`,
    /// and `name`.
    fn select_statement() -> Select<folders::Entity> {
        folders::Entity::find()
            .select_only()
            .column(folders::Column::Org)
            .column(folders::Column::Type)
            .column(folders::Column::Name)
            .column_as(folders::Column::Id.count(), "count")
            .group_by(folders::Column::Org)
            .group_by(folders::Column::Type)
            .group_by(folders::Column::Name)
    }
}

/// Statement to create the new unique index on org, type, and name.
fn create_new_folders_org_type_name_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(FOLDERS_ORG_TYPE_NAME_IDX)
        .table(Folders::Table)
        .col(Folders::Org)
        .col(Folders::Type)
        .col(Folders::Name)
        .unique()
        .to_owned()
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Org,
    Name,
    Type,
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

/// Representation of the folders table at the time this migration executes.
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
        #[sea_orm(column_type = "Text", nullable)]
        pub description: Option<String>,
        pub r#type: i16,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[cfg(test)]
mod tests {
    use collapse::*;
    use sea_orm::{DbBackend, QueryTrait};

    use super::*;

    #[test]
    fn duplicates_postgres() {
        let sql = FolderCount::select_statement()
            .build(DbBackend::Postgres)
            .to_string();
        collapsed_eq!(
            &sql,
            r#"
                SELECT 
                "folders"."org",
                "folders"."type",
                "folders"."name",
                COUNT("folders"."id") AS "count" 
                FROM "folders" 
                GROUP BY 
                "folders"."org",
                "folders"."type",
                "folders"."name"
            "#
        );
    }

    #[test]
    fn duplicates_mysql() {
        let sql = FolderCount::select_statement()
            .build(DbBackend::MySql)
            .to_string();
        collapsed_eq!(
            &sql,
            r#"
                SELECT
                `folders`.`org`,
                `folders`.`type`,
                `folders`.`name`,
                COUNT(`folders`.`id`) AS `count`
                FROM `folders` 
                GROUP BY 
                `folders`.`org`,
                `folders`.`type`,
                `folders`.`name`
            "#
        );
    }

    #[test]
    fn duplicates_sqlite() {
        let sql = FolderCount::select_statement()
            .build(DbBackend::Sqlite)
            .to_string();
        collapsed_eq!(
            &sql,
            r#"
                SELECT 
                "folders"."org",
                "folders"."type",
                "folders"."name",
                COUNT("folders"."id") AS "count" 
                FROM "folders" 
                GROUP BY 
                "folders"."org",
                "folders"."type",
                "folders"."name"
            "#
        );
    }
}
