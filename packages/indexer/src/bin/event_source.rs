use carbonable_domain::{
    domain::event_source::{
        event_bus::{Consumer, EventBus},
        minter::{
            MinterAirdropEventConsumer, MinterBuyEventConsumer, MinterMigrationEventConsumer,
        },
        project::{
            ProjectSlotChangedEventConsumer, ProjectTransferEventConsumer,
            ProjectTransferValueEventConsumer,
        },
        yielder::{
            YielderClaimEventConsumer, YielderDepositEventConsumer, YielderProvisionEventConsumer,
            YielderSnapshotEventConsumer, YielderWithdrawEventConsumer,
        },
        BlockMetadata, DomainEvent, Event,
    },
    infrastructure::{app::configure_application, postgres::get_connection},
};
use deadpool_postgres::{Pool, Transaction};
use futures_util::TryStreamExt;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let configuration = configure_application().await?;
    let db_client_pool = Arc::new(get_connection(None).await?);
    let pool = sqlx::PgPool::connect(&configuration.database_uri).await?;

    let mut event_bus =
        EventBus::<Pool, Box<dyn for<'a> Consumer<Transaction<'a>>>>::new(db_client_pool.clone());

    // Project
    event_bus.add_consumer(Box::new(ProjectTransferEventConsumer::new()));
    event_bus.add_consumer(Box::new(ProjectTransferValueEventConsumer::new()));
    event_bus.add_consumer(Box::new(ProjectSlotChangedEventConsumer::new()));
    // Yielder
    event_bus.add_consumer(Box::new(YielderClaimEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderDepositEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderProvisionEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderSnapshotEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderWithdrawEventConsumer::new()));
    //Minter
    event_bus.add_consumer(Box::new(MinterMigrationEventConsumer::new()));
    event_bus.add_consumer(Box::new(MinterAirdropEventConsumer::new()));
    event_bus.add_consumer(Box::new(MinterBuyEventConsumer::new()));

    let mut rows = sqlx::query_as!(
        DomainEvent,
        r#"SELECT event_id as "id: String", metadata as "metadata!: _", payload as "payload!: _", es."r#type" as "type: Event" FROM event_store es;"#
    )
    .fetch(&pool);

    while let Some(domain_event) = rows.try_next().await? {
        event_bus.replay_events(&domain_event).await?;
    }

    Ok(())
}
