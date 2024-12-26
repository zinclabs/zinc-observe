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
            .drop_table(Table::drop().table(SearchJobs::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create table.
fn create_table_stmt() -> TableCreateStatement {
    Table::create()
        .table(SearchJobs::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SearchJobs::Id)
                .string_len(64)
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(SearchJobs::TraceId)
                .string_len(64)
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobs::OrgId).string_len(256).not_null())
        .col(
            ColumnDef::new(SearchJobs::UserId)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobs::StreamType)
                .string_len(256)
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobs::StreamNames)
                .string_len(256)
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobs::Payload).text().not_null())
        .col(
            ColumnDef::new(SearchJobs::StartTime)
                .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobs::EndTime).big_integer().not_null())
        .col(
            ColumnDef::new(SearchJobs::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobs::UpdatedAt)
                .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobs::StartedAt).big_integer())
        .col(ColumnDef::new(SearchJobs::EndedAt).big_integer())
        .col(ColumnDef::new(SearchJobs::Cluster).string_len(256))
        .col(ColumnDef::new(SearchJobs::Node).string_len(256))
        .col(ColumnDef::new(SearchJobs::Status).big_integer().not_null())
        .col(ColumnDef::new(SearchJobs::ResultPath).string_len(512))
        .col(ColumnDef::new(SearchJobs::ErrorMessage).text())
        .col(ColumnDef::new(SearchJobs::PartitionNum).big_integer())
        .to_owned()
}

#[derive(DeriveIden)]
enum SearchJobs {
    Table,
    Id,
    TraceId,
    OrgId,
    UserId,
    StreamType,
    StreamNames,
    Payload,
    StartTime,
    EndTime,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
    Cluster,
    Node,
    Status,
    ResultPath,
    ErrorMessage,
    PartitionNum,
}
