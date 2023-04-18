use std::sync::Arc;

#[cfg(test)]
mod test_utils;

use apibara_core::starknet::v1alpha2::{Block, Filter, HeaderFilter};
use apibara_sdk::{ClientBuilder, Configuration, Uri};
use carbonable_domain::{
    domain::{
        event_source::{
            project::{ProjectTransferEventConsumer, ProjectTransferValueEventConsumer},
            EventBus,
        },
        Erc3525, Erc721,
    },
    infrastructure::{
        app::configure_application,
        postgres::{get_connection, PostgresModels},
        seed::{
            badge::BadgeSeeder, minter::MinterSeeder, offseter::OffseterSeeder,
            project::ProjectSeeder, vester::VesterSeeder, yielder::YielderSeeder, DataSeeder,
            Seeder,
        },
    },
};

use carbonable_indexer::filters::configure_stream_filters;

use futures::TryStreamExt;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let configuration = configure_application().await?;
    let db_client_pool = Arc::new(get_connection(None).await?);
    let file_path = format!("./data/{}.data.json", configuration.network);
    let db_models = Arc::new(PostgresModels::<Erc721>::new(db_client_pool.clone()));
    let db_models_3525 = Arc::new(PostgresModels::<Erc3525>::new(db_client_pool.clone()));

    // let seeders: Vec<Arc<dyn Seeder + Send + Sync>> = vec![
    //     Arc::new(ProjectSeeder::<Erc721>::new(db_models.clone())),
    //     Arc::new(ProjectSeeder::<Erc3525>::new(db_models_3525.clone())),
    //     Arc::new(BadgeSeeder::<Erc721>::new(db_models.clone())),
    //     Arc::new(BadgeSeeder::<Erc3525>::new(db_models_3525.clone())),
    //     Arc::new(MinterSeeder::<Erc721>::new(db_models.clone())),
    //     Arc::new(MinterSeeder::<Erc3525>::new(db_models_3525.clone())),
    //     Arc::new(OffseterSeeder::<Erc721>::new(db_models.clone())),
    //     Arc::new(OffseterSeeder::<Erc3525>::new(db_models_3525.clone())),
    //     Arc::new(VesterSeeder::<Erc721>::new(db_models.clone())),
    //     Arc::new(VesterSeeder::<Erc3525>::new(db_models_3525.clone())),
    //     Arc::new(YielderSeeder::<Erc721>::new(db_models)),
    //     Arc::new(YielderSeeder::<Erc3525>::new(db_models_3525)),
    // ];
    //
    // match DataSeeder::feed_from_data(&file_path, seeders)
    //     .await?
    //     .seed()
    //     .await
    // {
    //     Ok(_) => info!("Data seeded sucessfully"),
    //     Err(e) => error!("Data seeding failed: {:#?}", e),
    // };

    let stream_config = configure_stream_filters(&configuration, &file_path)?;
    println!("{:#?}", stream_config);

    let (mut stream, configuration_handle) = ClientBuilder::<Filter, Block>::default()
        .connect(Uri::from_static("https://goerli.starknet.a5a.ch"))
        .await?;

    configuration_handle.send(stream_config.clone()).await?;

    let mut event_bus = EventBus::new(db_client_pool.clone());
    event_bus.add_consumer(Box::new(ProjectTransferEventConsumer::new()));
    event_bus.add_consumer(Box::new(ProjectTransferValueEventConsumer::new()));

    while let Some(response) = stream.try_next().await? {
        match response {
            apibara_sdk::DataMessage::Data {
                cursor,
                end_cursor,
                finality,
                batch,
            } => {
                info!(
                    "Handling data within {} and {}",
                    cursor.expect("should have starting cursor").order_key,
                    end_cursor.order_key
                );
                println!("{:#?}", finality);
                println!("{:#?}", batch);

                for block in batch {
                    println!("{:#?}", block);
                }
            }
            apibara_sdk::DataMessage::Invalidate { cursor } => match cursor {
                Some(c) => error!("Received an invalidate request data at {}", &c.order_key),
                None => error!("Invalidate request without cursor provided"),
            },
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{self, SystemTime};

    use apibara_core::starknet::v1alpha2::{Block, Event, EventWithTransaction, FieldElement};
    use starknet::macros::selector;

    use crate::test_utils::BlockBuilder;

    #[test]
    fn test_indexer_works() {
        let block_mock = BlockBuilder::init()
            .with_events(vec![EventWithTransaction {
                transaction: None,
                receipt: None,
                event: Some(Event {
                    from_address: None,
                    keys: vec![FieldElement::from_bytes(
                        &selector!("Transfer").to_bytes_be(),
                    )],
                    data: vec![
                        FieldElement::from_u64(0),
                        FieldElement::from_hex(
                            "0x63675fa1ecea10063722e61557ed7f49ed2503d6cdd74f4b31e9770b473650c",
                        )
                        .unwrap(),
                        FieldElement::from_u64(1),
                        FieldElement::from_u64(0),
                    ],
                }),
            }])
            .build();

        println!("{:#?}", block_mock);
        assert_eq!(true, false);
    }

    fn build_block() -> Block {
        Block {
            status: 1,
            header: Some(apibara_core::starknet::v1alpha2::BlockHeader {
                block_hash: None,
                parent_block_hash: None,
                block_number: 1,
                sequencer_address: None,
                new_root: None,
                timestamp: None,
            }),
            transactions: vec![],
            state_update: None,
            events: vec![],
            l2_to_l1_messages: vec![],
        }
    }
}
