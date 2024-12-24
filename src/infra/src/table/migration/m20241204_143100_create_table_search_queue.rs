use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const SEARCH_QUEUE_WORK_GROUP_IDX: &str = "search_queue_work_group_idx";
const SEARCH_QUEUE_CREATED_AT_IDX: &str = "search_queue_created_at_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_table_stmt()).await?;
        manager.create_index(create_index_work_group_stmt()).await?;
        manager.create_index(create_index_created_at_stmt()).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name(SEARCH_QUEUE_WORK_GROUP_IDX)
                    .table(SearchQueue::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name(SEARCH_QUEUE_CREATED_AT_IDX)
                    .table(SearchQueue::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(SearchQueue::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create table.
fn create_table_stmt() -> TableCreateStatement {
    Table::create()
        .table(SearchQueue::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SearchQueue::Id)
                .big_integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(SearchQueue::WorkGroup).string_len(16).not_null())
        .col(ColumnDef::new(SearchQueue::UserId).string_len(256).not_null())
        // Identifier of the user that owns the dashboard.
        .col(ColumnDef::new(SearchQueue::TraceId).string_len(64).not_null())
        .col(
            ColumnDef::new(SearchQueue::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .to_owned()
}

/// Statement to create index work_group.
fn create_index_work_group_stmt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(SEARCH_QUEUE_WORK_GROUP_IDX)
        .table(SearchQueue::Table)
        .col(SearchQueue::WorkGroup)
        .col(SearchQueue::UserId)
        .to_owned()
}

/// Statement to create index created_at.
fn create_index_created_at_stmt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(SEARCH_QUEUE_CREATED_AT_IDX)
        .table(SearchQueue::Table)
        .col(SearchQueue::CreatedAt)
        .to_owned()
}

#[derive(DeriveIden)]
enum SearchQueue {
    Table,
    Id,
    WorkGroup,
    UserId,
    TraceId,
    CreatedAt,
}
