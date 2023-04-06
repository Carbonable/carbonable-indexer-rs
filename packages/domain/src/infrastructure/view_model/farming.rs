use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::infrastructure::starknet::model::{StarknetValue, StarknetValueResolver};

#[derive(Debug, Serialize, Deserialize)]
pub struct UriViewModel {
    pub uri: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FarmingProjectsViewModel {
    pub id: Uuid,
    pub address: String,
    pub name: String,
    pub slug: String,
    pub uri: UriViewModel,
}

impl From<tokio_postgres::Row> for FarmingProjectsViewModel {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            name: value.get(2),
            slug: value.get(3),
            uri: UriViewModel {
                uri: value.get(4),
                data: value.get(5),
            },
        }
    }
}

pub struct CustomerGlobalDataForComputation {
    pub id: uuid::Uuid,
    pub unit_price: f64,
    pub payment_decimals: i64,
    pub project_slot: i64,
    pub project_address: String,
    pub yielder_address: String,
    pub offseter_address: String,
    pub vester_address: String,
}

impl From<tokio_postgres::Row> for CustomerGlobalDataForComputation {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            unit_price: value.get(1),
            payment_decimals: value.get(2),
            project_slot: value.get(3),
            project_address: value.get(4),
            yielder_address: value.get(5),
            offseter_address: value.get(6),
            vester_address: value.get(7),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CustomerGlobalData {
    pub total_deposited: f64,
    pub total_released: f64,
    pub total_claimable: f64,
}
impl CustomerGlobalData {
    pub fn merge(mut self, other: Self) -> Self {
        self.total_deposited += other.total_deposited;
        self.total_released += other.total_released;
        self.total_claimable += other.total_claimable;
        self
    }
}

#[derive(Debug)]
pub struct CompleteFarmingData {
    pub id: Uuid,
    pub address: String,
    pub times: Vec<PrimitiveDateTime>,
    pub absorptions: Vec<i64>,
    pub ton_equivalent: i64,
    pub offseter_address: Option<String>,
    pub yielder_id: Option<Uuid>,
    pub yielder_address: Option<String>,
    pub vester_address: Option<String>,
}

impl From<tokio_postgres::Row> for CompleteFarmingData {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            times: value.get(2),
            absorptions: value.get(3),
            ton_equivalent: value.get(4),
            offseter_address: value.get(5),
            yielder_id: value.get(6),
            yielder_address: value.get(7),
            vester_address: value.get(8),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UnconnectedFarmingData {
    #[serde(flatten)]
    pub apr: ProjectApr,
    #[serde(flatten)]
    pub status: ProjectStatus,
    pub tvl: f64,
    pub total_removal: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(tag = "apr")]
pub enum ProjectApr {
    #[default]
    #[serde(rename = "n/a")]
    None,
    Value(f64),
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ProjectStatus {
    #[default]
    Upcoming,
    Ended,
    Live,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ContractsList {
    pub vester: String,
    pub yielder: String,
    pub offseter: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CustomerListingProjectData {
    pub customer_stake: f64,
    pub vesting_to_claim: f64,
    pub absorption_to_claim: f64,
    pub undeposited: i64,
    pub min_to_claim: i64,
    pub contracts: ContractsList,
}

impl
    From<(
        Vec<Vec<FieldElement>>,
        CustomerGlobalDataForComputation,
        CompleteFarmingData,
    )> for CustomerListingProjectData
{
    fn from(
        value: (
            Vec<Vec<FieldElement>>,
            CustomerGlobalDataForComputation,
            CompleteFarmingData,
        ),
    ) -> Self {
        let blockchain_response = value.0;
        let project_data = value.1;
        let farming_data = value.2;

        let balance_of: i64 = StarknetValue::new(blockchain_response[0].clone())
            .resolve("i64")
            .into();
        let releasable_of: i64 = StarknetValue::new(blockchain_response[1].clone())
            .resolve("i64")
            .into();
        let claimable_of: i64 = StarknetValue::new(blockchain_response[2].clone())
            .resolve("i64")
            .into();
        let yielder_deposited: i64 = StarknetValue::new(blockchain_response[3].clone())
            .resolve("i64")
            .into();
        let offseter_deposited: i64 = StarknetValue::new(blockchain_response[4].clone())
            .resolve("i64")
            .into();
        let min_claimable: i64 = StarknetValue::new(blockchain_response[5].clone())
            .resolve("i64")
            .into();

        Self {
            customer_stake: (project_data.unit_price as i64
                * (yielder_deposited + offseter_deposited)
                / project_data.payment_decimals) as f64,
            vesting_to_claim: claimable_of as f64,
            absorption_to_claim: (releasable_of / farming_data.ton_equivalent) as f64,
            undeposited: balance_of - (yielder_deposited + offseter_deposited),
            min_to_claim: min_claimable,
            contracts: ContractsList {
                vester: farming_data.vester_address.unwrap_or_default(),
                yielder: farming_data.yielder_address.unwrap_or_default(),
                offseter: farming_data.offseter_address.unwrap_or_default(),
            },
        }
    }
}
