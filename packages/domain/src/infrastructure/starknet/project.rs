use std::{collections::HashMap, sync::Arc};

use starknet::{
    core::types::{CallFunction, FieldElement},
    macros::felt,
    providers::{
        jsonrpc::{
            models::{BlockId, BlockTag},
            HttpTransport, JsonRpcClient,
        },
        Provider,
    },
};
use tracing::info;

use crate::{
    domain::{crypto::U256, Erc3525, Erc721},
    infrastructure::starknet::{
        model::{load_blockchain_slot_data, StarknetResolvedValue, StarknetValueResolver},
        uri::Metadata,
    },
};

use super::{
    get_starknet_provider_from_env, get_starknet_rpc_from_env,
    model::{
        felt_to_u256, get_call_function, load_blockchain_data, u256_to_felt, ModelError,
        StarknetModel, StarknetValue,
    },
    uri::UriModel,
};

pub struct ProjectModel<C = Erc721> {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    address: FieldElement,
    contract: std::marker::PhantomData<C>,
}

impl ProjectModel<Erc721> {
    pub fn new(address: FieldElement) -> Result<ProjectModel<Erc721>, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
            contract: std::marker::PhantomData::<Erc721>,
        })
    }
}

impl ProjectModel<Erc3525> {
    pub fn new(address: FieldElement) -> Result<ProjectModel<Erc3525>, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
            contract: std::marker::PhantomData::<Erc3525>,
        })
    }

    async fn load_slot_count(&self) -> Result<u64, ModelError> {
        let res = self
            .provider
            .clone()
            .call(
                get_call_function(&self.address, "slot_count", vec![]),
                &BlockId::Tag(BlockTag::Pending),
            )
            .await?;
        Ok(u64::try_from(res.first().unwrap().to_owned()).unwrap())
    }
    async fn load_slot_id_by_index(&self, index: u64) -> Result<U256, ModelError> {
        let res = self
            .provider
            .clone()
            .call(
                get_call_function(
                    &self.address,
                    "slot_by_index",
                    vec![FieldElement::from(index), FieldElement::ZERO],
                ),
                &BlockId::Tag(BlockTag::Pending),
            )
            .await?;
        Ok(felt_to_u256(*res.first().unwrap()))
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for ProjectModel<Erc721> {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading project with address {:#x}", self.address);
        let mut response_data: HashMap<String, StarknetValue> = load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "name",
                "symbol",
                "totalSupply",
                "contractURI",
                "owner",
                "getTonEquivalent",
                "getTimes",
                "getAbsorptions",
                "isSetup",
            ],
        )
        .await?;

        let uri = response_data
            .get_mut("contractURI")
            .expect("failed to get contractURI from blockchain");

        let ipfs_uri: String = uri.resolve("string_array").into();
        let uri_model = UriModel::<Erc721>::new(ipfs_uri)?;
        let metadata: Metadata = uri_model.load().await?;
        let slug = get_slug_from_uri(&metadata.external_url);

        response_data.insert(
            "slug".to_string(),
            StarknetValue::from_resolved_value(StarknetResolvedValue::String(slug)),
        );
        response_data.insert(
            "address".to_string(),
            StarknetValue::new(vec![self.address]),
        );

        Ok(response_data)
    }
}

#[async_trait::async_trait]
impl StarknetModel<Vec<HashMap<String, StarknetValue>>> for ProjectModel<Erc3525> {
    async fn load(&self) -> Result<Vec<HashMap<String, StarknetValue>>, ModelError> {
        info!("loading 3525 project with address {:#x}", self.address);
        let slots = self.load_slot_count().await?;
        let generic_data = load_blockchain_data(
            self.provider.clone(),
            self.address,
            &["owner", "symbol", "value_decimals"],
        )
        .await?;
        let mut response_data: Vec<HashMap<String, StarknetValue>> = Vec::new();
        for slot_index in 0..slots {
            let slot = self.load_slot_id_by_index(slot_index).await?;

            // name, slug from slotUri
            // contractUri = slotUri(slot)
            let mut slot_data = load_blockchain_slot_data(
                self.provider.clone(),
                self.address,
                slot,
                &[
                    "slot_uri",
                    "token_supply_in_slot",
                    "get_ton_equivalent",
                    "get_times",
                    "get_absorptions",
                    "is_setup",
                    "get_project_value",
                ],
            )
            .await?;
            let slot_uri: String = slot_data
                .get_mut("slot_uri")
                .expect("should have slot uri")
                .resolve("string_array")
                .into();
            let uri_model = UriModel::<Erc3525>::new(slot_uri)?;
            let metadata = uri_model.load().await?;
            slot_data.insert(
                "name".to_string(),
                StarknetValue::from_resolved_value(StarknetResolvedValue::String(metadata.name)),
            );
            slot_data.insert(
                "slug".to_string(),
                StarknetValue::from_resolved_value(StarknetResolvedValue::String(
                    get_slug_from_uri(&metadata.external_url),
                )),
            );
            slot_data.insert(
                "address".to_string(),
                StarknetValue::new(vec![self.address]),
            );
            slot_data.insert(
                "slot".to_string(),
                StarknetValue::from_resolved_value(StarknetResolvedValue::U256(slot)),
            );

            slot_data.extend(generic_data.clone().into_iter().map(|(k, v)| (k, v)));

            response_data.push(slot_data);
        }

        Ok(response_data)
    }
}

pub(crate) fn get_slug_from_uri(external_url: &str) -> String {
    let url = external_url.trim_end_matches('/');
    url.split('/')
        .last()
        .expect("failed to parse metadata external_url")
        .to_string()
}

pub async fn get_slot_uri_from_feeder(
    contract_address: FieldElement,
    slot: &U256,
) -> Result<String, ModelError> {
    let provider = get_starknet_provider_from_env()?;
    let res = provider
        .call_contract(
            CallFunction {
                contract_address,
                // this nasty thing means "slot_uri"
                entry_point_selector: felt!(
                    "0x1c8f7a21376b9ee15c70522ae16609dd1ee6545abfbbf2092e3acef630741a3"
                ),
                calldata: vec![u256_to_felt(slot), FieldElement::ZERO],
            },
            starknet::core::types::BlockId::Latest,
        )
        .await?;

    let slot_uri_string: String = res
        .result
        .iter()
        .skip(1)
        .map(|fe| {
            fe.to_bytes_be()
                .to_vec()
                .iter()
                .filter(|b| 0 != **b)
                .copied()
                .collect()
        })
        .map(|bytes| unsafe { String::from_utf8_unchecked(bytes) })
        .collect();
    Ok(slot_uri_string)
}
