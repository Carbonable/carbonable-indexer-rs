use std::{sync::Arc, time::Duration};

use apibara_core::starknet::v1alpha2::{Block, Filter};
use apibara_sdk::{ClientBuilder, Uri};
use carbonable_domain::{
    domain::{
        event_source::{
            event_bus::{Consumer, EventBus},
            minter::{
                MinterAirdropEventConsumer, MinterBuyEventConsumer, MinterFilters,
                MinterMigrationEventConsumer,
            },
            offseter::{
                OffsetFilters, OffseterClaimEventConsumer, OffseterDepositEventConsumer,
                OffseterUpgradedEventConsumer, OffseterWithdrawEventConsumer,
            },
            project::{
                ProjectFilters, ProjectProjectValueUpdateEventConsumer,
                ProjectSlotChangedEventConsumer, ProjectTransferEventConsumer,
                ProjectTransferValueEventConsumer,
            },
            yielder::{
                YieldFilters, YielderClaimEventConsumer, YielderDepositEventConsumer,
                YielderProvisionEventConsumer, YielderSnapshotEventConsumer,
                YielderWithdrawEventConsumer,
            },
            BlockMetadata, DomainEvent, Filterable,
        },
        Erc3525, Erc721,
    },
    infrastructure::{
        app::{Cli, Commands},
        postgres::{
            event_store::{
                batch_events, clear_view_models, get_last_dispatched_block,
                store_last_handled_event,
            },
            get_connection, PostgresModels,
        },
        seed::{
            badge::BadgeSeeder, minter::MinterSeeder, offseter::OffseterSeeder,
            project::ProjectSeeder, yielder::YielderSeeder, DataSeeder, Seeder,
        },
    },
};

use carbonable_indexer::filters::configure_stream_filters;

use clap::Parser;
use deadpool_postgres::{Pool, Transaction};
use futures::TryStreamExt;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let db_client_pool = Arc::new(get_connection(None).await?);

    let cli = Cli::parse();
    match cli.commands {
        Commands::Seed {
            network,
            gateway: _,
            database_uri: _,
        } => {
            let file_path = format!("./data/{}.data.json", network);
            handle_seeding(db_client_pool.clone(), file_path.as_str()).await
        }
        Commands::Index {
            network,
            gateway: _,
            database_uri: _,
            apibara_uri,
            apibara_token,
            starting_block,
            batch_size: _,
            force,
        } => {
            let file_path = format!("./data/{}.data.json", network);
            handle_indexing(
                db_client_pool.clone(),
                file_path.as_str(),
                apibara_uri.as_str(),
                apibara_token.as_str(),
                starting_block.unwrap_or(0),
                force,
            )
            .await
        }
        Commands::EventStore {
            database_uri: _,
            gateway: _,
            network: _,
            flush,
        } => {
            if flush {
                return handle_refresh_event_store(db_client_pool.clone()).await;
            }
            handle_event_store(db_client_pool.clone()).await
        }
        _ => panic!("Unknown command"),
    }
}

/// Data seeding
/// * db_client_pool - [`Arc<Pool>`]
/// * configuration - [`&Args`]
/// * file_path - [`&str`]
///
async fn handle_seeding(
    db_client_pool: Arc<Pool>,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_models = Arc::new(PostgresModels::<Erc721>::new(db_client_pool.clone()));
    let db_models_3525 = Arc::new(PostgresModels::<Erc3525>::new(db_client_pool.clone()));

    let seeders: Vec<Arc<dyn Seeder + Send + Sync>> = vec![
        Arc::new(ProjectSeeder::<Erc721>::new(db_models.clone())),
        Arc::new(ProjectSeeder::<Erc3525>::new(db_models_3525.clone())),
        Arc::new(BadgeSeeder::<Erc721>::new(db_models.clone())),
        Arc::new(BadgeSeeder::<Erc3525>::new(db_models_3525.clone())),
        Arc::new(MinterSeeder::<Erc721>::new(db_models.clone())),
        Arc::new(MinterSeeder::<Erc3525>::new(db_models_3525.clone())),
        Arc::new(OffseterSeeder::<Erc721>::new(db_models.clone())),
        Arc::new(OffseterSeeder::<Erc3525>::new(db_models_3525.clone())),
        Arc::new(YielderSeeder::<Erc721>::new(db_models)),
        Arc::new(YielderSeeder::<Erc3525>::new(db_models_3525)),
    ];

    match DataSeeder::feed_from_data(&file_path, seeders)
        .await?
        .seed()
        .await
    {
        Ok(_) => {
            info!("Data seeded sucessfully");
            Ok(())
        }
        Err(e) => {
            error!("Data seeding failed: {:#?}", e);
            Err(Box::new(e))
        }
    }
}

/// Runs events from blockchain to register them in local event_store
/// * db_client_pool - [`Arc<Pool>`]
/// * configuration - [`&Args`]
/// * file_path - [`&str`]
///
async fn handle_indexing(
    db_client_pool: Arc<Pool>,
    file_path: &str,
    apibara_uri: &str,
    apibara_token: &str,
    starting_block: u64,
    force: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut filters: [Box<dyn Filterable>; 4] = [
        Box::new(ProjectFilters::new()),
        Box::new(YieldFilters::new()),
        Box::new(OffsetFilters::new()),
        Box::new(MinterFilters::new()),
    ];
    let mut last_block_id = starting_block;
    if !force {
        last_block_id = starting_block;
        last_block_id = get_last_dispatched_block(&db_client_pool, &last_block_id).await?;
    }
    info!("Starting stream from block : {}", last_block_id);

    let stream_config = configure_stream_filters(&file_path, &mut filters, &last_block_id)?;

    let (mut stream, configuration_handle) = ClientBuilder::<Filter, Block>::default()
        .with_bearer_token(apibara_token.to_owned())
        .connect(Uri::from_static(Box::leak(
            apibara_uri.to_owned().into_boxed_str(),
        )))
        .await?;

    configuration_handle.send(stream_config.clone()).await?;
    let event_bus = create_event_bus(db_client_pool.clone());

    loop {
        match stream.try_next().await {
            Ok(Some(response)) => match response {
                apibara_sdk::DataMessage::Data {
                    cursor,
                    end_cursor,
                    finality: _,
                    batch,
                } => {
                    info!(
                        "Handling data within {} and {}",
                        cursor.expect("should have starting cursor").order_key,
                        end_cursor.order_key
                    );

                    for block in batch {
                        let metadata =
                            BlockMetadata::from(block.header.expect("should have blockheader"));
                        last_block_id = metadata.get_block();
                        debug!("Block id: {}", last_block_id);
                        let mut last_event_idx = 0;
                        let mut last_processed_felt = String::new();
                        for event in block.events {
                            let mut event = DomainEvent::from_starknet_event(
                                event,
                                &mut filters,
                                &mut last_event_idx,
                                &mut last_processed_felt,
                            );
                            event = event.with_metadata(&metadata.clone());
                            event_bus.register(&event, &metadata).await?;
                        }
                    }
                }
                apibara_sdk::DataMessage::Invalidate { cursor } => match cursor {
                    Some(c) => {
                        error!("Received an invalidate request data at {}", &c.order_key)
                    }
                    None => error!("Invalidate request without cursor provided"),
                },
            },
            Ok(None) => continue,
            Err(e) => {
                error!("Error while streaming: {}", e);
                // info!("Restarting stream");
                // let restarting_cfg = stream_config;
                // let cfg = restarting_cfg.with_starting_block(last_block_id);
                //
                // configuration_handle.send(cfg.clone()).await?;

                panic!("Restarting stream");
            }
        }
    }
}

async fn handle_event_store(db_client_pool: Arc<Pool>) -> Result<(), Box<dyn std::error::Error>> {
    let event_bus = create_event_bus(db_client_pool.clone());
    let client = db_client_pool.clone().get().await?;
    let mut key = None;
    while let batch = batch_events(&client, 10, key).await? {
        if 0 == batch.len() {
            sleep(Duration::from_secs(5)).await;
            continue;
        }
        for event in batch.as_slice() {
            let domain_event = DomainEvent::from(event);
            let metadata = BlockMetadata::from(event);
            match event_bus
                .consume_event_store(&domain_event, &metadata)
                .await
            {
                Ok(_) => debug!("Properly hydrated event : {}", event.event_id.to_string()),
                Err(e) => error!(
                    "Error while hydrating event: {}\n{}",
                    event.event_id.to_string(),
                    e
                ),
            }
        }
        key = Some(batch.last().unwrap().id.clone());
        let _ = store_last_handled_event(&client, key).await;
    }

    Ok(())
}

async fn handle_refresh_event_store(
    db_client_pool: Arc<Pool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = db_client_pool.clone().get().await?;

    Ok(clear_view_models(&client).await?)
}

fn create_event_bus(
    db_client_pool: Arc<Pool>,
) -> EventBus<Pool, Box<dyn for<'a> Consumer<Transaction<'a>>>> {
    let mut event_bus =
        EventBus::<Pool, Box<dyn for<'a> Consumer<Transaction<'a>>>>::new(db_client_pool.clone());

    // Project
    event_bus.add_consumer(Box::new(ProjectTransferEventConsumer::new()));
    event_bus.add_consumer(Box::new(ProjectTransferValueEventConsumer::new()));
    event_bus.add_consumer(Box::new(ProjectSlotChangedEventConsumer::new()));
    event_bus.add_consumer(Box::new(ProjectProjectValueUpdateEventConsumer::new()));
    // Yielder
    event_bus.add_consumer(Box::new(YielderClaimEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderDepositEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderProvisionEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderSnapshotEventConsumer::new()));
    event_bus.add_consumer(Box::new(YielderWithdrawEventConsumer::new()));
    // Offseter
    event_bus.add_consumer(Box::new(OffseterUpgradedEventConsumer::new()));
    event_bus.add_consumer(Box::new(OffseterDepositEventConsumer::new()));
    event_bus.add_consumer(Box::new(OffseterClaimEventConsumer::new()));
    event_bus.add_consumer(Box::new(OffseterWithdrawEventConsumer::new()));
    //Minter
    event_bus.add_consumer(Box::new(MinterMigrationEventConsumer::new()));
    event_bus.add_consumer(Box::new(MinterAirdropEventConsumer::new()));
    event_bus.add_consumer(Box::new(MinterBuyEventConsumer::new()));

    event_bus
}
