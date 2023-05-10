use carbonable_domain::infrastructure::postgres::entity::MinterIden;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MinterIden::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(MinterIden::SaleDate).date_time().null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MinterIden::Table)
                    .drop_column(MinterIden::SaleDate)
                    .to_owned(),
            )
            .await
    }
}
