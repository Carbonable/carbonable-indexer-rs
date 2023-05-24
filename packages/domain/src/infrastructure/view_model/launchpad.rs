use crate::domain::{crypto::U256, Erc20, HumanComprehensibleU256};

use super::project::UriViewModel;
use serde::Serialize;
use time::{macros::offset, OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Launchpad {
    is_ready: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    sale_date: Option<OffsetDateTime>,
    minter_contract: MinterContract,
    image: Option<String>,
    whitelisted_sale_open: bool,
    public_sale_open: bool,
    is_sold_out: bool,
}

#[derive(Debug, Serialize)]
pub struct MinterContract {
    address: String,
    abi: serde_json::Value,
}

#[derive(Serialize)]
pub struct LaunchpadProjectDetails {
    pub(crate) id: Uuid,
    pub(crate) address: String,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub uri: UriViewModel,
    pub(crate) forecasted_apr: Option<String>,
    pub(crate) total_value: Option<U256>,
    pub(crate) payment_token: Option<HumanComprehensibleU256<U256>>,
}

#[derive(Serialize)]
pub struct LaunchpadProject {
    project: LaunchpadProjectDetails,
    launchpad: Launchpad,
    #[serde(skip_serializing_if = "Option::is_none")]
    whitelist: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mint: Option<ProjectMint>,
}

#[derive(Serialize)]
pub struct ProjectMint {
    min_value_per_tx: U256,
    max_value_per_tx: U256,
    reserved_value: U256,
    max_value: Option<U256>,
    payment_token_address: String,
}

fn extract_payment_token_from_query(value: &tokio_postgres::Row) -> Option<Erc20> {
    let unit_price: U256 = match value.try_get(17) {
        Ok(up) => up,
        Err(_) => return None,
    };
    let decimals: U256 = match value.try_get(19) {
        Ok(d) => d,
        Err(_) => return None,
    };
    let symbol: String = match value.try_get(18) {
        Ok(s) => s,
        Err(_) => return None,
    };

    Some(Erc20::from_blockchain(unit_price, decimals, symbol))
}

fn extract_project_mint_from_query(value: &tokio_postgres::Row) -> Option<ProjectMint> {
    let min_value_per_tx: U256 = match value.try_get(20) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let max_value_per_tx: U256 = match value.try_get(21) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let reserved_value: U256 = match value.try_get(22) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let max_value: Option<U256> = match value.try_get(23) {
        Ok(v) => Some(v),
        Err(_) => None,
    };
    let payment_token_address: String = match value.try_get(24) {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(ProjectMint {
        min_value_per_tx,
        max_value_per_tx,
        reserved_value,
        max_value,
        payment_token_address,
    })
}

impl From<tokio_postgres::Row> for LaunchpadProject {
    fn from(value: tokio_postgres::Row) -> Self {
        let sale_date: Option<OffsetDateTime> = match value.try_get::<usize, PrimitiveDateTime>(8) {
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
                    id: value.get(5),
                    uri: value.get(6),
                    data: value.get(7),
                },
                total_value: match value.try_get(15) {
                    Ok(tv) => Some(tv),
                    Err(_) => None,
                },
                payment_token: payment_token.map(|p| p.into()),
                forecasted_apr: match value.try_get(16) {
                    Ok(fa) => Some(fa),
                    Err(_) => None,
                },
            },
            launchpad: Launchpad {
                is_ready: value.get(4),
                sale_date,
                minter_contract: MinterContract {
                    address: value.get(9),
                    abi: value.get(13),
                },
                image: None,
                whitelisted_sale_open: value.get(10),
                public_sale_open: value.get(11),
                is_sold_out: value.get(12),
            },
            whitelist: match value.try_get(14) {
                Ok(w) => Some(w),
                Err(_) => None,
            },
            mint: project_mint,
        }
    }
}
