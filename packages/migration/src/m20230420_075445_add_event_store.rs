use carbonable_domain::infrastructure::postgres::entity::EventStoreIden;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EventStoreIden::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventStoreIden::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EventStoreIden::EventId).string().not_null())
                    .col(
                        ColumnDef::new(EventStoreIden::BlockNumber)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventStoreIden::BlockHash)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EventStoreIden::Metadata).json().null())
                    .col(ColumnDef::new(EventStoreIden::Payload).json().null())
                    .col(
                        ColumnDef::new(EventStoreIden::RecordedAt)
                            .date_time()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EventStoreIden::Table).to_owned())
            .await
    }
}
