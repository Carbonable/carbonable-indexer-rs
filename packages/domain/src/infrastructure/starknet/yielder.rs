use std::{collections::HashMap, sync::Arc};

use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use tracing::info;

use crate::{
    domain::{Contract, Erc3525, Erc721},
    infrastructure::starknet::model::load_blockchain_data,
};

use super::{
    get_starknet_rpc_from_env,
    model::{ModelError, StarknetModel, StarknetValue},
};

pub struct YielderModel<C: Contract> {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub address: FieldElement,
    contract: std::marker::PhantomData<C>,
}

impl<C> YielderModel<C>
where
    C: Contract + Send + Sync,
{
    pub fn new(address: FieldElement) -> Result<Self, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
            contract: std::marker::PhantomData::<C>,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for YielderModel<Erc721> {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading yielder with address {:#x}", self.address);
        Ok(load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "getCarbonableProjectAddress",
                "getTotalAbsorption",
                "getTotalDeposited",
                "getSnapshotedTime",
            ],
        )
        .await?)
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for YielderModel<Erc3525> {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading 3525 yielder with address {:#x}", self.address);
        Ok(load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "get_carbonable_project_address",
                "get_total_absorption",
                "get_total_deposited",
                "get_carbonable_project_slot",
            ],
        )
        .await?)
    }
}
