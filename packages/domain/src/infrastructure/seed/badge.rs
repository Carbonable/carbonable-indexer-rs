use starknet::core::types::FieldElement;

use crate::infrastructure::{
    postgres::{find_or_create_implementation, find_or_create_uri, PostgresModels},
    starknet::{
        badge::BadgeModel,
        model::{StarknetModel, StarknetValueResolver},
    },
};

use super::{DataSeederError, Seeder};
use std::sync::Arc;

#[derive(Debug)]
pub struct BadgeSeeder {
    pub db_models: Arc<PostgresModels>,
}

#[async_trait::async_trait]
impl Seeder for BadgeSeeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();
        let badge_model = BadgeModel::new(FieldElement::from_hex_be(&address).unwrap())?;
        let mut data = badge_model.load().await?;

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
            badge_model.provider,
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

        let _badge = db_models
            .badge
            .create(&address, data, Some(implementation.id), Some(uri.id))
            .await?;
        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        seeder_type == "badge"
    }
}
