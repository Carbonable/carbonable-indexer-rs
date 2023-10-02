use carbonable_domain::infrastructure::postgres::entity::{CustomerTokenIden, ErcImplementation};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CustomerTokenIden::Table)
                    .col(
                        ColumnDef::new(CustomerTokenIden::Id)
                            .string()
                            .string_len(26)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CustomerTokenIden::Address)
                            .string()
                            .string_len(66)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerTokenIden::ProjectAddress)
                            .string()
                            .string_len(66)
                            .not_null(),
                    )
                    .col(ColumnDef::new(CustomerTokenIden::Slot).binary().null())
                    .col(
                        ColumnDef::new(CustomerTokenIden::TokenId)
                            .binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CustomerTokenIden::Value).binary().null())
                    .col(
                        ColumnDef::new(CustomerTokenIden::ValueDecimals)
                            .binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerTokenIden::ErcImplementation)
                            .enumeration(
                                ErcImplementation::Enum,
                                [ErcImplementation::Erc721, ErcImplementation::Erc3525],
                            )
                            .null(),
                    )
                    .col(ColumnDef::new(CustomerTokenIden::UnitPrice).binary().null())
                    .col(
                        ColumnDef::new(CustomerTokenIden::PriceSymbol)
                            .string()
                            .string_len(20)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerTokenIden::PriceDecimals)
                            .binary()
                            .null(),
                    )
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CustomerTokenIden::Table).to_owned())
            .await
    }
}
