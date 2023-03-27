use starknet::core::types::FieldElement;
use std::sync::Arc;

use crate::{
    domain::{Contract, Erc3525, Erc721},
    infrastructure::{
        postgres::{
            find_or_create_3525_project, find_or_create_implementation, find_or_create_project,
            PostgresModels,
        },
        starknet::{
            model::{StarknetModel, StarknetValueResolver},
            offseter::OffseterModel,
        },
    },
};

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct OffseterSeeder<C: Contract> {
    pub db_models: Arc<PostgresModels<C>>,
    contract: std::marker::PhantomData<C>,
}

impl<C> OffseterSeeder<C>
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
impl Seeder for OffseterSeeder<Erc721> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let offseter_model =
            OffseterModel::<Erc721>::new(FieldElement::from_hex_be(&address).unwrap())?;
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
        "offseter" == seeder_type
    }
}

#[async_trait::async_trait]
impl Seeder for OffseterSeeder<Erc3525> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();

        let offseter_model =
            OffseterModel::<Erc3525>::new(FieldElement::from_hex_be(&address).unwrap())?;
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
        let slot: u64 = data
            .get_mut("getCarbonableProjectSlot")
            .expect("should have getCarbonableProjectSlot")
            .resolve("u64")
            .into();

        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            offseter_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;
        let project =
            find_or_create_3525_project(db_models.clone(), &project_address, &slot).await?;

        let _minter = db_models
            .offseter
            .create(&address, data, Some(implementation.id), Some(project.id))
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "offseter_3525" == seeder_type
    }
}
