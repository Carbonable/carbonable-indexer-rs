use crate::domain::Ulid;
use serde::Serialize;

use crate::domain::crypto::U256;
use crate::domain::HumanComprehensibleU256;
use crate::infrastructure::postgres::entity::ErcImplementation;

#[derive(Debug)]
pub struct ProjectWithMinterAndPaymentViewModel {
    pub id: Ulid,
    pub address: String,
    pub name: String,
    pub slug: String,
    pub slot: Option<U256>,
    pub erc_implementation: ErcImplementation,
    pub value_decimals: U256,
    pub minter_id: Option<Ulid>,
    pub unit_price: Option<U256>,
    pub symbol: Option<String>,
    pub minter_address: String,
    pub payment_id: Ulid,
    pub payment_decimals: U256,
    pub abi: serde_json::Value,
    pub minter_abi: serde_json::Value,
    pub yielder_address: String,
    pub offseter_address: String,
    pub slot_uri: Option<String>,
}

impl From<tokio_postgres::Row> for ProjectWithMinterAndPaymentViewModel {
    fn from(value: tokio_postgres::Row) -> Self {
        let erc_implementation: ErcImplementation = value.get(5);
        let yielder_address: Option<String> = value.get(15);
        let offsetter_address: Option<String> = value.get(16);
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
            yielder_address: if let Some(yielder_address) = yielder_address {
                yielder_address
            } else {
                "".to_string()
            },
            offseter_address: if let Some(offsetter_address) = offsetter_address {
                offsetter_address
            } else {
                "".to_string()
            },
            slot_uri: value.get(17),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Token {
    pub token_id: U256,
    pub name: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Erc3525Token {
    pub token_id: U256,
    #[serde(skip_serializing)]
    pub value: U256,
    pub name: String,
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
        id: Ulid,
        name: String,
        address: String,
        minter_address: String,
        tokens: Vec<Token>,
        #[serde(skip_serializing)]
        total_amount: U256,
        abi: PortfolioAbi,
        image: serde_json::Value,
        migrator_address: Option<String>,
    },
    Erc3525 {
        id: Ulid,
        name: String,
        address: String,
        minter_address: String,
        tokens: Vec<Erc3525Token>,
        #[serde(skip_serializing)]
        total_amount: U256,
        total_deposited_value: HumanComprehensibleU256<U256>,
        abi: PortfolioAbi,
        image: serde_json::Value,
        asset_area: Option<String>,
        asset_carbon_unit: Option<String>,
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
