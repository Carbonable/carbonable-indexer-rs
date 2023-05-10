use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::domain::crypto::U256;
use crate::domain::{Contract, Erc3525, Erc721};
use crate::infrastructure::postgres::entity::ErcImplementation;
use crate::infrastructure::postgres::{
    find_or_create_implementation, find_or_create_uri_3525, find_or_create_uri_721, PostgresModels,
};
use crate::infrastructure::starknet::model::{
    parallelize_blockchain_rpc_calls, ModelError, StarknetModel, StarknetResolvedValue,
    StarknetValue, StarknetValueResolver,
};
use crate::infrastructure::starknet::project::{get_slug_from_uri, ProjectModel};
use crate::infrastructure::starknet::uri::UriModel;

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
        // seeding is done with minter.
        // but field is still required because based on the same file
        "DONTSEEDWITHTHESEDATA." == seeder_type
    }
}

impl ProjectSeeder<Erc3525> {
    pub async fn seed_from_slot(
        &self,
        address: String,
        slot: &u64,
    ) -> Result<String, DataSeederError> {
        info!("seeding Erc3525 project {address} with slot: {slot}");
        let project_model =
            ProjectModel::<Erc3525>::new(FieldElement::from_hex_be(address.as_str()).unwrap())?;
        let db_models = self.db_models.clone();
        let slot_felt: FieldElement = <u64 as Into<FieldElement>>::into(*slot);

        let provider = project_model.provider.clone();
        let mut slot_data =
            map_multicall_to_hashmap(provider.clone(), &address, slot, slot_felt).await?;
        let implementation_hash: String = slot_data
            .get_mut("getImplementationHash")
            .expect("should have implementation hash")
            .resolve("address")
            .into();
        let slot_uri: String = slot_data
            .get_mut("slotURI")
            .expect("should have slot uri")
            .resolve("string_array")
            .into();

        let implementation = find_or_create_implementation(
            db_models.implementation.clone(),
            provider,
            address.as_str(),
            implementation_hash.as_str(),
        )
        .await?;
        let uri =
            find_or_create_uri_3525(db_models.uri.clone(), address.as_str(), slot_uri.as_str())
                .await?;

        self.db_models
            .clone()
            .project
            .create(
                slot_data,
                ErcImplementation::Erc3525,
                Some(implementation.id),
                Some(uri.id),
            )
            .await?;

        info!("Properly seeded project {}", address);
        Ok(String::from("seeded"))
    }
}

async fn map_multicall_to_hashmap(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: &str,
    slot: &u64,
    slot_felt: FieldElement,
) -> Result<HashMap<String, StarknetValue>, ModelError> {
    let calldata = [
        (
            address.to_string(),
            "slotURI",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (
            address.to_string(),
            "tokenSupplyInSlot",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (
            address.to_string(),
            "getTonEquivalent",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (
            address.to_string(),
            "getTimes",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (
            address.to_string(),
            "getAbsorptions",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (
            address.to_string(),
            "isSetup",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (address.to_string(), "getImplementationHash", vec![]),
        (address.to_string(), "owner", vec![]),
        (address.to_string(), "symbol", vec![]),
        (address.to_string(), "valueDecimals", vec![]),
    ];

    let data = parallelize_blockchain_rpc_calls(provider.clone(), calldata.to_vec()).await?;
    let mut slot_data = HashMap::new();

    let mut slot_uri = StarknetValue::new(data[0].clone());
    slot_data.insert("slotURI".to_string(), slot_uri.clone());
    slot_data.insert(
        "tokenSupplyInSlot".to_string(),
        StarknetValue::new(data[1].clone()),
    );
    slot_data.insert(
        "getTonEquivalent".to_string(),
        StarknetValue::new(data[2].clone()),
    );
    slot_data.insert("getTimes".to_string(), StarknetValue::new(data[3].clone()));
    slot_data.insert(
        "getAbsorptions".to_string(),
        StarknetValue::new(data[4].clone()),
    );
    slot_data.insert("isSetup".to_string(), StarknetValue::new(data[5].clone()));
    slot_data.insert(
        "getImplementationHash".to_string(),
        StarknetValue::new(data[6].clone()),
    );
    slot_data.insert("owner".to_string(), StarknetValue::new(data[7].clone()));
    slot_data.insert("symbol".to_string(), StarknetValue::new(data[8].clone()));
    slot_data.insert(
        "valueDecimals".to_string(),
        StarknetValue::new(data[9].clone()),
    );

    let uri: String = slot_uri.resolve("string_array").into();
    let uri_model = UriModel::<Erc3525>::new(uri)?;
    let metadata = uri_model.load().await?;

    slot_data.insert(
        "name".to_string(),
        StarknetValue::from_resolved_value(StarknetResolvedValue::String(metadata.name)),
    );
    slot_data.insert(
        "slug".to_string(),
        StarknetValue::from_resolved_value(StarknetResolvedValue::String(get_slug_from_uri(
            &metadata.external_url,
        ))),
    );
    slot_data.insert(
        "address".to_string(),
        StarknetValue::from_resolved_value(StarknetResolvedValue::String(address.to_string())),
    );
    slot_data.insert(
        "slot".to_string(),
        StarknetValue::from_resolved_value(StarknetResolvedValue::U256(U256(
            crypto_bigint::U256::from_u64(*slot),
        ))),
    );

    Ok(slot_data)
}
