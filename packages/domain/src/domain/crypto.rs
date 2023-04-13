use bigdecimal::ToPrimitive;
use crypto_bigint::{CheckedAdd, CheckedMul, CheckedSub, Encoding};
use postgres_types::FromSql;
use serde::{ser::SerializeStruct, Serialize};

#[derive(Debug, Copy, PartialEq, Eq, Default, Clone)]
pub struct U256(pub(crate) crypto_bigint::U256);
impl U256 {
    pub fn zero() -> Self {
        Self(crypto_bigint::U256::ZERO)
    }
}

/// A U256 human-comprehensible representation.
/// This keeps track of an inner U256 and a string
/// that will help frontend to display content easyli
#[derive(Debug, Default, Clone, Serialize)]
pub struct HumanComprehensibleU256 {
    #[serde(rename = "value")]
    pub inner: U256,
    #[serde(rename = "displayable_value")]
    pub repr: String,
}

impl From<U256> for HumanComprehensibleU256 {
    fn from(value: U256) -> Self {
        Self {
            inner: value,
            repr: value.0.to_string(),
        }
    }
}

impl From<HumanComprehensibleU256> for U256 {
    fn from(value: HumanComprehensibleU256) -> Self {
        value.inner
    }
}

impl Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let words = self.0.to_words();
        let mut u256 = serializer.serialize_struct("U256", 4)?;
        u256.serialize_field("lo_lo", &words[0])?;
        u256.serialize_field("lo_hi", &words[1])?;
        u256.serialize_field("hi_lo", &words[2])?;
        u256.serialize_field("hi_hi", &words[3])?;
        u256.end()
    }
}

impl<'a> FromSql<'a> for U256 {
    fn from_sql(
        _ty: &postgres_types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(U256(crypto_bigint::U256::from_be_slice(raw)))
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty == &postgres_types::Type::BYTEA
    }
}

impl From<crypto_bigint::U256> for U256 {
    fn from(value: crypto_bigint::U256) -> Self {
        U256(value)
    }
}
impl From<U256> for crypto_bigint::U256 {
    fn from(value: U256) -> Self {
        value.0
    }
}

impl From<U256> for sea_query::Value {
    fn from(value: U256) -> Self {
        sea_query::Value::Bytes(Some(Box::new(value.0.to_be_bytes().to_vec())))
    }
}
impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        U256(crypto_bigint::U256::from_u64(value))
    }
}
impl From<u128> for U256 {
    fn from(value: u128) -> Self {
        U256(crypto_bigint::U256::from_u128(value))
    }
}
impl From<usize> for U256 {
    fn from(value: usize) -> Self {
        U256(crypto_bigint::U256::from_u32(value.to_u32().unwrap()))
    }
}
impl From<time::Duration> for U256 {
    fn from(value: time::Duration) -> Self {
        value.unsigned_abs().as_micros().into()
    }
}
impl From<std::time::Duration> for U256 {
    fn from(value: std::time::Duration) -> Self {
        value.as_micros().into()
    }
}
impl std::ops::Div for U256 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        U256(self.0.checked_div(&rhs.0).unwrap())
    }
}
impl std::ops::Add for U256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        U256(self.0.checked_add(&rhs.0).unwrap())
    }
}
impl std::ops::AddAssign for U256 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.checked_add(&rhs.0).unwrap();
    }
}
impl std::ops::Sub for U256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        U256(self.0.checked_sub(&rhs.0).unwrap())
    }
}
impl std::ops::Mul for U256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        U256(self.0.checked_mul(&rhs.0).unwrap())
    }
}
impl std::ops::MulAssign for U256 {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.checked_mul(&rhs.0).unwrap();
    }
}
