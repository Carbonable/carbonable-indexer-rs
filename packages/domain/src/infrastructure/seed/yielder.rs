use starknet::core::types::FieldElement;

use crate::infrastructure::{
    postgres::{
        find_or_create_implementation, find_or_create_project, find_or_create_vester,
        PostgresModels,
    },
    starknet::{
        model::{StarknetModel, StarknetValueResolver},
        yielder::YielderModel,
    },
};
use std::sync::Arc;

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct YielderSeeder {
    pub db_models: Arc<PostgresModels>,
}

#[async_trait::async_trait]
impl Seeder for YielderSeeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let yielder_model = YielderModel::new(FieldElement::from_hex_be(&address).unwrap())?;
        let mut data = yielder_model.load().await?;

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
        let vester_address: String = data
            .get_mut("getCarbonableVesterAddress")
            .expect("should have carbonableVesterAddress")
            .resolve("address")
            .into();

        let project = find_or_create_project(db_models.clone(), &project_address).await?;
        let vester = find_or_create_vester(db_models.clone(), &vester_address).await?;
        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            yielder_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;

        let _yielder = db_models
            .yielder
            .create(
                &address,
                data,
                Some(project.id),
                Some(vester.id),
                Some(implementation.id),
            )
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        seeder_type == "yielder"
    }
}
