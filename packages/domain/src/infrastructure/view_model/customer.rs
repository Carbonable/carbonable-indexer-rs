use crate::domain::crypto::U256;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CustomerToken {
    pub wallet: String,
    pub project_address: String,
    pub slot: U256,
    pub token_id: U256,
    pub value: U256,
    pub value_decimals: U256,
}

impl From<tokio_postgres::Row> for CustomerToken {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            wallet: value.get(0),
            project_address: value.get(1),
            slot: value.get(2),
            token_id: value.get(3),
            value: value.get(4),
            value_decimals: value.get(5),
        }
    }
}
