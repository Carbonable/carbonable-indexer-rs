use std::sync::Arc;

use starknet::core::types::FieldElement;

use crate::infrastructure::{
    postgres::{find_or_create_implementation, PostgresModels},
    starknet::{
        model::{StarknetModel, StarknetValueResolver},
        vester::VesterModel,
    },
};

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct VesterSeeder {
    pub db_models: Arc<PostgresModels>,
}

#[async_trait::async_trait]
impl Seeder for VesterSeeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let vester_model = VesterModel::new(FieldElement::from_hex_be(&address).unwrap())?;
        let mut data = vester_model.load().await?;

        let implementation_hash: String = data
            .get_mut("getImplementationHash")
            .expect("should have implementation hash")
            .resolve("address")
            .into();

        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            vester_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;

        let _minter = db_models
            .vester
            .create(&address, data, Some(implementation.id))
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        seeder_type == "vester"
    }
}
