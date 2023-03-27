pub mod badge;
pub mod minter;
pub mod model;
pub mod offseter;
pub mod payment;
pub mod project;
pub mod uri;
pub mod vester;
pub mod yielder;

use starknet::{
    core::types::FieldElement,
    providers::{
        jsonrpc::{
            models::{BlockId, BlockTag},
            HttpTransport, JsonRpcClient,
        },
        SequencerGatewayProvider,
    },
};
use std::sync::Arc;
use thiserror::Error;
use url::Url;

use self::model::ModelError;

#[derive(Error, Debug)]
pub enum SequencerError {
    #[error("environment variable 'NETWORK' not provided")]
    NoEnvProvided,
    #[error("environment variable 'SEQUENCER_DOMAIN' not provided")]
    NoSequencerDomainProvided,
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
}

pub enum StarknetEnv {
    Mainnet,
    Goerli,
    Goerli2,
    Local,
}

impl From<String> for StarknetEnv {
    fn from(env: String) -> Self {
        match env.as_str() {
            "mainnet" => Self::Mainnet,
            "goerli" => Self::Goerli,
            "goerli2" => Self::Goerli2,
            "local" => Self::Local,
            _ => panic!("Invalid environment"),
        }
    }
}

/// Get starknet provider base on "NETWORK" environment variable
/// get_starknet_provider_from_env();
pub fn get_starknet_provider_from_env() -> Result<SequencerGatewayProvider, SequencerError> {
    if let Ok(env) = std::env::var("NETWORK") {
        return get_starknet_provider(env.into());
    }
    Err(SequencerError::NoEnvProvided)
}

/// Get starknet rpc client base on param given "NETWORK" and "SEQUENCER_DOMAIN"
/// get_starknet_rpc_from_env();
pub fn get_starknet_rpc_from_env() -> Result<JsonRpcClient<HttpTransport>, SequencerError> {
    if let Ok(env) = std::env::var("NETWORK") {
        return get_starknet_rpc_client(env.into());
    }
    Err(SequencerError::NoEnvProvided)
}

/// Get starknet provider base on param given:
/// get_starknet_provider(StarknetEnv::Mainnet);
pub fn get_starknet_provider(env: StarknetEnv) -> Result<SequencerGatewayProvider, SequencerError> {
    Ok(match env {
        StarknetEnv::Mainnet => SequencerGatewayProvider::starknet_alpha_mainnet(),
        StarknetEnv::Goerli => SequencerGatewayProvider::starknet_alpha_goerli(),
        StarknetEnv::Goerli2 => SequencerGatewayProvider::starknet_alpha_goerli_2(),
        StarknetEnv::Local => SequencerGatewayProvider::starknet_nile_localhost(),
    })
}

fn get_starknet_rpc_client(
    env: StarknetEnv,
) -> Result<JsonRpcClient<HttpTransport>, SequencerError> {
    let sequencer_domain = get_sequencer_domain(&env)?;
    Ok(JsonRpcClient::new(HttpTransport::new(Url::parse(
        &sequencer_domain,
    )?)))
}

fn get_sequencer_domain(env: &StarknetEnv) -> Result<String, SequencerError> {
    if let Ok(domain) = std::env::var("SEQUENCER_DOMAIN") {
        let subdomain = match env {
            StarknetEnv::Mainnet => "starknet-mainnet",
            StarknetEnv::Goerli => "starknet-goerli",
            StarknetEnv::Goerli2 => "starknet-goerli2",
            StarknetEnv::Local => "http://localhost:3000",
        };

        return Ok(domain.replace("DOMAIN", subdomain));
    }
    Err(SequencerError::NoSequencerDomainProvided)
}

/// Get proxy class abi
pub async fn get_proxy_abi(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    implementation_hash: FieldElement,
) -> Result<serde_json::Value, ModelError> {
    let res = provider
        .get_class(&BlockId::Tag(BlockTag::Latest), implementation_hash)
        .await?;
    Ok(serde_json::to_value(res.abi)?)
}
