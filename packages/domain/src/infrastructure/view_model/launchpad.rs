use crate::domain::{crypto::U256, Erc20, HumanComprehensibleU256, SlotValue};

use super::project::UriViewModel;
use crate::domain::Ulid;
use serde::{Deserialize, Serialize};
use time::{macros::offset, OffsetDateTime, PrimitiveDateTime};

#[derive(Debug, Serialize)]
pub struct Launchpad {
    is_ready: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    sale_date: Option<OffsetDateTime>,
    pub minter_contract: MinterContract,
    image: Option<String>,
    whitelisted_sale_open: bool,
    public_sale_open: bool,
    is_sold_out: bool,
}

#[derive(Debug, Serialize)]
pub struct CurrentMilestone {
    pub remaining: HumanComprehensibleU256<U256>,
    pub milestone_ceil: u64,
    pub boost: Option<String>,
    pub id: u64,
    pub ha: Option<String>,
    pub ton: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Milestone {
    pub boost: Option<String>,
    pub ceil: u64,
    pub id: u64,
    pub ha: Option<String>,
    pub ton: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub milestones: Vec<Milestone>,
    pub rating: String,
    pub ton_price: String,
}
impl From<&serde_json::Value> for ProjectMetadata {
    fn from(value: &serde_json::Value) -> Self {
        Self {
            milestones: value["milestones"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| Milestone {
                    boost: m["boost"].as_str().map(|b| b.to_owned()),
                    ceil: m["ceil"].as_u64().unwrap(),
                    id: m["id"].as_u64().unwrap_or(0),
                    ha: m["ha"].as_str().map(|b| b.to_owned()),
                    ton: m["ton"].as_str().map(|b| b.to_owned()),
                })
                .collect(),
            rating: value["rating"].as_str().unwrap().to_owned(),
            ton_price: value["ton_price"].as_str().unwrap().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MinterContract {
    pub address: String,
    pub abi: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct LaunchpadProjectDetails {
    pub(crate) id: Ulid,
    pub address: String,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) value_decimals: U256,
    pub slot: U256,
    pub uri: UriViewModel,
    pub(crate) forecasted_apr: Option<String>,
    pub(crate) total_value: Option<U256>,
    pub(crate) payment_token: Option<HumanComprehensibleU256<U256>>,
    pub metadata: Option<serde_json::Value>,
    pub current_milestone: Option<CurrentMilestone>,
}

#[derive(Debug, Serialize)]
pub struct LaunchpadProject {
    pub project: LaunchpadProjectDetails,
    pub launchpad: Launchpad,
    #[serde(skip_serializing_if = "Option::is_none")]
    whitelist: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<ProjectMint>,
}

#[derive(Debug, Serialize)]
pub struct ProjectMint {
    min_value_per_tx: HumanComprehensibleU256<U256>,
    max_value_per_tx: HumanComprehensibleU256<U256>,
    reserved_value: HumanComprehensibleU256<U256>,
    payment_token_address: String,
    pub total_value: Option<HumanComprehensibleU256<U256>>,
    pub remaining_value: Option<HumanComprehensibleU256<U256>>,
}

fn extract_payment_token_from_query(value: &tokio_postgres::Row) -> Option<Erc20> {
    let value_decimals: U256 = value.get(5);
    let unit_price: U256 = match value.try_get(19) {
        Ok(up) => up,
        Err(_) => return None,
    };
    let decimals: U256 = match value.try_get(21) {
        Ok(d) => d,
        Err(_) => return None,
    };
    let symbol: String = match value.try_get(20) {
        Ok(s) => s,
        Err(_) => return None,
    };

    Some(Erc20::from_blockchain(
        unit_price,
        decimals - value_decimals,
        symbol,
    ))
}

fn extract_project_mint_from_query(value: &tokio_postgres::Row) -> Option<ProjectMint> {
    let value_decimals = value.get(5);
    let min_value_per_tx: HumanComprehensibleU256<U256> = match value.try_get(22) {
        Ok(v) => SlotValue::from_blockchain(v, value_decimals).into(),
        Err(_) => return None,
    };
    let max_value_per_tx: HumanComprehensibleU256<U256> = match value.try_get(23) {
        Ok(v) => SlotValue::from_blockchain(v, value_decimals).into(),
        Err(_) => return None,
    };
    let reserved_value: HumanComprehensibleU256<U256> = match value.try_get(24) {
        Ok(v) => SlotValue::from_blockchain(v, value_decimals).into(),
        Err(_) => return None,
    };
    let payment_token_address: String = match value.try_get(25) {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(ProjectMint {
        min_value_per_tx,
        max_value_per_tx,
        reserved_value,
        payment_token_address,
        total_value: None,
        remaining_value: None,
    })
}

impl From<tokio_postgres::Row> for LaunchpadProject {
    fn from(value: tokio_postgres::Row) -> Self {
        let sale_date: Option<OffsetDateTime> = match value.try_get::<usize, PrimitiveDateTime>(10)
        {
            Ok(dt) => Some(
                OffsetDateTime::from_unix_timestamp(dt.assume_offset(offset!(+0)).unix_timestamp())
                    .unwrap(),
            ),
            Err(_) => None,
        };
        let payment_token: Option<Erc20> = extract_payment_token_from_query(&value);
        let project_mint: Option<ProjectMint> = extract_project_mint_from_query(&value);

        LaunchpadProject {
            project: LaunchpadProjectDetails {
                id: value.get(0),
                address: value.get(1),
                name: value.get(2),
                slug: value.get(3),
                uri: super::project::UriViewModel {
                    id: value.get(7),
                    uri: value.get(8),
                    data: value.get(9),
                },
                value_decimals: value.get(5),
                slot: value.get(6),
                total_value: match value.try_get(17) {
                    Ok(tv) => Some(tv),
                    Err(_) => None,
                },
                payment_token: payment_token.map(|p| p.into()),
                forecasted_apr: match value.try_get(18) {
                    Ok(fa) => Some(fa),
                    Err(_) => None,
                },
                metadata: match value.try_get(26) {
                    Ok(m) => Some(m),
                    Err(_) => None,
                },
                current_milestone: None,
            },
            launchpad: Launchpad {
                is_ready: value.get(4),
                sale_date,
                minter_contract: MinterContract {
                    address: value.get(11),
                    abi: value.get(15),
                },
                image: None,
                whitelisted_sale_open: value.get(12),
                public_sale_open: value.get(13),
                is_sold_out: value.get(14),
            },
            whitelist: match value.try_get(16) {
                Ok(w) => Some(w),
                Err(_) => None,
            },
            mint: project_mint,
        }
    }
}
