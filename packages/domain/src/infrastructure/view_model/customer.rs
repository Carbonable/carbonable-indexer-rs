use crate::domain::{crypto::U256, SlotValue};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CustomerToken {
    pub wallet: String,
    pub project_address: String,
    pub slot: U256,
    pub token_id: U256,
    pub value: U256,
    pub value_decimals: Option<U256>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerTokenWithSlotValue {
    pub wallet: String,
    pub project_address: String,
    pub slot: U256,
    pub token_id: U256,
    pub value: SlotValue,
}

impl From<tokio_postgres::Row> for CustomerToken {
    fn from(value: tokio_postgres::Row) -> Self {
        let row_token_value: Option<U256> = value.get(4);
        let token_value = match row_token_value {
            Some(v) => v,
            None => U256::zero(),
        };

        Self {
            wallet: value.get(0),
            project_address: value.get(1),
            slot: value.get(2),
            token_id: value.get(3),
            value: token_value,
            value_decimals: value.get(5),
        }
    }
}
