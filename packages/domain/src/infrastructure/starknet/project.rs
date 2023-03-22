use std::{collections::HashMap, sync::Arc};

use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use tracing::info;

use crate::infrastructure::starknet::model::{StarknetResolvedValue, StarknetValueResolver};

use super::{
    get_starknet_rpc_from_env,
    model::{load_blockchain_data, ModelError, StarknetModel, StarknetValue},
    uri::UriModel,
};

pub struct ProjectModel {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
    address: FieldElement,
}

impl ProjectModel {
    pub fn new(address: FieldElement) -> Result<Self, ModelError> {
        Ok(Self {
            provider: Arc::new(get_starknet_rpc_from_env()?),
            address,
        })
    }
}

#[async_trait::async_trait]
impl StarknetModel<HashMap<String, StarknetValue>> for ProjectModel {
    async fn load(&self) -> Result<HashMap<String, StarknetValue>, ModelError> {
        info!("loading project with address {:#x}", self.address);
        let res = load_blockchain_data(
            self.provider.clone(),
            self.address,
            &[
                "getImplementationHash",
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

        let mut response_data: HashMap<String, StarknetValue> =
            res.iter().fold(HashMap::new(), |mut acc, res| {
                acc.insert(res.0.clone(), res.1.clone());
                acc
            });

        let uri = response_data
            .get_mut("contractURI")
            .expect("failed to get contractURI from blockchain");

        let ipfs_uri: String = uri.resolve("string_array").into();
        let uri_model = UriModel::new(ipfs_uri)?;
        let metadata = uri_model.load().await?;
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

fn get_slug_from_uri(external_url: &str) -> String {
    let url = external_url.trim_end_matches('/');
    url.split('/')
        .last()
        .expect("failed to parse metadata external_url")
        .to_string()
}

#[cfg(test)]
mod tests {
    use starknet::core::types::FieldElement;

    use crate::infrastructure::starknet::project::ProjectModel;

    #[tokio::test]
    async fn test_get_abi() {
        let model = ProjectModel::new(
            FieldElement::from_hex_be(
                "0x003d062b797ca97c2302bfdd0e9b687548771eda981d417faace4f6913ed8f2a",
            )
            .unwrap(),
        )
        .unwrap();

        let _abi = model
            .get_project_proxy_abi(
                FieldElement::from_hex_be(
                    "0x2ae72e57d8b5f77bb6fc8183018709f236b32e4b27ddbdfecde229da175815d",
                )
                .unwrap(),
            )
            .await
            .unwrap();
    }
}
