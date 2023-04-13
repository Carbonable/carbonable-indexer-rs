use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{
    domain::{crypto::U256, project::format_ton},
    infrastructure::starknet::model::{StarknetValue, StarknetValueResolver},
};

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
    pub unit_price: U256,
    pub payment_decimals: U256,
    pub project_slot: U256,
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

#[derive(Debug, Default, Serialize)]
pub struct CustomerGlobalData {
    pub total_deposited: U256,
    pub total_released: U256,
    pub total_claimable: U256,
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
    pub absorptions: Vec<U256>,
    pub ton_equivalent: U256,
    pub payment_decimals: U256,
    pub offseter_address: Option<String>,
    pub yielder_id: Option<Uuid>,
    pub yielder_address: Option<String>,
    pub vester_address: Option<String>,
    pub minter_id: Option<Uuid>,
    pub total_supply: Option<U256>,
}
impl CompleteFarmingData {
    pub fn final_absorption(&self) -> U256 {
        *self.absorptions.last().unwrap_or(&U256::zero())
    }
}

impl From<tokio_postgres::Row> for CompleteFarmingData {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get(0),
            address: value.get(1),
            times: value.get(2),
            absorptions: value.get(3),
            ton_equivalent: value.get(4),
            payment_decimals: value.get(5),
            offseter_address: value.get(6),
            yielder_id: value.get(7),
            yielder_address: value.get(8),
            vester_address: value.get(9),
            minter_id: value.get(10),
            total_supply: value.get(11),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct UnconnectedFarmingData {
    #[serde(flatten)]
    pub apr: ProjectApr,
    #[serde(flatten)]
    pub status: ProjectStatus,
    pub tvl: U256,
    pub total_removal: U256,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(tag = "apr")]
pub enum ProjectApr {
    #[default]
    #[serde(rename = "n/a")]
    None,
    Value(U256),
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ProjectStatus {
    #[default]
    Upcoming,
    Ended,
    Live,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ContractsList {
    pub vester: String,
    pub yielder: String,
    pub offseter: String,
}

#[derive(Default, Debug, Serialize)]
pub struct CustomerListingProjectData {
    pub customer_stake: U256,
    pub payment_decimals: U256,
    pub ton_equivalent: U256,
    pub vesting_to_claim: U256,
    pub absorption_to_claim: U256,
    pub undeposited: U256,
    /// min_to_claim in kg
    pub min_to_claim: U256,
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

        let balance_of: U256 = StarknetValue::new(blockchain_response[0].clone())
            .resolve("u256")
            .into();
        let releasable_of: U256 = StarknetValue::new(blockchain_response[1].clone())
            .resolve("u256")
            .into();
        let claimable_of: U256 = StarknetValue::new(blockchain_response[2].clone())
            .resolve("u256")
            .into();
        let yielder_deposited: U256 = StarknetValue::new(blockchain_response[3].clone())
            .resolve("u256")
            .into();
        let offseter_deposited: U256 = StarknetValue::new(blockchain_response[4].clone())
            .resolve("u256")
            .into();
        let min_claimable: U256 = StarknetValue::new(blockchain_response[5].clone())
            .resolve("u256")
            .into();

        Self {
            customer_stake: project_data.unit_price * (yielder_deposited + offseter_deposited),
            payment_decimals: project_data.payment_decimals,
            ton_equivalent: farming_data.ton_equivalent,
            vesting_to_claim: claimable_of,
            absorption_to_claim: (releasable_of / farming_data.ton_equivalent),
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

#[derive(Default, Clone, Debug, Serialize)]
pub struct Overview {
    total_removal: U256,
    tvl: U256,
    current_apr: ProjectApr,
    total_yielded: U256,
    total_offseted: U256,
}
#[derive(Default, Clone, Debug, Serialize)]
pub struct PoolLiquidity {
    total: U256,
    available: U256,
}
#[derive(Default, Clone, Debug, Serialize)]
pub struct CarbonCredits {
    generated_credits: U256,
    to_be_generated: U256,
    r#yield: PoolLiquidity,
    offset: PoolLiquidity,
}
#[derive(Default, Clone, Debug, Serialize)]
pub struct Allocation {
    total: U256,
    r#yield: U256,
    offseted: U256,
    undeposited: U256,
}
#[derive(Default, Clone, Debug, Serialize)]
pub struct CustomerDetailsProjectData {
    overview: Overview,
    carbon_credits: CarbonCredits,
    allocation: Allocation,
    contracts: ContractsList,
    payment_decimals: U256,
    ton_equivalent: U256,
}

impl CustomerDetailsProjectData {
    pub fn with_contracts(
        &mut self,
        vester_address: &str,
        yielder_address: &str,
        offseter_address: &str,
    ) -> &mut Self {
        self.contracts = ContractsList {
            vester: String::from(vester_address),
            yielder: String::from(yielder_address),
            offseter: String::from(offseter_address),
        };
        self
    }

    pub fn with_apr(&mut self, apr: ProjectApr) -> &mut Self {
        self.overview.current_apr = apr;
        self
    }

    pub fn compute_blockchain_data(
        &mut self,
        data: Vec<Vec<FieldElement>>,
        project: &CompleteFarmingData,
    ) -> &mut Self {
        let balance_of: U256 = StarknetValue::new(data[0].clone()).resolve("u256").into();
        let current_absorption: U256 = StarknetValue::new(data[1].clone()).resolve("u256").into();
        let offseter_deposited_of: U256 =
            StarknetValue::new(data[2].clone()).resolve("u256").into();
        let yielder_deposited_of: U256 = StarknetValue::new(data[3].clone()).resolve("u256").into();
        let claimable_of: U256 = StarknetValue::new(data[4].clone()).resolve("u256").into();
        let releasable_of: U256 = StarknetValue::new(data[5].clone()).resolve("u256").into();
        let claimed_of: U256 = StarknetValue::new(data[6].clone()).resolve("u256").into();
        let released_of: U256 = StarknetValue::new(data[7].clone()).resolve("u256").into();
        let offseter_total_deposited: U256 =
            StarknetValue::new(data[8].clone()).resolve("u256").into();
        let yielder_total_deposited: U256 =
            StarknetValue::new(data[9].clone()).resolve("u256").into();
        let total_supply: U256 = project
            .total_supply
            .unwrap_or(U256::from(crypto_bigint::U256::from_u8(0)));

        self.overview.total_removal = project.final_absorption() - current_absorption;
        self.overview.total_yielded = yielder_total_deposited;
        self.overview.total_offseted = offseter_total_deposited;

        self.carbon_credits.generated_credits = (current_absorption / total_supply) * balance_of;
        self.carbon_credits.to_be_generated =
            ((project.final_absorption() - current_absorption) / total_supply) * balance_of;
        self.carbon_credits.r#yield = PoolLiquidity {
            available: releasable_of,
            total: released_of,
        };
        self.carbon_credits.offset = PoolLiquidity {
            available: claimable_of,
            total: claimed_of,
        };

        self.allocation.total = balance_of;
        self.allocation.r#yield = yielder_deposited_of;
        self.allocation.offseted = offseter_deposited_of;
        self.allocation.undeposited = balance_of - (yielder_deposited_of + offseter_deposited_of);

        self.ton_equivalent = project.ton_equivalent;
        self.payment_decimals = project.payment_decimals;

        self
    }

    pub fn build(&self) -> Self {
        self.clone()
    }
}
