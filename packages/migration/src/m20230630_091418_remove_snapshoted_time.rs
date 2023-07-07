use carbonable_domain::infrastructure::postgres::entity::YielderIden;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(YielderIden::Table)
                    .drop_column(YielderIden::SnapshotTime)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(YielderIden::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(YielderIden::SnapshotTime).date_time().null(),
                    )
                    .to_owned(),
            )
            .await
    }
}
