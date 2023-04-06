use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infrastructure::postgres::entity::ErcImplementation;

#[derive(Debug)]
pub struct ProjectWithMinterAndPaymentViewModel {
    pub id: Uuid,
    pub address: String,
    pub name: String,
    pub slug: String,
    pub slot: Option<i64>,
    pub erc_implementation: ErcImplementation,
    pub minter_id: Uuid,
    pub unit_price: f64,
    pub minter_address: String,
    pub payment_id: Uuid,
    pub payment_decimals: i64,
}

impl From<tokio_postgres::Row> for ProjectWithMinterAndPaymentViewModel {
    fn from(value: tokio_postgres::Row) -> Self {
        let erc_implementation: ErcImplementation = value.get(5);
        Self {
            id: value.get(0),
            address: value.get(1),
            name: value.get(2),
            slug: value.get(3),
            slot: value.get(4),
            erc_implementation,
            minter_id: value.get(6),
            unit_price: value.get(7),
            minter_address: value.get(8),
            payment_id: value.get(9),
            payment_decimals: value.get(10),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub token_id: u64,
    pub image: String,
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Erc3525Token {
    pub token_id: u64,
    pub value: u64,
    pub name: String,
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProjectWithTokens {
    Erc721 {
        id: Uuid,
        name: String,
        address: String,
        minter_address: String,
        tokens: Vec<Token>,
        #[serde(skip_serializing)]
        total_amount: f64,
    },
    Erc3525 {
        id: Uuid,
        name: String,
        address: String,
        minter_address: String,
        tokens: Vec<Erc3525Token>,
        #[serde(skip_serializing)]
        total_amount: f64,
    },
}

impl ProjectWithTokens {
    pub fn get_total_amount(&self) -> f64 {
        match self {
            ProjectWithTokens::Erc721 { total_amount, .. } => *total_amount,
            ProjectWithTokens::Erc3525 { total_amount, .. } => *total_amount,
        }
    }
}
