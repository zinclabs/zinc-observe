use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const DASHBOARDS_FOLDERS_FK: &str = "dashboards_folders_fk";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_dashboards_table_statement())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Dashboards::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the dashboards table.
fn create_dashboards_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Dashboards::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Dashboards::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        // A user-facing ID for the folder. This value can be a 64-bit signed
        // integer "snowflake".
        .col(ColumnDef::new(Dashboards::DashboardId).string_len(256).not_null())
        // Foreign key to the folders table.
        .col(ColumnDef::new(Dashboards::FolderId).big_integer().not_null())
        // Identifier of the user that owns the dashboard.
        .col(ColumnDef::new(Dashboards::Owner).string_len(256).not_null())
        // TODO: What does this column for?
        .col(ColumnDef::new(Dashboards::Role).string_len(256).null())
        .col(ColumnDef::new(Dashboards::Title).string_len(256).not_null())
        .col(ColumnDef::new(Dashboards::Description).text())
        .col(ColumnDef::new(Dashboards::Data).json().not_null())
        .col(ColumnDef::new(Dashboards::Version).integer().not_null())
        .col(
            ColumnDef::new(Dashboards::CreatedAt)
                .timestamp()
                .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp))
                .not_null(),
        )
        .foreign_key(    
            sea_query::ForeignKey::create()
                    .name(DASHBOARDS_FOLDERS_FK)
                    .from(Dashboards::Table, Dashboards::FolderId)
                    .to(Folders::Table, Folders::Id)
        )
        .to_owned()
}


/// Identifiers used in queries on the dashboards table.
#[derive(DeriveIden)]
enum Dashboards {
    Table,
    Id,
    DashboardId,
    FolderId,
    Owner,
    Role,
    Title,
    Description,
    Data,
    Version,
    CreatedAt,
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Id,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &create_dashboards_table_statement().to_string(PostgresQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "dashboards" ( 
                "id" bigserial NOT NULL PRIMARY KEY, 
                "dashboard_id" varchar(256) NOT NULL, 
                "folder_id" bigint NOT NULL, 
                "owner" varchar(256) NOT NULL, 
                "role" varchar(256) NULL, 
                "title" varchar(256) NOT NULL, 
                "description" text, 
                "data" json NOT NULL,
                "version" integer NOT NULL, 
                "created_at" timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL, 
                CONSTRAINT "dashboards_folders_fk" FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") 
            )"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_dashboards_table_statement().to_string(MysqlQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS `dashboards` ( 
                `id` bigint NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `folder_id` bigint NOT NULL, 
                `owner` varchar(256) NOT NULL, 
                `role` varchar(256) NULL, 
                `title` varchar(256) NOT NULL, 
                `description` text,
                `data` json NOT NULL,
                `version` int NOT NULL, 
                `created_at` timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL, 
                CONSTRAINT `dashboards_folders_fk` FOREIGN KEY (`folder_id`) REFERENCES `folders` (`id`) 
            )"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_dashboards_table_statement().to_string(SqliteQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "dashboards" ( 
                "id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
                "folder_id" bigint NOT NULL, 
                "owner" varchar(256) NOT NULL, 
                "role" varchar(256) NULL, 
                "title" varchar(256) NOT NULL, 
                "description" text, 
                "data" json_text NOT NULL, 
                "version" integer NOT NULL, 
                "created_at" timestamp_text DEFAULT CURRENT_TIMESTAMP NOT NULL, 
                FOREIGN KEY ("folder_id") REFERENCES "folders" ("id") 
            )"#
        );
    }
}
