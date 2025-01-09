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
            .drop_table(Table::drop().table(SearchJobPartitions::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create table.
fn create_table_stmt() -> TableCreateStatement {
    Table::create()
        .table(SearchJobPartitions::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SearchJobPartitions::JobId)
                .string_len(64)
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobPartitions::PartitionId)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobPartitions::StartTime)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobPartitions::EndTime)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(SearchJobPartitions::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobPartitions::StartedAt).big_integer())
        .col(ColumnDef::new(SearchJobPartitions::EndedAt).big_integer())
        .col(ColumnDef::new(SearchJobPartitions::Cluster).string_len(256))
        .col(
            ColumnDef::new(SearchJobPartitions::Status)
                .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(SearchJobPartitions::ResultPath).string_len(512))
        .col(ColumnDef::new(SearchJobPartitions::ErrorMessage).text())
        .primary_key(
            Index::create()
                .name("pk_search_job_partitions")
                .col(SearchJobPartitions::JobId)
                .col(SearchJobPartitions::PartitionId),
        )
        .to_owned()
}

#[derive(DeriveIden)]
enum SearchJobPartitions {
    Table,
    JobId,
    PartitionId,
    StartTime,
    EndTime,
    CreatedAt,
    StartedAt,
    EndedAt,
    Cluster,
    Status,
    ResultPath,
    ErrorMessage,
}
