use std::fmt::Display;

use bigdecimal::ToPrimitive;
use crypto_bigint::{CheckedAdd, CheckedMul, CheckedSub, Encoding};
use postgres_types::FromSql;
use serde::Serialize;

#[derive(Debug, Copy, PartialEq, PartialOrd, Eq, Default, Clone)]
pub struct U256(pub(crate) crypto_bigint::U256);
impl U256 {
    pub fn zero() -> Self {
        Self(crypto_bigint::U256::ZERO)
    }
    pub fn to_big_decimal<D: Into<i64>>(&self, decimals: D) -> bigdecimal::BigDecimal {
        use bigdecimal::num_bigint::{BigInt, Sign};

        bigdecimal::BigDecimal::new(
            BigInt::from_bytes_be(Sign::Plus, &crypto_bigint::Encoding::to_be_bytes(&self.0)),
            decimals.into(),
        )
    }
}
impl Display for U256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl From<U256> for u32 {
    fn from(value: U256) -> Self {
        value.0.to_words()[0]
            .try_into()
            .expect("failed to convert u256 to u32")
    }
}

impl Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

impl<'a> FromSql<'a> for U256 {
    fn from_sql(
        _ty: &postgres_types::Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let bytes: [u8; 32] = raw.try_into().unwrap();
        Ok(U256(crypto_bigint::U256::from_be_bytes(bytes)))
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
impl From<apibara_core::starknet::v1alpha2::FieldElement> for U256 {
    fn from(value: apibara_core::starknet::v1alpha2::FieldElement) -> Self {
        U256(crypto_bigint::U256::from_be_bytes(value.to_bytes()))
    }
}
impl From<U256> for crypto_bigint::U256 {
    fn from(value: U256) -> Self {
        value.0
    }
}
impl From<U256> for u64 {
    fn from(value: U256) -> Self {
        value.0.to_words()[0]
    }
}

impl From<U256> for sea_query::Value {
    fn from(value: U256) -> Self {
        sea_query::Value::Bytes(Some(Box::new(value.0.to_be_bytes().to_vec())))
    }
}
impl From<&U256> for sea_query::Value {
    fn from(value: &U256) -> Self {
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
        value.unsigned_abs().as_secs().into()
    }
}
impl From<std::time::Duration> for U256 {
    fn from(value: std::time::Duration) -> Self {
        value.as_micros().into()
    }
}
impl From<tokio_postgres::Row> for U256 {
    fn from(value: tokio_postgres::Row) -> Self {
        U256(crypto_bigint::U256::from_be_slice(value.get(0)))
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
