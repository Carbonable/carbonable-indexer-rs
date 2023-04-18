use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::{
    domain::{crypto::U256, Erc20, HumanComprehensibleU256, Mass, SlotValue},
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
    pub payment_symbol: String,
    pub project_slot: U256,
    pub project_address: String,
    pub value_decimals: U256,
    pub ton_equivalent: U256,
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
            payment_symbol: value.get(3),
            project_slot: value.get(4),
            project_address: value.get(5),
            value_decimals: value.get(6),
            ton_equivalent: value.get(7),
            yielder_address: value.get(8),
            offseter_address: value.get(9),
            vester_address: value.get(10),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct CustomerGlobalData {
    // slot value
    pub total_deposited: SlotValue,
    // erc 20
    pub total_released: Erc20,
    // mass in gram
    pub total_claimable: Mass<U256>,
}
#[derive(Debug, Default, Serialize)]
pub struct DisplayableCustomerGlobalData {
    pub total_deposited: HumanComprehensibleU256<U256>,
    pub total_released: HumanComprehensibleU256<U256>,
    pub total_claimable: HumanComprehensibleU256<U256>,
}

impl From<CustomerGlobalData> for DisplayableCustomerGlobalData {
    fn from(value: CustomerGlobalData) -> Self {
        Self {
            total_deposited: value.total_deposited.into(),
            total_released: value.total_released.into(),
            total_claimable: value.total_claimable.into(),
        }
    }
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
    pub value_decimals: U256,
    pub payment_decimals: U256,
    pub payment_symbol: String,
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
            value_decimals: value.get(5),
            payment_decimals: value.get(6),
            payment_symbol: value.get(7),
            offseter_address: value.get(8),
            yielder_id: value.get(9),
            yielder_address: value.get(10),
            vester_address: value.get(11),
            minter_id: value.get(12),
            total_supply: value.get(13),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct UnconnectedFarmingData {
    #[serde(flatten)]
    pub apr: ProjectApr,
    #[serde(flatten)]
    pub status: ProjectStatus,
    pub tvl: HumanComprehensibleU256<U256>,
    pub total_removal: HumanComprehensibleU256<U256>,
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
    pub customer_stake: HumanComprehensibleU256<U256>,
    pub payment_decimals: u32,
    pub ton_equivalent: U256,
    pub vesting_to_claim: HumanComprehensibleU256<U256>,
    pub absorption_to_claim: HumanComprehensibleU256<U256>,
    pub undeposited: HumanComprehensibleU256<U256>,
    /// min_to_claim in kg
    pub min_to_claim: HumanComprehensibleU256<U256>,
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
            customer_stake: SlotValue::from_blockchain(
                yielder_deposited + offseter_deposited,
                farming_data.value_decimals,
            )
            .into(),
            payment_decimals: project_data.payment_decimals.into(),
            ton_equivalent: farming_data.ton_equivalent,
            vesting_to_claim: Erc20::from_blockchain(
                claimable_of,
                farming_data.payment_decimals,
                farming_data.payment_symbol,
            )
            .into(),
            absorption_to_claim: Mass::<U256>::from_blockchain(
                releasable_of,
                farming_data.ton_equivalent,
            )
            .into(),
            undeposited: SlotValue::from_blockchain(
                balance_of - (yielder_deposited + offseter_deposited),
                farming_data.value_decimals,
            )
            .into(),
            min_to_claim: Mass::<U256>::from_blockchain(min_claimable, farming_data.ton_equivalent)
                .into(),
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
    total_removal: HumanComprehensibleU256<U256>,
    tvl: HumanComprehensibleU256<U256>,
    #[serde(flatten)]
    current_apr: ProjectApr,
    total_yielded: HumanComprehensibleU256<U256>,
    total_offseted: HumanComprehensibleU256<U256>,
}

#[derive(Default, Clone, Debug, Serialize)]
pub struct PoolLiquidity<T> {
    total: HumanComprehensibleU256<T>,
    available: HumanComprehensibleU256<T>,
}

#[derive(Default, Clone, Debug, Serialize)]
pub struct CarbonCredits {
    generated_credits: HumanComprehensibleU256<BigDecimal>,
    to_be_generated: HumanComprehensibleU256<BigDecimal>,
    r#yield: PoolLiquidity<Erc20>,
    offset: PoolLiquidity<Mass<U256>>,
}
#[derive(Default, Clone, Debug, Serialize)]
pub struct Allocation {
    total: HumanComprehensibleU256<U256>,
    r#yield: HumanComprehensibleU256<U256>,
    offseted: HumanComprehensibleU256<U256>,
    undeposited: HumanComprehensibleU256<U256>,
}
#[derive(Default, Clone, Debug, Serialize)]
pub struct CustomerDetailsProjectData {
    overview: Overview,
    carbon_credits: CarbonCredits,
    allocation: Allocation,
    contracts: ContractsList,
    payment_decimals: u32,
    ton_equivalent: BigDecimal,
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
        farming_data: &CustomerGlobalDataForComputation,
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

        self.overview.total_removal = Mass::<U256>::from_blockchain(
            project.final_absorption() - current_absorption,
            project.ton_equivalent,
        )
        .into();
        self.overview.total_yielded =
            SlotValue::from_blockchain(yielder_total_deposited, project.value_decimals).into();
        self.overview.total_offseted =
            SlotValue::from_blockchain(offseter_total_deposited, project.value_decimals).into();

        self.overview.tvl = Erc20::from_blockchain(
            farming_data.unit_price * (offseter_deposited_of + yielder_total_deposited),
            farming_data.payment_decimals,
            farming_data.payment_symbol.clone(),
        )
        .into();

        self.carbon_credits.generated_credits = Mass::<BigDecimal>::from_blockchain(
            current_absorption.to_big_decimal(0) * balance_of.to_big_decimal(0)
                / total_supply.to_big_decimal(0),
        )
        .into();
        self.carbon_credits.to_be_generated = Mass::<BigDecimal>::from_blockchain(
            (project.final_absorption().to_big_decimal(0) - current_absorption.to_big_decimal(0))
                * balance_of.to_big_decimal(0)
                / total_supply.to_big_decimal(0),
        )
        .into();

        self.carbon_credits.r#yield = PoolLiquidity {
            available: Erc20::from_blockchain(
                releasable_of,
                project.payment_decimals,
                project.payment_symbol.clone(),
            )
            .into(),
            total: Erc20::from_blockchain(
                released_of,
                project.payment_decimals,
                project.payment_symbol.clone(),
            )
            .into(),
        };
        self.carbon_credits.offset = PoolLiquidity {
            available: Mass::<U256>::from_blockchain(claimable_of, project.ton_equivalent).into(),
            total: Mass::<U256>::from_blockchain(claimed_of, project.ton_equivalent).into(),
        };

        self.allocation.total =
            SlotValue::from_blockchain(balance_of, project.value_decimals).into();
        self.allocation.r#yield = Erc20::from_blockchain(
            yielder_deposited_of,
            project.payment_decimals,
            project.payment_symbol.clone(),
        )
        .into();
        self.allocation.offseted =
            Mass::<U256>::from_blockchain(offseter_deposited_of, project.ton_equivalent).into();
        self.allocation.undeposited = SlotValue::from_blockchain(
            balance_of - (yielder_deposited_of + offseter_deposited_of),
            project.value_decimals,
        )
        .into();

        self.ton_equivalent = project.ton_equivalent.to_big_decimal(0);
        self.payment_decimals = project.payment_decimals.into();

        self
    }

    pub fn build(&self) -> Self {
        self.clone()
    }
}
