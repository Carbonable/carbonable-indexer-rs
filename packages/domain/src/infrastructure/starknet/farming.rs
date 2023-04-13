use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, BlockTag},
        HttpTransport, JsonRpcClient,
    },
};
use time::OffsetDateTime;

use crate::domain::crypto::U256;
use crate::infrastructure::{
    flatten,
    postgres::entity::{Snapshot, Vesting},
    view_model::farming::{
        CompleteFarmingData, CustomerDetailsProjectData, CustomerGlobalData,
        CustomerGlobalDataForComputation, CustomerListingProjectData, ProjectApr, ProjectStatus,
        UnconnectedFarmingData,
    },
};
use std::sync::Arc;

use super::{
    get_starknet_rpc_from_env,
    model::{
        felt_to_u256, get_call_function, parallelize_blockchain_rpc_calls, u256_to_felt, ModelError,
    },
    portfolio::{get_balance_of, get_slot_of, get_token_id, get_value_of_token_in_slot},
};

/// Get cumulated value of tokens in slot for customer
async fn get_value_of(
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

        value += value_in_slot.into();
    }
    Ok(value)
}
/// Get claimable amount for slot in customer
async fn get_claimable_of(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: String,
    wallet: String,
) -> Result<U256, ModelError> {
    let response = provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address.as_str()).unwrap(),
                "getClaimableOf",
                vec![FieldElement::from_hex_be(wallet.as_str()).unwrap()],
            ),
            &BlockId::Tag(BlockTag::Latest),
        )
        .await?;
    Ok(felt_to_u256(*response.first().unwrap()))
}
/// Get releasable amount for slot in customer
async fn get_releasable_of(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: String,
    wallet: String,
) -> Result<U256, ModelError> {
    let response = provider
        .call(
            get_call_function(
                &FieldElement::from_hex_be(address.as_str()).unwrap(),
                "getReleasableOf",
                vec![FieldElement::from_hex_be(wallet.as_str()).unwrap()],
            ),
            &BlockId::Tag(BlockTag::Latest),
        )
        .await?;
    Ok(felt_to_u256(*response.first().unwrap()))
}

async fn customer_farming_data(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    wallet: String,
    data: CustomerGlobalDataForComputation,
) -> Result<CustomerGlobalData, ModelError> {
    let value = get_value_of(
        provider.clone(),
        data.project_address.to_string(),
        data.project_slot,
        wallet.clone(),
    )
    .await?;
    let releaseable_of = get_releasable_of(
        provider.clone(),
        data.offseter_address.to_string(),
        wallet.clone(),
    )
    .await?;
    let claimable_of = get_claimable_of(
        provider.clone(),
        data.yielder_address.to_string(),
        wallet.to_string(),
    )
    .await?;

    Ok(CustomerGlobalData {
        total_deposited: value * (data.unit_price),
        total_released: releaseable_of,
        total_claimable: claimable_of,
    })
}

/// Customer global farming data maps to route :
/// /farming/list/global/{wallet}
pub async fn get_customer_global_farming_data(
    wallet: String,
    addresses: Vec<CustomerGlobalDataForComputation>,
) -> Result<CustomerGlobalData, ModelError> {
    let mut handles = vec![];
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    for data in addresses.into_iter() {
        let provider = provider.clone();
        let wallet = wallet.to_string();
        let handle =
            tokio::spawn(async move { customer_farming_data(provider, wallet, data).await });
        handles.push(flatten(handle));
    }
    let customer_global_data = futures::future::try_join_all(handles).await;
    Ok(customer_global_data
        .into_iter()
        .flatten()
        .fold(CustomerGlobalData::default(), |acc, e| acc.merge(e)))
}

/// Calculates project APR base on :
///
fn get_project_current_apr(
    snapshots: Vec<Snapshot>,
    vestings: Vec<Vesting>,
    total_value: U256,
) -> Result<ProjectApr, ModelError> {
    if snapshots.is_empty() || vestings.is_empty() {
        return Ok(ProjectApr::None);
    }
    let last_vesting = match vestings.last() {
        Some(v) => v,
        None => return Err(ModelError::InvalidDataSet("vestings".to_string())),
    };
    let snapshot = match snapshots
        .iter()
        .filter(|s| s.time < last_vesting.time)
        .last()
    {
        Some(s) => s,
        None => return Err(ModelError::InvalidDataSet("snapshots".to_string())),
    };
    let diff_time = snapshot.time - snapshot.previous_time;
    let apr = U256(crypto_bigint::U256::from_u8(100))
        * last_vesting.amount
        * (U256(crypto_bigint::U256::from_u16(365))
            * U256(crypto_bigint::U256::from_u8(24))
            * U256(crypto_bigint::U256::from_u32(3600)))
        / U256::from(diff_time)
        / total_value;

    Ok(ProjectApr::Value(apr))
}

/// Get project status
fn get_project_status(farming_data: &CompleteFarmingData) -> ProjectStatus {
    if farming_data.times.is_empty() && farming_data.absorptions.is_empty()
        || (farming_data.yielder_address.is_none()
            || farming_data.offseter_address.is_none()
            || farming_data.vester_address.is_none())
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
    data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    snapshots: Vec<Snapshot>,
    vestings: Vec<Vesting>,
    total_value: U256,
) -> Result<UnconnectedFarmingData, ModelError> {
    // times, absorptions, ton_equivalent, unit_price, payment_decimals

    let apr = get_project_current_apr(snapshots, vestings, total_value)?;
    let status = get_project_status(&farming_data);

    let provider = Arc::new(get_starknet_rpc_from_env()?);

    let total_offseted = felt_to_u256(
        *provider
            .call(
                get_call_function(
                    &FieldElement::from_hex_be(data.offseter_address.as_str()).unwrap(),
                    "getTotalDeposited",
                    vec![],
                ),
                &BlockId::Tag(BlockTag::Latest),
            )
            .await?
            .first()
            .unwrap(),
    );
    let total_yielded = felt_to_u256(
        *provider
            .call(
                get_call_function(
                    &FieldElement::from_hex_be(data.yielder_address.as_str()).unwrap(),
                    "getTotalDeposited",
                    vec![],
                ),
                &BlockId::Tag(BlockTag::Latest),
            )
            .await?
            .first()
            .unwrap(),
    );
    // Asserted in controller that absorptions are not empty
    let last_absorptions = farming_data.final_absorption();

    Ok(UnconnectedFarmingData {
        apr,
        status,
        tvl: (data.unit_price * (total_yielded + total_yielded) / data.payment_decimals),
        total_removal: last_absorptions / farming_data.ton_equivalent,
    })
}

/// Get listing data per project computed with customer specific data
pub async fn get_customer_listing_project_data(
    project_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    wallet: &str,
) -> Result<CustomerListingProjectData, ModelError> {
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (
            project_data.project_address.to_string(),
            "balanceOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.vester_address.to_string(),
            "releasableOf",
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

    Ok(CustomerListingProjectData::from((
        data,
        project_data,
        farming_data,
    )))
}

/// Customer project data for project with slug
pub async fn get_customer_details_project_data(
    project_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    wallet: &str,
    snapshots: Vec<Snapshot>,
    vestings: Vec<Vesting>,
    total_value: U256,
) -> Result<CustomerDetailsProjectData, ModelError> {
    let mut customer_details_project_data = CustomerDetailsProjectData::default();

    let apr = get_project_current_apr(snapshots, vestings, total_value)?;
    let mut builder = customer_details_project_data
        .with_contracts(
            &project_data.vester_address,
            &project_data.yielder_address,
            &project_data.offseter_address,
        )
        .with_apr(apr);

    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (
            project_data.project_address.to_string(),
            "balanceOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
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
            project_data.vester_address.to_string(),
            "releasableOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.offseter_address.to_string(),
            "getClaimedOf",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            project_data.vester_address.to_string(),
            "releasedOf",
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
    ];

    let data = parallelize_blockchain_rpc_calls(provider.clone(), values.to_vec()).await?;
    builder = builder.compute_blockchain_data(data, &farming_data);

    let customer_details_project_data = builder.build();

    Ok(customer_details_project_data)
}