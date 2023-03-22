use std::{collections::HashMap, sync::Arc};

use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use tracing::info;

use crate::infrastructure::starknet::model::load_blockchain_data;

use super::{
    get_starknet_rpc_from_env,
    model::{ModelError, StarknetModel, StarknetValue},
};

pub struct MinterModel {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub address: FieldElement,
}

impl MinterModel {
    pub fn new(address: FieldElement) -> Result<Self, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for MinterModel {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading minter with address {:#x}", self.address);
        let res = load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "getImplementationHash",
                "getCarbonableProjectAddress",
                "getPaymentTokenAddress",
                "isPreSaleOpen",
                "isPublicSaleOpen",
                "getMaxBuyPerTx",
                "getReservedSupplyForMint",
                "getMaxSupplyForMint",
                "getUnitPrice",
                "getWhitelistMerkleRoot",
                "isSoldOut",
                "getTotalValue",
            ],
        )
        .await?;

        let response_data: HashMap<String, StarknetValue> = res
            .iter()
            .map(|res| (res.0.clone(), res.1.clone()))
            .collect();

        Ok(response_data)
    }
}
