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

pub struct BadgeModel {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    pub address: FieldElement,
}
impl BadgeModel {
    pub fn new(address: FieldElement) -> Result<Self, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for BadgeModel {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading badge with address {:#x}", self.address);
        Ok(load_blockchain_data(
            self.provider.clone(),
            self.address,
            &["getImplementationHash", "name", "contractURI", "owner"],
        )
        .await?)
    }
}
