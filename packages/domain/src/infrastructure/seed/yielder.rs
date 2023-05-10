use starknet::core::types::FieldElement;

use crate::{
    domain::{Contract, Erc3525, Erc721},
    infrastructure::{
        postgres::{
            find_or_create_3525_project, find_or_create_implementation, find_or_create_project,
            PostgresModels,
        },
        starknet::{
            model::{StarknetModel, StarknetValueResolver},
            yielder::YielderModel,
        },
    },
};
use std::sync::Arc;

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct YielderSeeder<C: Contract> {
    pub db_models: Arc<PostgresModels<C>>,
    contract: std::marker::PhantomData<C>,
}

impl<C> YielderSeeder<C>
where
    C: Contract + Send + Sync,
{
    pub fn new(db_models: Arc<PostgresModels<C>>) -> Self {
        Self {
            db_models,
            contract: std::marker::PhantomData::<C>,
        }
    }
}

#[async_trait::async_trait]
impl Seeder for YielderSeeder<Erc721> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let yielder_model =
            YielderModel::<Erc721>::new(FieldElement::from_hex_be(&address).unwrap())?;
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

        let project = find_or_create_project(db_models.clone(), &project_address).await?;
        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            yielder_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;

        let _yielder = db_models
            .yielder
            .create(&address, data, Some(project.id), Some(implementation.id))
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "yielder" == seeder_type
    }
}

#[async_trait::async_trait]
impl Seeder for YielderSeeder<Erc3525> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let yielder_model =
            YielderModel::<Erc3525>::new(FieldElement::from_hex_be(&address).unwrap())?;
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
        let slot: u64 = data
            .get_mut("getCarbonableProjectSlot")
            .expect("should have getCarbonableProjectSlot")
            .resolve("u64")
            .into();

        let project =
            find_or_create_3525_project(db_models.clone(), &project_address, &slot).await?;
        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            yielder_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;

        let _yielder = db_models
            .yielder
            .create(&address, data, Some(project.id), Some(implementation.id))
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "yielder_3525" == seeder_type
    }
}
