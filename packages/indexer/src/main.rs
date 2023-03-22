use std::sync::Arc;

use carbonable_domain::infrastructure::{
    app::configure_application,
    postgres::{get_connection, PostgresModels},
    seed::{
        badge::BadgeSeeder, minter::MinterSeeder, offseter::OffseterSeeder, project::ProjectSeeder,
        vester::VesterSeeder, yielder::YielderSeeder, DataSeeder, Seeder,
    },
};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let configuration = configure_application().await?;
    let db_client_pool = Arc::new(get_connection(None).await?);
    let file_path = format!("./data/{}.data.json", configuration.network);
    let db_models = Arc::new(PostgresModels::new(db_client_pool));
    let seeders: Vec<Arc<dyn Seeder + Send + Sync>> = vec![
        Arc::new(ProjectSeeder {
            db_models: db_models.clone(),
        }),
        Arc::new(BadgeSeeder {
            db_models: db_models.clone(),
        }),
        Arc::new(MinterSeeder {
            db_models: db_models.clone(),
        }),
        Arc::new(OffseterSeeder {
            db_models: db_models.clone(),
        }),
        Arc::new(VesterSeeder {
            db_models: db_models.clone(),
        }),
        Arc::new(YielderSeeder {
            db_models: db_models.clone(),
        }),
    ];

    match DataSeeder::feed_from_data(file_path, seeders)
        .await?
        .seed()
        .await
    {
        Ok(_) => info!("Data seeded sucessfully"),
        Err(e) => error!("Data seeding failed: {}", e),
    };

    // let (mut stream, configuration_handle) = ClientBuilder::<Filter, Block>::default()
    //     .build(Some("https://goerli.starknet.a5a.ch"))
    //     .await?;
    //
    // let mut config = Configuration::<Filter>::default();
    // configuration_handle
    //     .send(
    //         config
    //             .starting_at_block(1)
    //             .with_batch_size(10)
    //             .with_filter(|filter| filter.with_header(HeaderFilter::new()))
    //             .clone(),
    //     )
    //     .await?;
    //
    // while let Some(response) = stream.next().await {
    //     println!("Response: {:?}", response);
    // }

    Ok(())
}
