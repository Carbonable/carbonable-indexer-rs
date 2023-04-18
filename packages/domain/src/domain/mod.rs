use self::crypto::U256;
use serde::Serialize;

pub mod crypto;
pub mod event_source;
pub mod project;

pub trait Contract {}

/// Structure representing an ERC-721 smart contract
pub struct Erc721;
impl Contract for Erc721 {}

/// Structure representing an ERC-3525 smart contract
pub struct Erc3525;
impl Contract for Erc3525 {}

/// Represents a project slot value
#[derive(Debug, Default, Serialize, Copy, Clone)]
pub struct SlotValue {
    value: U256,
    value_decimals: u32,
}

impl SlotValue {
    pub fn from_blockchain(value: U256, value_decimals: U256) -> Self {
        Self {
            value,
            value_decimals: value_decimals.into(),
        }
    }
}

impl std::ops::AddAssign for SlotValue {
    fn add_assign(&mut self, rhs: Self) {
        self.value = self.value + rhs.value;
    }
}

/// Represents an ERC-20 token used to finance projects
#[derive(Debug, Default, Serialize, Clone)]
pub struct Erc20 {
    symbol: String,
    decimals: u32,
    value: U256,
}

impl Erc20 {
    pub fn from_blockchain(value: U256, decimals: U256, symbol: String) -> Self {
        Self {
            symbol,
            decimals: decimals.into(),
            value,
        }
    }
}

impl std::ops::AddAssign for Erc20 {
    fn add_assign(&mut self, rhs: Self) {
        self.value = self.value + rhs.value;
    }
}

/// Mass can either be a U256 or a bigdecimal depending if we're dividing value by another one or not
/// If at any moment you have to divide anything please reach out king of bits aka @tekkac
/// Represents mass from blockchain
#[derive(Debug, Default, Serialize, Copy, Clone)]
pub struct Mass<T> {
    value: T,
}

impl Mass<U256> {
    pub fn from_blockchain(value: U256, ton_equivalent: U256) -> Self {
        Self {
            // Convert to grams. For the moment we store the values in grams but it might change
            value: (value * U256(crypto_bigint::U256::from_u64(1000000))) / ton_equivalent,
        }
    }
}

impl Mass<bigdecimal::BigDecimal> {
    pub fn from_blockchain(value: bigdecimal::BigDecimal) -> Self {
        Self { value }
    }
}

impl<T: std::ops::AddAssign + std::ops::Add<Output = T> + Clone> std::ops::AddAssign for Mass<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.value = self.value.clone() + rhs.value;
    }
}
///
/// A U256 human-comprehensible representation.
/// This keeps track of an inner U256 and a string
/// that will help frontend to display content easyli
#[derive(Debug, Default, Clone, Serialize)]
#[serde(tag = "type")]
pub enum HumanComprehensibleU256<T> {
    #[default]
    NotSet,
    #[serde(rename = "slot_value")]
    SlotValue {
        #[serde(rename = "value")]
        inner: SlotValue,
        displayable_value: String,
    },
    #[serde(rename = "erc20")]
    Erc20 {
        #[serde(rename = "value")]
        inner: Erc20,
        displayable_value: String,
    },
    #[serde(rename = "mass")]
    Mass {
        #[serde(rename = "value")]
        inner: Mass<T>,
        displayable_value: String,
    },
}

impl From<SlotValue> for HumanComprehensibleU256<U256> {
    fn from(value: SlotValue) -> Self {
        Self::SlotValue {
            inner: value,
            displayable_value: value.value.to_big_decimal(value.value_decimals).to_string(),
        }
    }
}
impl From<Erc20> for HumanComprehensibleU256<Erc20> {
    fn from(value: Erc20) -> Self {
        Self::Erc20 {
            inner: value.clone(),
            displayable_value: value.value.to_big_decimal(value.decimals).to_string(),
        }
    }
}
impl From<Erc20> for HumanComprehensibleU256<U256> {
    fn from(value: Erc20) -> Self {
        Self::Erc20 {
            inner: value.clone(),
            displayable_value: value.value.to_big_decimal(value.decimals).to_string(),
        }
    }
}

impl From<Mass<U256>> for HumanComprehensibleU256<Mass<U256>> {
    fn from(value: Mass<U256>) -> Self {
        Self::Mass {
            inner: Mass { value },
            displayable_value: value.value.to_big_decimal(0).to_string(),
        }
    }
}

impl From<Mass<U256>> for HumanComprehensibleU256<U256> {
    fn from(value: Mass<U256>) -> Self {
        Self::Mass {
            inner: value,
            displayable_value: value.value.to_big_decimal(0).to_string(),
        }
    }
}

impl From<Mass<bigdecimal::BigDecimal>> for HumanComprehensibleU256<bigdecimal::BigDecimal> {
    fn from(value: Mass<bigdecimal::BigDecimal>) -> Self {
        Self::Mass {
            inner: value.clone(),
            displayable_value: value.value.to_string(),
        }
    }
}
