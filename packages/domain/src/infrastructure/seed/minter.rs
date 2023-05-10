use std::sync::Arc;

use starknet::core::types::FieldElement;

use crate::domain::{Contract, Erc3525, Erc721};
use crate::infrastructure::postgres::{
    find_or_create_3525_project, find_or_create_payment, find_or_create_project,
};
use crate::infrastructure::starknet::model::{StarknetModel, StarknetValueResolver};
use crate::infrastructure::{
    postgres::{find_or_create_implementation, PostgresModels},
    starknet::minter::MinterModel,
};

use super::{DataSeederError, Seeder};

#[derive(Debug)]
pub struct MinterSeeder<C: Contract> {
    pub db_models: Arc<PostgresModels<C>>,
    contract: std::marker::PhantomData<C>,
}

impl MinterSeeder<Erc721> {
    pub fn new(db_models: Arc<PostgresModels<Erc721>>) -> Self {
        Self {
            db_models,
            contract: std::marker::PhantomData::<Erc721>,
        }
    }
}
impl MinterSeeder<Erc3525> {
    pub fn new(db_models: Arc<PostgresModels<Erc3525>>) -> Self {
        Self {
            db_models,
            contract: std::marker::PhantomData::<Erc3525>,
        }
    }
}

#[async_trait::async_trait]
impl Seeder for MinterSeeder<Erc721> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();
        let minter_model =
            MinterModel::<Erc721>::new(FieldElement::from_hex_be(&address).unwrap())?;
        let mut data = minter_model.load().await?;

        let implementation_hash: String = data
            .get_mut("getImplementationHash")
            .expect("should have implementation hash")
            .resolve("address")
            .into();
        let project_address: String = data
            .get_mut("getCarbonableProjectAddress")
            .expect("should have getCarbonableProjectAddress")
            .resolve("address")
            .into();
        let payment_address: String = data
            .get_mut("getPaymentTokenAddress")
            .expect("should have getPaymentTokenAddress")
            .resolve("address")
            .into();

        let project = find_or_create_project(db_models.clone(), project_address.as_str()).await?;
        let payment = find_or_create_payment(db_models.clone(), payment_address.as_str()).await?;
        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            minter_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;

        let _minter = db_models
            .minter
            .create(
                &address,
                data,
                Some(project.id),
                Some(payment.id),
                Some(implementation.id),
            )
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "minter" == seeder_type
    }
}

#[async_trait::async_trait]
impl Seeder for MinterSeeder<Erc3525> {
    async fn seed(&self, address: String) -> Result<String, DataSeederError> {
        let db_models = self.db_models.clone();
        let minter_model =
            MinterModel::<Erc3525>::new(FieldElement::from_hex_be(&address).unwrap())?;
        let mut data = minter_model.load().await?;

        let implementation_hash: String = data
            .get_mut("getImplementationHash")
            .expect("should have implementation hash")
            .resolve("address")
            .into();
        let project_address: String = data
            .get_mut("getCarbonableProjectAddress")
            .expect("should have getCarbonableProjectAddress")
            .resolve("address")
            .into();
        let payment_address: String = data
            .get_mut("getPaymentTokenAddress")
            .expect("should have getPaymentTokenAddress")
            .resolve("address")
            .into();
        let slot: u64 = data
            .get_mut("getCarbonableProjectSlot")
            .expect("should have getCarbonableProjectSlot")
            .resolve("u64")
            .into();

        let project =
            find_or_create_3525_project(db_models.clone(), project_address.as_str(), &slot).await?;
        let payment = find_or_create_payment(db_models.clone(), payment_address.as_str()).await?;
        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            minter_model.provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;

        let _minter = db_models
            .minter
            .create(
                &address,
                data,
                Some(project.id),
                Some(payment.id),
                Some(implementation.id),
            )
            .await?;

        Ok(address)
    }

    fn can_process(&self, seeder_type: String) -> bool {
        "minter_3525" == seeder_type
    }
}
