use serde::Serialize;
use uuid::Uuid;

use crate::domain::crypto::U256;
use crate::domain::HumanComprehensibleU256;
use crate::infrastructure::postgres::entity::ErcImplementation;

#[derive(Debug)]
pub struct ProjectWithMinterAndPaymentViewModel {
    pub id: Uuid,
    pub address: String,
    pub name: String,
    pub slug: String,
    pub slot: Option<U256>,
    pub erc_implementation: ErcImplementation,
    pub value_decimals: U256,
    pub minter_id: Uuid,
    pub unit_price: U256,
    pub symbol: String,
    pub minter_address: String,
    pub payment_id: Uuid,
    pub payment_decimals: U256,
    pub abi: serde_json::Value,
    pub minter_abi: serde_json::Value,
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
            value_decimals: value.get(6),
            minter_id: value.get(7),
            unit_price: value.get(8),
            symbol: value.get(9),
            minter_address: value.get(10),
            payment_id: value.get(11),
            payment_decimals: value.get(12),
            abi: value.get(13),
            minter_abi: value.get(14),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Token {
    pub token_id: U256,
    pub image: String,
    pub name: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Erc3525Token {
    pub token_id: U256,
    #[serde(skip_serializing)]
    pub value: U256,
    pub name: String,
    pub image: String,
    #[serde(rename = "value")]
    pub slot_value: HumanComprehensibleU256<U256>,
}

#[derive(Debug, Serialize)]
pub struct PortfolioAbi {
    pub project: serde_json::Value,
    pub minter: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProjectWithTokens {
    Erc721 {
        id: Uuid,
        name: String,
        address: String,
        minter_address: String,
        tokens: Vec<Token>,
        #[serde(skip_serializing)]
        total_amount: U256,
        abi: PortfolioAbi,
    },
    Erc3525 {
        id: Uuid,
        name: String,
        address: String,
        minter_address: String,
        tokens: Vec<Erc3525Token>,
        #[serde(skip_serializing)]
        total_amount: U256,
        abi: PortfolioAbi,
    },
}

impl ProjectWithTokens {
    pub fn get_total_amount(&self) -> U256 {
        match self {
            ProjectWithTokens::Erc721 { total_amount, .. } => *total_amount,
            ProjectWithTokens::Erc3525 { total_amount, .. } => *total_amount,
        }
    }
}
