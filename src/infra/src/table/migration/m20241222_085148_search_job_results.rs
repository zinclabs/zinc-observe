use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_table_stmt()).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SearchJobResults::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create table.
fn create_table_stmt() -> TableCreateStatement {
    Table::create()
        .table(SearchJobResults::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SearchJobResults::JobId)
                .string_len(64)
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobResults::TraceId)
                .string_len(64)
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobResults::StartedAt).big_integer())
        .col(ColumnDef::new(SearchJobResults::EndedAt).big_integer())
        .col(ColumnDef::new(SearchJobResults::Cluster).string_len(256))
        .col(ColumnDef::new(SearchJobResults::ResultPath).string_len(512))
        .col(ColumnDef::new(SearchJobResults::ErrorMessage).text())
        .primary_key(
            Index::create()
                .name("pk_search_job_results")
                .col(SearchJobResults::JobId)
                .col(SearchJobResults::TraceId),
        )
        .to_owned()
}

#[derive(DeriveIden)]
enum SearchJobResults {
    Table,
    JobId,
    TraceId,
    StartedAt,
    EndedAt,
    Cluster,
    ResultPath,
    ErrorMessage,
}
