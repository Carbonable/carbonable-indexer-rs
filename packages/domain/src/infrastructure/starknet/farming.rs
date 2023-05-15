use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use time::OffsetDateTime;

use crate::infrastructure::{
    flatten,
    postgres::{
        customer::PostgresCustomer,
        entity::{Provision, Snapshot},
    },
    view_model::{
        customer::CustomerToken,
        farming::{
            CompleteFarmingData, CustomerDetailsProjectData, CustomerGlobalData,
            CustomerGlobalDataForComputation, CustomerListingProjectData, ProjectApr,
            ProjectStatus, UnconnectedFarmingData,
        },
    },
};
use crate::{
    domain::{crypto::U256, Erc20, Mass, SlotValue},
    infrastructure::view_model::farming::DisplayableCustomerGlobalData,
};
use std::sync::Arc;

use super::{
    get_starknet_rpc_from_env,
    model::{
        felt_to_u256, parallelize_blockchain_rpc_calls, u256_to_felt, ModelError, StarknetValue,
        StarknetValueResolver,
    },
    portfolio::{get_balance_of, get_slot_of, get_token_id, get_value_of_token_in_slot},
};

// Bisextiles bitch
const SECONDS_IN_YEAR: u64 = 31557600;

/// Get cumulated value of tokens in slot for customer
pub async fn get_value_of(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: String,
    slot: U256,
    wallet: String,
) -> Result<U256, ModelError> {
    let balance = get_balance_of(&provider.clone(), &address.clone(), &wallet.clone()).await?;
    let mut value = U256::zero();

    for token_index in 0..balance {
        let token_id = get_token_id(&provider, &address, &wallet, &token_index).await?;
        if get_slot_of(&provider, &address, &token_id).await? != slot {
            continue;
        }

        let value_in_slot = get_value_of_token_in_slot(&provider, &address, &token_id).await?;

        value += value_in_slot;
    }
    Ok(value)
}

async fn customer_farming_data(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    wallet: String,
    data: CustomerGlobalDataForComputation,
    _customer_tokens: Vec<CustomerToken>,
) -> Result<CustomerGlobalData, ModelError> {
    let value = get_value_of(
        provider.clone(),
        data.project_address.to_string(),
        data.project_slot,
        wallet.clone(),
    )
    .await?;

    if U256::zero() == value {
        return Ok(CustomerGlobalData::default());
    }
    let calldata = [
        (
            data.offseter_address.to_string(),
            "getDepositedOf",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
        (
            data.yielder_address.to_string(),
            "getDepositedOf",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
        (
            data.offseter_address.to_string(),
            "getClaimableOf",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
        (
            data.yielder_address.to_string(),
            "getClaimableOf",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
    ];
    let blockchain_data =
        parallelize_blockchain_rpc_calls(provider.clone(), calldata.to_vec()).await?;

    let offseter_deposited = felt_to_u256(*blockchain_data[0].clone().first().unwrap());
    let yielder_deposited = felt_to_u256(*blockchain_data[1].clone().first().unwrap());
    let offseter_claimable = felt_to_u256(*blockchain_data[2].clone().first().unwrap());
    let yielder_claimable = felt_to_u256(*blockchain_data[3].clone().first().unwrap());

    let total_deposited_value = offseter_deposited + yielder_deposited;
    Ok(CustomerGlobalData {
        total_deposited_value: SlotValue::from_blockchain(
            total_deposited_value,
            data.value_decimals,
        ),
        total_investment: Erc20::from_blockchain(
            total_deposited_value * data.unit_price,
            data.payment_decimals,
            data.payment_symbol.clone(),
        ),
        total_yielder_claimable: Erc20::from_blockchain(
            yielder_claimable,
            data.payment_decimals,
            data.payment_symbol.clone(),
        ),
        total_offseter_claimable: Mass::<U256>::from_blockchain(
            offseter_claimable,
            data.ton_equivalent,
        ),
    })
}

/// Customer global farming data maps to route :
/// /farming/list/global/{wallet}
pub async fn get_customer_global_farming_data(
    wallet: String,
    addresses: Vec<CustomerGlobalDataForComputation>,
    customer_model: &PostgresCustomer,
) -> Result<DisplayableCustomerGlobalData, ModelError> {
    let mut handles = vec![];
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    for data in addresses.into_iter() {
        let provider = provider.clone();
        let wallet = wallet.to_string();
        let customer_tokens = customer_model
            .get_customer_tokens(&wallet, &data.project_address)
            .await
            .map_err(|_| ModelError::FailedToFetchCustomerTokens)?;
        let handle = tokio::spawn(async move {
            customer_farming_data(provider, wallet, data, customer_tokens).await
        });
        handles.push(flatten(handle));
    }
    let customer_global_data = futures::future::try_join_all(handles).await;
    let aggregated_data = customer_global_data
        .into_iter()
        .flatten()
        .fold(CustomerGlobalData::default(), |acc, e| acc.merge(e));
    Ok(aggregated_data.into())
}

/// Calculates project APR base on :
/// Explanation :
/// APR = ratio / dt
/// ratio = 100 * amount_$_provision / total_$_project
/// dt = (time_snapshot(n) - time_snapshot(n-1)) / nb_seconds_per_year
///
/// * `snapshots` - [Vec<Snapshots>] - Yielder snapshots
/// * `provisions` - [Vec<Provision>] - Yielder provision - an admin deposit cashflow on yielder
/// represents project carbon credit sale
/// * `total_value` - [U256] - Total value of a slot * unit_price of minter
///
fn get_project_current_apr(
    snapshots: Vec<Snapshot>,
    provisions: Vec<Provision>,
    total_value: U256,
) -> Result<ProjectApr, ModelError> {
    if snapshots.is_empty() {
        return Ok(ProjectApr::None);
    }
    if provisions.is_empty() {
        return Ok(ProjectApr::None);
    }
    let provision = provisions
        .last()
        .expect("should have at least one provision");

    let snapshot = match snapshots.iter().filter(|s| s.time < provision.time).last() {
        Some(s) => s,
        None => return Err(ModelError::InvalidDataSet("snapshots".to_string())),
    };

    let diff_time = snapshot.time - snapshot.previous_time;

    let numerator = (U256(crypto_bigint::U256::from_u8(100))
        * provision.amount
        * snapshot.project_absorption
        * U256(crypto_bigint::U256::from_u64(SECONDS_IN_YEAR)))
        * U256(crypto_bigint::U256::from_u32(1000));
    let denominator = total_value * snapshot.yielder_absorption * (U256::from(diff_time));

    let apr = numerator / denominator;

    Ok(ProjectApr::Value(apr.to_big_decimal(3)))
}

/// Get project status
fn get_project_status(farming_data: &CompleteFarmingData) -> ProjectStatus {
    if farming_data.times.is_empty() && farming_data.absorptions.is_empty()
        || (farming_data.yielder_address.is_none() || farming_data.offseter_address.is_none())
    {
        return ProjectStatus::Upcoming;
    }
    if OffsetDateTime::now_utc() > farming_data.times.last().unwrap().assume_utc() {
        return ProjectStatus::Ended;
    }

    ProjectStatus::Live
}

/// Data required for details to project on farming index page
pub async fn get_unconnected_project_data(
    global_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    snapshots: Vec<Snapshot>,
    provisions: Vec<Provision>,
    total_value: U256,
) -> Result<UnconnectedFarmingData, ModelError> {
    let apr = get_project_current_apr(snapshots, provisions, total_value)?;
    let status = get_project_status(&farming_data);

    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (
            global_data.offseter_address.to_string(),
            "getTotalDeposited",
            vec![],
        ),
        (
            global_data.yielder_address.to_string(),
            "getTotalDeposited",
            vec![],
        ),
        (
            global_data.project_address.to_string(),
            "getCurrentAbsorption",
            vec![u256_to_felt(&global_data.slot), FieldElement::ZERO],
        ),
    ];

    let data = parallelize_blockchain_rpc_calls(provider.clone(), values.to_vec()).await?;
    let total_offseted: U256 = StarknetValue::new(data[0].clone()).resolve("u256").into();
    let total_yielded: U256 = StarknetValue::new(data[1].clone()).resolve("u256").into();
    let current_absorption: U256 = StarknetValue::new(data[2].clone()).resolve("u256").into();

    Ok(UnconnectedFarmingData {
        apr,
        status,
        tvl: Erc20::from_blockchain(
            global_data.unit_price * (total_offseted + total_yielded),
            farming_data.payment_decimals,
            farming_data.payment_symbol,
        )
        .into(),
        total_removal: Mass::<U256>::from_blockchain(
            current_absorption,
            farming_data.ton_equivalent,
        )
        .into(),
    })
}

/// Get listing data per project computed with customer specific data
pub async fn get_customer_listing_project_data(
    project_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    wallet: &str,
    _customer_tokens: Vec<CustomerToken>,
) -> Result<CustomerListingProjectData, ModelError> {
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (
            project_data.yielder_address.to_string(),
            "getClaimableOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getClaimableOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.yielder_address.to_string(),
            "getDepositedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getDepositedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getMinClaimable",
            vec![],
        ),
    ];

    let data = parallelize_blockchain_rpc_calls(provider.clone(), values.to_vec()).await?;

    let value_of = get_value_of(
        provider.clone(),
        project_data.project_address.to_string(),
        project_data.project_slot,
        wallet.to_string(),
    )
    .await?;

    Ok(CustomerListingProjectData::from((
        data,
        project_data,
        farming_data,
        value_of,
    )))
}

/// Customer project data for project with slug
pub async fn get_customer_details_project_data(
    project_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    wallet: &str,
    snapshots: Vec<Snapshot>,
    provisions: Vec<Provision>,
    total_value: U256,
    customer_tokens: Vec<CustomerToken>,
) -> Result<CustomerDetailsProjectData, ModelError> {
    let mut customer_details_project_data = CustomerDetailsProjectData::default();

    let apr = get_project_current_apr(snapshots, provisions, total_value)?;
    let mut builder = customer_details_project_data
        .with_contracts(&project_data, &farming_data)
        .with_apr(apr);

    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (
            project_data.project_address.to_string(),
            "getCurrentAbsorption",
            vec![u256_to_felt(&project_data.project_slot), FieldElement::ZERO],
        ),
        (
            project_data.offseter_address.to_string(),
            "getDepositedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.yielder_address.to_string(),
            "getDepositedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getClaimableOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.yielder_address.to_string(),
            "getClaimableOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getClaimedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.yielder_address.to_string(),
            "getClaimedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getTotalDeposited",
            vec![],
        ),
        (
            project_data.yielder_address.to_string(),
            "getTotalDeposited",
            vec![],
        ),
        (
            project_data.offseter_address.to_string(),
            "getMinClaimable",
            vec![],
        ),
    ];

    let value_of = get_value_of(
        provider.clone(),
        project_data.project_address.to_string(),
        project_data.project_slot,
        wallet.to_string(),
    )
    .await?;
    let data = parallelize_blockchain_rpc_calls(provider.clone(), values.to_vec()).await?;
    builder = builder.compute_blockchain_data(
        data,
        &farming_data,
        &project_data,
        &value_of,
        customer_tokens,
    );

    let customer_details_project_data = builder.build();

    Ok(customer_details_project_data)
}
