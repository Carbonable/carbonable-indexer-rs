use carbonable_domain::infrastructure::postgres::entity::{
    ActionType, CustomerFarmIden, FarmType, LastStoredEventIden,
};
use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};
use sea_query::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .create_type(
                Type::create()
                    .as_enum(FarmType::Enum)
                    .values([FarmType::Yield, FarmType::Offset])
                    .to_owned(),
            )
            .await?;
        manager
            .create_type(
                Type::create()
                    .as_enum(ActionType::Enum)
                    .values([ActionType::Withdraw, ActionType::Deposit, ActionType::Claim])
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(LastStoredEventIden::Table)
                    .col(
                        ColumnDef::new(LastStoredEventIden::Id)
                            .string()
                            .string_len(26)
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared("INSERT INTO last_stored_event (id) VALUES ('')")
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(CustomerFarmIden::Table)
                    .col(
                        ColumnDef::new(CustomerFarmIden::Id)
                            .string()
                            .string_len(26)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CustomerFarmIden::CustomerAddress)
                            .string()
                            .string_len(66)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerFarmIden::ProjectAddress)
                            .string()
                            .string_len(66)
                            .not_null(),
                    )
                    .col(ColumnDef::new(CustomerFarmIden::Slot).binary().null())
                    .col(ColumnDef::new(CustomerFarmIden::Value).binary().null())
                    .col(
                        ColumnDef::new(CustomerFarmIden::FarmType)
                            .enumeration(FarmType::Enum, [FarmType::Yield, FarmType::Offset])
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerFarmIden::ActionType)
                            .enumeration(
                                ActionType::Enum,
                                [ActionType::Withdraw, ActionType::Deposit, ActionType::Claim],
                            )
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerFarmIden::EventId)
                            .string()
                            .unique_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerFarmIden::EventTimestamp)
                            .date_time()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CustomerFarmIden::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LastStoredEventIden::Table).to_owned())
            .await?;
        manager
            .drop_type(Type::drop().if_exists().name(ActionType::Enum).to_owned())
            .await?;
        manager
            .drop_type(Type::drop().if_exists().name(FarmType::Enum).to_owned())
            .await
    }
}
