use starknet::core::types::FieldElement;
use std::sync::Arc;

use crate::infrastructure::{
    postgres::{find_or_create_implementation, find_or_create_project, PostgresModels},
    starknet::{
        model::{StarknetModel, StarknetValueResolver},
        offseter::OffseterModel,
    },
};

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct OffseterSeeder {
    pub db_models: Arc<PostgresModels>,
}

impl OffseterSeeder {
    pub fn new(db_models: Arc<PostgresModels>) -> Self {
        Self { db_models }
    }
}

#[async_trait::async_trait]
impl Seeder for OffseterSeeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let offseter_model = OffseterModel::new(FieldElement::from_hex_be(&address).unwrap())?;
        let mut data = offseter_model.load().await?;

        let implementation_hash: String = data
            .get_mut("getImplementationHash")
            .expect("should have implementation hash")
            .resolve("address")
            .into();
        let project_address: String = data
            .get_mut("getCarbonableProjectAddress")
            .expect("should have carbonableProjectAddress")
            .resolve("address")
            .into();

        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            offseter_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;
        let project = find_or_create_project(db_models.clone(), &project_address).await?;

        let _minter = db_models
            .offseter
            .create(&address, data, Some(implementation.id), Some(project.id))
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        seeder_type == "offseter"
    }
}
