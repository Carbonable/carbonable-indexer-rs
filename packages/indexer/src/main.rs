use std::sync::Arc;

use apibara_core::starknet::v1alpha2::{Block, Filter};
use apibara_sdk::{ClientBuilder, Uri};
use carbonable_domain::{
    domain::{
        event_source::{
            event_bus::{Consumer, EventBus},
            project::{
                ProjectFilters, ProjectSlotChangedEventConsumer, ProjectTransferEventConsumer,
                ProjectTransferValueEventConsumer,
            },
            BlockMetadata, DomainEvent, Filterable,
        },
        Erc3525, Erc721,
    },
    infrastructure::{
        app::configure_application,
        postgres::{event_store::get_last_dispatched_block, get_connection, PostgresModels},
        seed::{
            badge::BadgeSeeder, minter::MinterSeeder, offseter::OffseterSeeder,
            project::ProjectSeeder, vester::VesterSeeder, yielder::YielderSeeder, DataSeeder,
            Seeder,
        },
    },
};

use carbonable_indexer::filters::configure_stream_filters;

use deadpool_postgres::{Pool, Transaction};
use futures::TryStreamExt;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let configuration = configure_application().await?;
    let db_client_pool = Arc::new(get_connection(None).await?);
    let file_path = format!("./data/{}.data.json", configuration.network);

    if configuration.only_seed {
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
            Arc::new(VesterSeeder::<Erc721>::new(db_models.clone())),
            Arc::new(VesterSeeder::<Erc3525>::new(db_models_3525.clone())),
            Arc::new(YielderSeeder::<Erc721>::new(db_models)),
            Arc::new(YielderSeeder::<Erc3525>::new(db_models_3525)),
        ];

        match DataSeeder::feed_from_data(&file_path, seeders)
            .await?
            .seed()
            .await
        {
            Ok(_) => info!("Data seeded sucessfully"),
            Err(e) => error!("Data seeding failed: {:#?}", e),
        };
    }

    if configuration.only_index {
        let mut filters: [Box<dyn Filterable>; 1] = [Box::new(ProjectFilters::new())];
        let mut last_block_id = configuration.starting_block;
        if !configuration.force {
            last_block_id = configuration.starting_block;
            last_block_id = get_last_dispatched_block(&db_client_pool, &last_block_id).await?;
        }
        info!("Starting stream from block : {}", last_block_id);

        let stream_config =
            configure_stream_filters(&configuration, &file_path, &mut filters, &last_block_id)?;

        let (mut stream, configuration_handle) = ClientBuilder::<Filter, Block>::default()
            .connect(Uri::from_static("https://goerli.starknet.a5a.ch"))
            .await?;

        configuration_handle.send(stream_config.clone()).await?;

        let mut event_bus = EventBus::<Pool, Box<dyn for<'a> Consumer<Transaction<'a>>>>::new(
            db_client_pool.clone(),
        );
        event_bus.add_consumer(Box::new(ProjectTransferEventConsumer::new()));
        event_bus.add_consumer(Box::new(ProjectTransferValueEventConsumer::new()));
        event_bus.add_consumer(Box::new(ProjectSlotChangedEventConsumer::new()));

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
                            for event in block.events {
                                let mut event =
                                    DomainEvent::from_starknet_event(event, &mut filters);
                                event = event.with_metadata(&metadata.clone());
                                event_bus.dispatch(&event, &metadata).await?;
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
                    info!("Restarting stream");
                    let restarting_cfg = stream_config;
                    let cfg = restarting_cfg.with_starting_block(last_block_id);

                    configuration_handle.send(cfg.clone()).await?;

                    break;
                }
            }
        }
    }

    Ok(())
}
