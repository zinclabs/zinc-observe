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

//! An org should be able to have multiple folders with a `folder_id` of
//! `"default"` as long as those folders have different types. Therefore we need
//! to drop the `folders` table's unique index on the `org` and `folder_id`
//! columns and replace it with a unique index on the `org`, `type`, and
//! `folder_id` columns.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const OLD_FOLDERS_ORG_FOLDER_ID_IDX: &str = "folders_org_folder_id_idx";
const NEW_FOLDERS_ORG_TYPE_FOLDER_ID_IDX: &str = "folders_org_type_folder_id_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(OLD_FOLDERS_ORG_FOLDER_ID_IDX)
                    .table(Folders::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(create_new_folders_org_type_folder_id_idx_stmnt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(NEW_FOLDERS_ORG_TYPE_FOLDER_ID_IDX)
                    .table(Folders::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(create_old_folders_org_folder_id_idx_stmnt())
            .await?;
        Ok(())
    }
}

/// Statement to create the new unique index on org, type, and folder_id.
fn create_new_folders_org_type_folder_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(NEW_FOLDERS_ORG_TYPE_FOLDER_ID_IDX)
        .table(Folders::Table)
        .col(Folders::Org)
        .col(Folders::Type)
        .col(Folders::FolderId)
        .unique()
        .to_owned()
}

/// Statement to recreate the old unique index on org and folder_id.
fn create_old_folders_org_folder_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(OLD_FOLDERS_ORG_FOLDER_ID_IDX)
        .table(Folders::Table)
        .col(Folders::Org)
        .col(Folders::FolderId)
        .unique()
        .to_owned()
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Org,
    FolderId,
    Type,
}
