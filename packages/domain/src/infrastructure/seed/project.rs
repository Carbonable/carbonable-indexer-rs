use starknet::core::types::FieldElement;
use std::sync::Arc;
use tracing::info;

use crate::infrastructure::postgres::{
    find_or_create_implementation, find_or_create_uri, PostgresModels,
};
use crate::infrastructure::starknet::model::{StarknetModel, StarknetValueResolver};
use crate::infrastructure::starknet::project::ProjectModel;

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct ProjectSeeder {
    pub db_models: Arc<PostgresModels>,
}

#[async_trait::async_trait]
impl Seeder for ProjectSeeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let project_model =
            ProjectModel::new(FieldElement::from_hex_be(address.as_str()).unwrap())?;
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
        let uri = find_or_create_uri(
            db_models.uri.clone(),
            address.as_str(),
            project_uri.as_str(),
        )
        .await?;

        let _saved = self
            .db_models
            .clone()
            .project
            .create(data, Some(implementation.id), Some(uri.id))
            .await?;
        info!("Properly seeded project {}", address);
        Ok(String::from("seeded"))
    }

    fn can_process(&self, seeder_type: String) -> bool {
        seeder_type == "project"
    }
}
