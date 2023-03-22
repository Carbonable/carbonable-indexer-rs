use std::{collections::HashMap, sync::Arc};

use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use tracing::info;

use crate::infrastructure::starknet::model::{load_blockchain_data, StarknetValue};

use super::{
    get_starknet_rpc_from_env,
    model::{ModelError, StarknetModel},
};

pub struct OffseterModel {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub address: FieldElement,
}

impl OffseterModel {
    pub fn new(address: FieldElement) -> Result<Self, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for OffseterModel {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading offseter with address {:#x}", self.address);
        let res = load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "getImplementationHash",
                "getCarbonableProjectAddress",
                "getMinClaimable",
                "getTotalClaimable",
                "getTotalDeposited",
                "getTotalClaimed",
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
