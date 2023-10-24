use crate::{
    domain::Erc3525,
    infrastructure::{
        postgres::{
            find_721_project_id, find_or_create_3525_project, update_721_project_migrator_address,
            PostgresModels,
        },
        starknet::{
            get_starknet_rpc_from_env,
            model::{felt_to_u256, parallelize_blockchain_rpc_calls},
        },
    },
};
use std::sync::Arc;

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct MigratorSeeder {
    pub db_models: Arc<PostgresModels<Erc3525>>,
}

impl MigratorSeeder {
    pub fn new(db_models: Arc<PostgresModels<Erc3525>>) -> Self {
        Self { db_models }
    }
}

#[async_trait::async_trait]
impl Seeder for MigratorSeeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();
        let provider = get_starknet_rpc_from_env()?;
        let calldata = [
            (address.to_owned(), "source_address", vec![]),
            (address.to_owned(), "target_address", vec![]),
            (address.to_owned(), "slot", vec![]),
        ];

        let data = parallelize_blockchain_rpc_calls(provider.into(), calldata.to_vec()).await?;

        let source_address = format!("{:#066x}", data[0][0].clone());
        let target_address = format!("{:#066x}", data[1][0].clone());
        let slot = felt_to_u256(data[2][0].clone());

        let _project =
            find_or_create_3525_project(db_models.clone(), target_address.as_str(), &slot.into())
                .await?;

        let project_721_id = match find_721_project_id(db_models.clone(), &source_address).await {
            Ok(id) => id,
            Err(_) => loop {
                match find_721_project_id(db_models.clone(), &source_address).await {
                    Ok(id) => break id,
                    Err(e) => {
                        tracing::warn!("failed to find 721 project id: {:?}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }
                }
            },
        };
        let _updated =
            update_721_project_migrator_address(db_models.clone(), &project_721_id, &address)
                .await?;

        Ok("project properly seeded from migrator address".to_owned())
    }
    fn can_process(&self, seeder_type: String) -> bool {
        "migrator" == seeder_type
    }
}
