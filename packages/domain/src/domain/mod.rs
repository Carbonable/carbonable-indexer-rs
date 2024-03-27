use std::fmt::Display;

use self::crypto::U256;
use postgres_types::FromSql;
use sea_query::Nullable;
use serde::{Deserialize, Serialize};

pub mod crypto;
pub mod event_source;
pub mod project;

pub trait Contract {}

/// Structure representing an ERC-721 smart contract
#[derive(Debug)]
pub struct Erc721;
impl Contract for Erc721 {}

/// Structure representing an ERC-3525 smart contract
#[derive(Debug)]
pub struct Erc3525;
impl Contract for Erc3525 {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Ulid(ulid::Ulid);
impl Ulid {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl From<ulid::Ulid> for Ulid {
    fn from(value: ulid::Ulid) -> Self {
        Self(value)
    }
}
impl From<Ulid> for ulid::Ulid {
    fn from(value: Ulid) -> Self {
        value.0
    }
}
impl From<Ulid> for sea_query::Value {
    fn from(value: Ulid) -> Self {
        sea_query::Value::String(Some(Box::new(value.0.to_string())))
    }
}
impl Nullable for Ulid {
    fn null() -> sea_query::Value {
        sea_query::Value::String(None)
    }
}
impl Display for Ulid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}
impl From<String> for Ulid {
    fn from(value: String) -> Self {
        Self(ulid::Ulid::from_string(value.as_str()).expect("string is not a valid ulid"))
    }
}
impl From<&str> for Ulid {
    fn from(value: &str) -> Self {
        Self(ulid::Ulid::from_string(value).expect("string is not a valid ulid"))
    }
}
impl<'a> FromSql<'a> for Ulid {
    fn from_sql(
        _ty: &postgres_types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let str = std::str::from_utf8(raw)?;
        Ok(Ulid(ulid::Ulid::from_string(str)?))
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty == &postgres_types::Type::VARCHAR
    }
}

/// Represents a project slot value
#[derive(Debug, Default, Serialize, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
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

    pub fn inner(&self) -> U256 {
        self.value
    }
}

impl std::ops::AddAssign for SlotValue {
    fn add_assign(&mut self, rhs: Self) {
        self.value = self.value + rhs.value;
    }
}
impl std::ops::AddAssign<U256> for SlotValue {
    fn add_assign(&mut self, rhs: U256) {
        self.value = self.value + rhs;
    }
}
impl std::ops::SubAssign<U256> for SlotValue {
    fn sub_assign(&mut self, rhs: U256) {
        self.value = self.value - rhs;
    }
}

/// Represents an ERC-20 token used to finance projects
#[derive(Debug, Default, Serialize, Clone, Eq, Ord, PartialEq, PartialOrd)]
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
        if self.decimals == 0 {
            self.decimals = rhs.decimals;
        }
        if self.symbol.is_empty() {
            self.symbol = rhs.symbol;
        }
    }
}
impl std::ops::AddAssign<U256> for Erc20 {
    fn add_assign(&mut self, rhs: U256) {
        self.value = self.value + rhs;
    }
}
impl std::ops::SubAssign<U256> for Erc20 {
    fn sub_assign(&mut self, rhs: U256) {
        self.value = self.value - rhs;
    }
}

/// Mass can either be a U256 or a bigdecimal depending if we're dividing value by another one or not
/// If at any moment you have to divide anything please reach out king of bits aka @tekkac
/// Represents mass from blockchain
#[derive(Debug, Default, Serialize, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Mass<T> {
    value: T,
}

impl Mass<U256> {
    pub fn from_blockchain(value: U256, ton_equivalent: U256) -> Self {
        if U256::zero() == ton_equivalent {
            return Self {
                value: value * U256(crypto_bigint::U256::from_u64(1000000)),
            };
        }

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
impl std::ops::AddAssign<U256> for Mass<U256> {
    fn add_assign(&mut self, rhs: U256) {
        self.value = self.value + rhs;
    }
}
impl std::ops::SubAssign<U256> for Mass<U256> {
    fn sub_assign(&mut self, rhs: U256) {
        self.value = self.value - rhs;
    }
}
///
/// A U256 human-comprehensible representation.
/// This keeps track of an inner U256 and a string
/// that will help frontend to display content easyli
#[derive(Debug, Default, Clone, Serialize, Ord, PartialOrd, Eq, PartialEq)]
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
