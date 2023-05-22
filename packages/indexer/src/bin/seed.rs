use std::sync::Arc;
use tracing::{error, info};

use carbonable_domain::{
    domain::{Erc3525, Erc721},
    infrastructure::{
        app::configure_application,
        postgres::{get_connection, PostgresModels},
        seed::{
            badge::BadgeSeeder, minter::MinterSeeder, offseter::OffseterSeeder,
            project::ProjectSeeder, yielder::YielderSeeder, DataSeeder, Seeder,
        },
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let configuration = configure_application().await?;
    let db_client_pool = Arc::new(get_connection(None).await?);
    let file_path = format!("./data/{}.data.json", configuration.network);

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
        Ok(_) => info!("Data seeded sucessfully"),
        Err(e) => error!("Data seeding failed: {:#?}", e),
    };
    Ok(())
}
