use starknet::core::types::FieldElement;
use std::sync::Arc;
use tracing::info;

use crate::domain::{Contract, Erc3525, Erc721};
use crate::infrastructure::postgres::entity::ErcImplementation;
use crate::infrastructure::postgres::{
    find_or_create_implementation, find_or_create_uri_3525, find_or_create_uri_721, PostgresModels,
};
use crate::infrastructure::starknet::model::{StarknetModel, StarknetValueResolver};
use crate::infrastructure::starknet::project::ProjectModel;

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct ProjectSeeder<C: Contract = Erc721> {
    pub db_models: Arc<PostgresModels<C>>,
    pub contract_type: std::marker::PhantomData<C>,
}

impl<C> ProjectSeeder<C>
where
    C: Contract,
{
    pub fn new(db_models: Arc<PostgresModels<C>>) -> ProjectSeeder<C> {
        ProjectSeeder {
            db_models,
            contract_type: std::marker::PhantomData::<C>,
        }
    }
}

#[async_trait::async_trait]
impl Seeder for ProjectSeeder<Erc721> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let project_model =
            ProjectModel::<Erc721>::new(FieldElement::from_hex_be(address.as_str()).unwrap())?;
        let db_models = self.db_models.clone();
        // fetch onchain project data
        let mut data = project_model.load().await?;
        let implementation_hash: String = data
            .get_mut("getImplementationHash")
            .expect("should have implementation hash")
            .resolve("address")
            .into();
        let project_uri: String = data
            .get_mut("contractURI")
            .expect("should have contract uri")
            .resolve("string_array")
            .into();
        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            project_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;
        let uri = find_or_create_uri_721(
            db_models.uri.clone(),
            address.as_str(),
            project_uri.as_str(),
        )
        .await?;

        let _saved = self
            .db_models
            .clone()
            .project
            .create(
                data,
                ErcImplementation::Erc721,
                Some(implementation.id),
                Some(uri.id),
            )
            .await?;
        info!("Properly seeded project {}", address);
        Ok(String::from("seeded"))
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "project" == seeder_type
    }
}

#[async_trait::async_trait]
impl Seeder for ProjectSeeder<Erc3525> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        info!("seeding Erc3525 project {}", address);
        let project_model =
            ProjectModel::<Erc3525>::new(FieldElement::from_hex_be(address.as_str()).unwrap())?;
        let db_models = self.db_models.clone();

        // fetch onchain project data
        let mut data = project_model.load().await?;
        // ERC-3525 has many slots that represent founded projects
        for slot in data.iter_mut() {
            let provider = project_model.provider.clone();
            let implementation_hash: String = slot
                .get_mut("getImplementationHash")
                .expect("should have implementation hash")
                .resolve("address")
                .into();
            let project_uri: String = slot
                .get_mut("slotURI")
                .expect("should have contract uri")
                .resolve("string_array")
                .into();
            let implementation = find_or_create_implementation(
                db_models.implementation.clone(),
                provider,
                address.as_str(),
                implementation_hash.as_str(),
            )
            .await?;
            let uri = find_or_create_uri_3525(
                db_models.uri.clone(),
                address.as_str(),
                project_uri.as_str(),
            )
            .await?;

            let _saved = self
                .db_models
                .clone()
                .project
                .create(
                    slot.clone(),
                    ErcImplementation::Erc3525,
                    Some(implementation.id),
                    Some(uri.id),
                )
                .await?;
        }
        info!("Properly seeded project {}", address);
        Ok(String::from("seeded"))
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "project_3525" == seeder_type
    }
}
