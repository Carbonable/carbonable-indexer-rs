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

pub struct MinterModel<C: Contract> {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub address: FieldElement,
    contract: std::marker::PhantomData<C>,
}

impl<C> MinterModel<C>
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
impl StarknetModel<HashMap<String, StarknetValue>> for MinterModel<Erc721> {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading minter with address {:#x}", self.address);
        Ok(load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
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
        .await?)
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for MinterModel<Erc3525> {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading 3525 minter with address {:#x}", self.address);
        Ok(load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "get_carbonable_project_address",
                "get_payment_token_address",
                "is_pre_sale_open",
                "is_public_sale_open",
                "get_max_value_per_tx",
                "get_min_value_per_tx",
                "get_reserved_value",
                "get_unit_price",
                "get_max_value",
                "get_whitelist_merkle_root",
                "get_carbonable_project_slot",
                "is_sold_out",
            ],
        )
        .await?)
    }
}
