use std::sync::Arc;

use starknet::{
    core::{
        types::{BlockId, CallFunction, FieldElement},
        utils::{get_selector_from_name, NonAsciiNameError},
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, ProviderError, SequencerGatewayProvider, SequencerGatewayProviderError,
    },
};
use thiserror::Error;
use tokio::task::JoinError;
use tracing::info;

use crate::infrastructure::flatten;

use super::{get_starknet_rpc_client, get_starknet_rpc_from_env, SequencerError};

const MAX_RETRY_COUNT: u8 = 5;
const RETRY_DELAY: u64 = 10;

#[derive(Debug, Error)]
pub enum ModelError<T: Provider> {
    #[error(transparent)]
    FailedToBuildModel(#[from] SequencerError),
    #[error("failed to parse out contract selector")]
    FailedToParseSelector(#[from] NonAsciiNameError),
    #[error("failed to parse env var {0}")]
    FailedToParseEnvVar(#[from] std::env::VarError),
    #[error("starknet rpc rate limited")]
    RateLimited,
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SequencerError(#[from] ProviderError<SequencerGatewayProviderError>),
    #[error(transparent)]
    ProviderError(#[from] ProviderError<<T as starknet::providers::Provider>::Error>),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    JoinError(#[from] JoinError),
}

#[async_trait::async_trait]
pub trait StarknetModel<T> {
    async fn load(&self) -> Result<T, ModelError>;
}

fn get_call_function(address: &FieldElement, selector: &str) -> CallFunction {
    CallFunction {
        contract_address: *address,
        entry_point_selector: get_selector_from_name(selector).unwrap(),
        calldata: vec![],
    }
}

/// Sync starknet model with some base data
pub(crate) async fn load_blockchain_data<T: Provider + Send + Sync + 'static>(
    provider: Arc<T>,
    address: FieldElement,
    selectors: &'static [&str],
) -> Result<Vec<(String, StarknetValue)>, ModelError> {
    let mut handles = vec![];
    for selector in selectors {
        let provider = provider.clone();

        let handle = tokio::spawn(async move {
            let contract_entrypoint = selector;
            let mut count = 0;
            let mut res = provider
                .call_contract(
                    get_call_function(&address, contract_entrypoint),
                    BlockId::Latest,
                )
                .await;
            while let Err(ProviderError::RateLimited) = res {
                if count >= MAX_RETRY_COUNT {
                    return Err(ModelError::RateLimited);
                }
                info!("retrying to call starknet RPC");
                tokio::time::sleep(std::time::Duration::from_secs(RETRY_DELAY)).await;
                res = provider
                    .call_contract(
                        get_call_function(&address, contract_entrypoint),
                        BlockId::Latest,
                    )
                    .await;
                count += 1;
            }
            Ok((
                selector.to_string(),
                StarknetValue::new(res.unwrap().result),
            ))
        });

        handles.push(flatten(handle));
    }

    match futures::future::try_join_all(handles).await {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}

pub trait StarknetValueResolver {
    fn resolve(&mut self, required_type: &str) -> StarknetResolvedValue;
}

/// Represents starknet inner FieldElement human comprehensible values
#[derive(Clone, Debug)]
pub struct StarknetValue {
    inner: Vec<FieldElement>,
    resolved: Option<StarknetResolvedValue>,
}

impl StarknetValue {
    pub fn new(inner: Vec<FieldElement>) -> Self {
        Self {
            inner,
            resolved: None,
        }
    }

    pub fn from_resolved_value(resolved: StarknetResolvedValue) -> Self {
        Self {
            inner: vec![],
            resolved: Some(resolved),
        }
    }
}

impl StarknetValueResolver for StarknetValue {
    fn resolve(&mut self, required_type: &str) -> StarknetResolvedValue {
        if let Some(resolved) = &self.resolved {
            return resolved.clone();
        }
        return match required_type {
            // Starknet strings are represented as [char]
            // first FieldElement is the length of the array
            // other are byte of subsequent string
            "string" => {
                let string: String = self
                    .inner
                    .iter()
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
                let resolved = StarknetResolvedValue::String(string);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "string_array" => {
                let string: String = self
                    .inner
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
                let resolved = StarknetResolvedValue::String(string);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "address" => {
                let string = format!(
                    "{:#066x}",
                    self.inner.pop().expect(
                        "provide only one StarknetResolvedValue cannot be processed as an address"
                    )
                );

                let resolved = StarknetResolvedValue::Address(string);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "u64" => {
                let int = u64::try_from(self.inner.pop().unwrap()).unwrap();
                let resolved = StarknetResolvedValue::Int(int);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "u64_array" => {
                let integers = self
                    .inner
                    .iter()
                    .skip(1)
                    .map(|fe| u64::try_from(fe.to_owned()).unwrap())
                    .collect();
                let resolved = StarknetResolvedValue::IntArray(integers);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "bool" => {
                let bool = self.inner.pop().unwrap() == FieldElement::ONE;
                let resolved = StarknetResolvedValue::Bool(bool);
                self.resolved = Some(resolved.clone());
                resolved
            }
            _ => panic!("starknet required type not implemented yet"),
        };
    }
}

#[derive(Clone, Debug)]
pub enum StarknetResolvedValue {
    Address(String),
    Int(u64),
    Float,
    String(String),
    Bool(bool),
    IntArray(Vec<u64>),
    DateArray,
    Date,
}

impl From<StarknetResolvedValue> for String {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::String(s) => s,
            StarknetResolvedValue::Int(i) => i.to_string(),
            StarknetResolvedValue::Address(a) => a,
            _ => panic!("cannot convert StarknetResolvedValue to string"),
        }
    }
}

impl From<StarknetResolvedValue> for i64 {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::Int(i) => i64::try_from(i).unwrap(),
            _ => panic!("cannot convert StarknetResolvedValue to string"),
        }
    }
}

impl From<StarknetResolvedValue> for sea_query::Value {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::String(s) => sea_query::Value::String(Some(Box::new(s))),
            StarknetResolvedValue::Address(s) => sea_query::Value::String(Some(Box::new(s))),
            StarknetResolvedValue::Int(i) => sea_query::Value::BigUnsigned(Some(i)),
            StarknetResolvedValue::Bool(b) => sea_query::Value::Bool(Some(b)),
            StarknetResolvedValue::IntArray(ia) => sea_query::Value::Array(
                sea_query::ArrayType::BigUnsigned,
                Some(Box::new(
                    ia.iter()
                        .map(|u| sea_query::Value::BigUnsigned(Some(u.to_owned())))
                        .collect(),
                )),
            ),
            StarknetResolvedValue::Float => todo!(),
            StarknetResolvedValue::DateArray => todo!(),
            StarknetResolvedValue::Date => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use starknet::core::types::FieldElement;

    use crate::infrastructure::starknet::model::{StarknetValue, StarknetValueResolver};

    use super::StarknetResolvedValue;

    #[test]
    fn test_starknet_resolved_value_from_bool_felt() {
        let falsy = StarknetValue::new(vec![FieldElement::ZERO]).resolve("bool");
        let truthy = StarknetValue::new(vec![FieldElement::ONE]).resolve("bool");
        if let StarknetResolvedValue::Bool(false_bool) = falsy {
            assert_eq!(false, false_bool);
        }
        if let StarknetResolvedValue::Bool(true_bool) = truthy {
            assert_eq!(true, true_bool);
        }
    }
}
