use carbonable_domain::infrastructure::postgres::entity::ProjectIden;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// .col(ColumnDef::new(UriIden::Data).json().not_null())
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ProjectIden::Table)
                    .add_column_if_not_exists(ColumnDef::new(ProjectIden::Metadata).json().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ProjectIden::Table)
                    .drop_column(ProjectIden::Metadata)
                    .to_owned(),
            )
            .await
    }
}
