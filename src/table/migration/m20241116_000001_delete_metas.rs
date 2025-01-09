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

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.exec_stmt(delete_meta_folders_stmnt()).await?;
        Ok(())
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        // The deletion of records from the meta table is not reversable.
        Ok(())
    }
}

/// Statement to delete folder records from the meta table.
fn delete_meta_folders_stmnt() -> DeleteStatement {
    Query::delete()
        .from_table(Meta::Table)
        .and_where(Expr::col(Meta::Module).eq("folders"))
        .to_owned()
}

/// Identifiers used in queries on the meta table.
#[derive(DeriveIden)]
enum Meta {
    Table,
    Module,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &delete_meta_folders_stmnt().to_string(PostgresQueryBuilder),
            r#"DELETE FROM "meta" WHERE "module" = 'folders'"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &delete_meta_folders_stmnt().to_string(MysqlQueryBuilder),
            r#"DELETE FROM `meta` WHERE `module` = 'folders'"#
        );
    }

    #[test]
    fn sqlite() {
        assert_eq!(
            &delete_meta_folders_stmnt().to_string(SqliteQueryBuilder),
            r#"DELETE FROM "meta" WHERE "module" = 'folders'"#
        );
    }
}
