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
