use bigdecimal::{BigDecimal, ToPrimitive};
use crypto_bigint::Encoding;
use std::{collections::HashMap, sync::Arc};

use starknet::{
    core::{
        types::FieldElement,
        utils::{get_selector_from_name, NonAsciiNameError},
    },
    providers::{
        jsonrpc::{
            models::{BlockId, BlockTag, FunctionCall},
            HttpTransport, JsonRpcClient, JsonRpcClientError,
        },
        ProviderError, SequencerGatewayProviderError,
    },
};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::task::JoinError;

use crate::domain::crypto::U256;
use crate::infrastructure::flatten;

use super::SequencerError;

#[derive(Debug, Error)]
pub enum ModelError {
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
    ProviderError(#[from] JsonRpcClientError<reqwest::Error>),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error("invalid data set on field {0}")]
    InvalidDataSet(String),
}

#[async_trait::async_trait]
pub trait StarknetModel<T> {
    async fn load(&self) -> Result<T, ModelError>;
}

/// Transforms FieldElelent to U256
pub fn felt_to_u256(element: FieldElement) -> U256 {
    U256(crypto_bigint::U256::from_be_bytes(element.to_bytes_be()))
}

/// Transforms U256 to FieldElement
pub fn u256_to_felt(u: &U256) -> FieldElement {
    FieldElement::from_bytes_be(&u.0.to_be_bytes()).unwrap()
}

pub fn get_call_function(
    address: &FieldElement,
    selector: &str,
    calldata: Vec<FieldElement>,
) -> FunctionCall {
    FunctionCall {
        contract_address: *address,
        entry_point_selector: get_selector_from_name(selector).unwrap(),
        calldata,
    }
}

/// Sync starknet model with some base data
pub(crate) async fn load_blockchain_data(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: FieldElement,
    selectors: &'static [&str],
) -> Result<HashMap<String, StarknetValue>, ModelError> {
    let mut handles = vec![];
    for selector in selectors {
        let provider = provider.clone();

        let handle = tokio::spawn(async move {
            let contract_entrypoint = selector;
            let res = provider
                .call(
                    get_call_function(&address, contract_entrypoint, vec![]),
                    &BlockId::Tag(BlockTag::Latest),
                )
                .await;
            Ok((selector.to_string(), StarknetValue::new(res.unwrap())))
        });

        handles.push(flatten(handle));
    }

    match futures::future::try_join_all(handles).await {
        Ok(res) => Ok(to_hash_map(res)),
        Err(e) => Err(e),
    }
}
/// Get blockchain data with a whole call construction
/// Async & multi-threaded
pub async fn parallelize_blockchain_rpc_calls(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    data: Vec<(String, &'static str, Vec<FieldElement>)>,
) -> Result<Vec<Vec<FieldElement>>, ModelError> {
    let mut handles = vec![];
    for (address, endpoint, values) in data.into_iter() {
        let provider = provider.clone();
        let address = address.to_string();
        let endpoint = endpoint.to_string();
        let handle = tokio::spawn(async move {
            match provider
                .call(
                    get_call_function(
                        &FieldElement::from_hex_be(address.as_str()).unwrap(),
                        endpoint.as_str(),
                        values.to_vec(),
                    ),
                    &BlockId::Tag(BlockTag::Latest),
                )
                .await
            {
                Ok(res) => Ok(res),
                Err(err) => Err(ModelError::ProviderError(err)),
            }
        });

        handles.push(flatten(handle));
    }

    match futures::future::try_join_all(handles).await {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
}

pub(crate) async fn load_blockchain_slot_data(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: FieldElement,
    slot: U256,
    selectors: &'static [&str],
) -> Result<HashMap<String, StarknetValue>, ModelError> {
    let mut handles = vec![];
    for selector in selectors {
        let provider = provider.clone();
        let handle = tokio::spawn(async move {
            let contract_entrypoint = selector;
            let res = provider
                .call(
                    get_call_function(
                        &address,
                        contract_entrypoint,
                        vec![u256_to_felt(&slot), FieldElement::ZERO],
                    ),
                    &BlockId::Tag(BlockTag::Latest),
                )
                .await;
            Ok((selector.to_string(), StarknetValue::new(res.unwrap())))
        });

        handles.push(flatten(handle));
    }
    match futures::future::try_join_all(handles).await {
        Ok(res) => Ok(to_hash_map(res)),
        Err(e) => Err(e),
    }
}
fn to_hash_map(value: Vec<(String, StarknetValue)>) -> HashMap<String, StarknetValue> {
    value.iter().fold(HashMap::new(), |mut acc, res| {
        acc.insert(res.0.clone(), res.1.clone());
        acc
    })
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
                let resolved = StarknetResolvedValue::Unsigned(int);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "i64" => {
                let decimal = self.inner.first().unwrap().to_owned().to_big_decimal(0);
                let resolved = StarknetResolvedValue::Int(decimal.to_i64().unwrap());
                self.resolved = Some(resolved.clone());
                resolved
            }
            "bigint" => {
                let big_decimal = self.inner.first().unwrap().to_owned().to_big_decimal(0);
                let resolved = StarknetResolvedValue::BigInt(big_decimal);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "u256" => {
                let int: U256 = felt_to_u256(*self.inner.first().unwrap());
                let resolved = StarknetResolvedValue::U256(int);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "u256_array" => {
                let integers = self
                    .inner
                    .iter()
                    .skip(1)
                    .map(|fe| felt_to_u256(fe.to_owned()))
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
            "datetime" => {
                let int = u64::try_from(self.inner.pop().unwrap()).unwrap();
                let datetime = OffsetDateTime::from_unix_timestamp(int as i64).unwrap();
                let resolved = StarknetResolvedValue::Date(datetime);
                self.resolved = Some(resolved.clone());
                resolved
            }
            "datetime_array" => {
                let integers = self
                    .inner
                    .iter()
                    .skip(1)
                    .map(|fe| u64::try_from(fe.to_owned()).unwrap())
                    .collect();
                let resolved = StarknetResolvedValue::DateArray(integers);
                self.resolved = Some(resolved.clone());
                resolved
            }
            _ => panic!(
                "starknet required type not implemented yet {}",
                required_type
            ),
        };
    }
}

#[derive(Clone, Debug)]
pub enum StarknetResolvedValue {
    Address(String),
    Unsigned(u64),
    Int(i64),
    Float,
    String(String),
    Bool(bool),
    IntArray(Vec<U256>),
    DateArray(Vec<u64>),
    Date(OffsetDateTime),
    BigInt(BigDecimal),
    U256(U256),
}

impl From<StarknetResolvedValue> for String {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::String(s) => s,
            StarknetResolvedValue::Unsigned(i) => i.to_string(),
            StarknetResolvedValue::Address(a) => a,
            _ => panic!("cannot convert StarknetResolvedValue to string"),
        }
    }
}

impl From<StarknetResolvedValue> for BigDecimal {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::BigInt(s) => s,
            StarknetResolvedValue::Unsigned(_i) => BigDecimal::from(value),
            _ => panic!("cannot convert StarknetResolvedValue to big decimal"),
        }
    }
}

impl From<StarknetResolvedValue> for i64 {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::Unsigned(i) => i64::try_from(i).unwrap(),
            StarknetResolvedValue::Int(i) => i,
            _ => panic!("cannot convert StarknetResolvedValue to i64"),
        }
    }
}

impl From<StarknetResolvedValue> for u64 {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::Unsigned(i) => i,
            _ => panic!("cannot convert StarknetResolvedValue to u64"),
        }
    }
}

impl From<StarknetResolvedValue> for U256 {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::U256(i) => i,
            _ => panic!("cannot convert StarknetResolvedValue to U256"),
        }
    }
}
impl From<StarknetResolvedValue> for crypto_bigint::U256 {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::U256(i) => crypto_bigint::U256::from_words(i.0.to_words()),
            _ => panic!("cannot convert StarknetResolvedValue to U256"),
        }
    }
}

impl From<StarknetResolvedValue> for sea_query::Value {
    fn from(value: StarknetResolvedValue) -> Self {
        match value {
            StarknetResolvedValue::String(s) => sea_query::Value::String(Some(Box::new(s))),
            StarknetResolvedValue::Address(s) => sea_query::Value::String(Some(Box::new(s))),
            StarknetResolvedValue::Unsigned(i) => sea_query::Value::BigUnsigned(Some(i)),
            StarknetResolvedValue::Bool(b) => sea_query::Value::Bool(Some(b)),
            StarknetResolvedValue::IntArray(ia) => sea_query::Value::Array(
                sea_query::ArrayType::Bytes,
                Some(Box::new(
                    ia.iter()
                        .map(|u| {
                            sea_query::Value::Bytes(Some(Box::new(u.0.to_be_bytes().to_vec())))
                        })
                        .collect(),
                )),
            ),
            StarknetResolvedValue::Float => todo!(),
            StarknetResolvedValue::DateArray(ia) => sea_query::Value::Array(
                sea_query::ArrayType::BigUnsigned,
                Some(Box::new(
                    ia.iter()
                        .map(|u| sea_query::Value::BigUnsigned(Some(*u)))
                        .collect(),
                )),
            ),
            StarknetResolvedValue::Date(d) => {
                sea_query::Value::TimeDateTimeWithTimeZone(Some(Box::new(d)))
            }
            StarknetResolvedValue::BigInt(big_decimal) => {
                sea_query::Value::BigDecimal(Some(Box::new(big_decimal)))
            }
            StarknetResolvedValue::Int(i) => sea_query::Value::BigInt(Some(i)),
            StarknetResolvedValue::U256(u) => {
                sea_query::Value::Bytes(Some(Box::new(u.0.to_be_bytes().to_vec())))
            }
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
