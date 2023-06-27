use carbonable_domain::infrastructure::postgres::entity::{
    ProvisionIden, SnapshotIden, YielderIden,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop snapshot table
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(SnapshotIden::Table)
                    .name("snapshot_yielder_id_fkey")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(SnapshotIden::Table).to_owned())
            .await?;

        // Drop provision table
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(ProvisionIden::Table)
                    .name("provision_yielder_id_fkey")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(ProvisionIden::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(YielderIden::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(YielderIden::Prices)
                            .array(ColumnType::Binary(BlobSize::Medium)),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SnapshotIden::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SnapshotIden::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::PreviousTime)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::PreviousProjectAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::PreviousYielderAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::PreviousOffseterAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::CurrentProjectAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::CurrentYielderAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::CurrentOffseterAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::ProjectAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::YielderAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SnapshotIden::OffseterAbsorption)
                            .binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SnapshotIden::Time).date_time().not_null())
                    .col(ColumnDef::new(SnapshotIden::YielderId).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("snapshot_yielder_id_fkey")
                            .from(SnapshotIden::Table, SnapshotIden::YielderId)
                            .to(YielderIden::Table, YielderIden::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("snapshot_time_idx")
                            .table(SnapshotIden::Table)
                            .col(SnapshotIden::YielderId)
                            .col(SnapshotIden::Time)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProvisionIden::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProvisionIden::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ProvisionIden::Amount).binary().not_null())
                    .col(ColumnDef::new(ProvisionIden::Time).date_time().not_null())
                    .col(ColumnDef::new(ProvisionIden::YielderId).uuid().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("provision_yielder_id_fkey")
                            .from(ProvisionIden::Table, ProvisionIden::YielderId)
                            .to(YielderIden::Table, YielderIden::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
}
