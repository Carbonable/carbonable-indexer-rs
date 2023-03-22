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
use std::{collections::HashMap, sync::Arc};

pub struct PaymentModel {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub address: FieldElement,
}

impl PaymentModel {
    pub fn new(address: FieldElement) -> Result<Self, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for PaymentModel {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading payment with address {:#x}", self.address);
        let res = load_blockchain_data(
            self.provider.clone(),
            self.address,
            &["name", "decimals", "symbol"],
        )
        .await?;

        let response_data: HashMap<String, StarknetValue> = res
            .iter()
            .map(|res| (res.0.clone(), res.1.clone()))
            .collect();

        Ok(response_data)
    }
}
