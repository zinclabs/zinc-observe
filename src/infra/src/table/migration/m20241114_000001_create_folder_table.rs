use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const FOLDER_ORG_IDX: &str = "folder_org_idx";
const FOLDER_FOLDER_ID_IDX: &str = "folder_folder_id_idx";
const FOLDER_ORG_NAME_TYPE_IDX: &str = "folder_org_name_type_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_folder_table_statement())
            .await?;
        manager.create_index(create_folder_org_idx_stmnt()).await?;
        manager
            .create_index(create_folder_folder_id_idx_stmnt())
            .await?;
        manager
            .create_index(create_folder_org_name_type_idx_stmnt())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(FOLDER_ORG_NAME_TYPE_IDX).to_owned())
            .await?;
        manager
            .drop_index(Index::drop().name(FOLDER_FOLDER_ID_IDX).to_owned())
            .await?;
        manager
            .drop_index(Index::drop().name(FOLDER_ORG_IDX).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Folder::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the folder table.
fn create_folder_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Folder::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Folder::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        // Surrogate key for folders that at the time of writing is...
        // - stored in meta.key2
        // - set by the server in calls to CreateFolder
        // - referenced by the client in calls to ListDashboards
        .col(ColumnDef::new(Folder::FolderId).string_len(256).not_null())
        .col(ColumnDef::new(Folder::Org).string_len(100).not_null())
        .col(ColumnDef::new(Folder::Name).string_len(256).not_null())
        .col(ColumnDef::new(Folder::Description).text())
        // Folder type where...
        // - 0 is a dashboards folder
        // - 1 is an alerts folder
        // - 2 is a reports folder
        .col(ColumnDef::new(Folder::Type).small_integer().not_null())
        .col(
            ColumnDef::new(Folder::CreatedAt)
                .timestamp()
                .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp))
                .not_null(),
        )
        .to_owned()
}

/// Statement to create index on org.
fn create_folder_org_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(FOLDER_ORG_IDX)
        .table(Folder::Table)
        .col(Folder::Org)
        .to_owned()
}

/// Statement to create unique index on folder_id surrogate key.
fn create_folder_folder_id_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(FOLDER_FOLDER_ID_IDX)
        .table(Folder::Table)
        .col(Folder::FolderId)
        .unique()
        .to_owned()
}

/// Statement to create unique index on org, name, type combination.
fn create_folder_org_name_type_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(FOLDER_ORG_NAME_TYPE_IDX)
        .table(Folder::Table)
        .col(Folder::Org)
        .col(Folder::Name)
        .col(Folder::Type)
        .unique()
        .to_owned()
}

/// Identifiers used in queries on the folder table.
#[derive(DeriveIden)]
enum Folder {
    Table,
    Id,
    FolderId,
    Org,
    Name,
    Description,
    Type,
    CreatedAt,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &create_folder_table_statement().to_string(PostgresQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "folder" (
                "id" bigserial NOT NULL PRIMARY KEY,
                "folder_id" varchar(256) NOT NULL,
                "org" varchar(100) NOT NULL,
                "name" varchar(256) NOT NULL,
                "description" text,
                "type" smallint NOT NULL,
                "created_at" timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL
            )"#
        );
        assert_eq!(
            &create_folder_org_idx_stmnt().to_string(PostgresQueryBuilder),
            r#"CREATE INDEX IF NOT EXISTS "folder_org_idx" ON "folder" ("org")"#
        );
        assert_eq!(
            &create_folder_folder_id_idx_stmnt().to_string(PostgresQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "folder_folder_id_idx" ON "folder" ("folder_id")"#
        );
        assert_eq!(
            &create_folder_org_name_type_idx_stmnt().to_string(PostgresQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "folder_org_name_type_idx" ON "folder" ("org", "name", "type")"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_folder_table_statement().to_string(MysqlQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS `folder` (
                `id` bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `folder_id` varchar(256) NOT NULL,
                `org` varchar(100) NOT NULL,
                `name` varchar(256) NOT NULL,
                `description` text,
                `type` smallint NOT NULL,
                `created_at` timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL
            )"#
        );
        assert_eq!(
            &create_folder_org_idx_stmnt().to_string(MysqlQueryBuilder),
            r#"CREATE INDEX `folder_org_idx` ON `folder` (`org`)"#
        );
        assert_eq!(
            &create_folder_folder_id_idx_stmnt().to_string(MysqlQueryBuilder),
            r#"CREATE UNIQUE INDEX `folder_folder_id_idx` ON `folder` (`folder_id`)"#
        );
        assert_eq!(
            &create_folder_org_name_type_idx_stmnt().to_string(MysqlQueryBuilder),
            r#"CREATE UNIQUE INDEX `folder_org_name_type_idx` ON `folder` (`org`, `name`, `type`)"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_folder_table_statement().to_string(SqliteQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "folder" (
                "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
                "folder_id" varchar(256) NOT NULL,
                "org" varchar(100) NOT NULL,
                "name" varchar(256) NOT NULL,
                "description" text,
                "type" smallint NOT NULL,
                "created_at" timestamp_text DEFAULT CURRENT_TIMESTAMP NOT NULL
            )"#
        );
        assert_eq!(
            &create_folder_org_idx_stmnt().to_string(SqliteQueryBuilder),
            r#"CREATE INDEX IF NOT EXISTS "folder_org_idx" ON "folder" ("org")"#
        );
        assert_eq!(
            &create_folder_folder_id_idx_stmnt().to_string(SqliteQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "folder_folder_id_idx" ON "folder" ("folder_id")"#
        );
        assert_eq!(
            &create_folder_org_name_type_idx_stmnt().to_string(SqliteQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "folder_org_name_type_idx" ON "folder" ("org", "name", "type")"#
        );
    }
}
