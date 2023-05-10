use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, BlockTag},
        HttpTransport, JsonRpcClient,
    },
};
use time::OffsetDateTime;

use crate::infrastructure::{
    flatten,
    postgres::{customer::PostgresCustomer, entity::Snapshot},
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
        felt_to_u256, get_call_function, parallelize_blockchain_rpc_calls, u256_to_felt, ModelError,
    },
    portfolio::{get_balance_of, get_slot_of, get_token_id, get_value_of_token_in_slot},
};

/// Get cumulated value of tokens in slot for customer
pub async fn get_value_of(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    customer_tokens: &[CustomerToken],
    address: String,
    slot: U256,
    wallet: String,
) -> Result<U256, ModelError> {
    let balance = get_balance_of(&provider.clone(), &address.clone(), &wallet.clone()).await?;
    let mut value = U256::zero();
    let token_len = u64::try_from(customer_tokens.len()).unwrap();
    if balance == token_len {
        for t in customer_tokens {
            value += t.value;
        }
        return Ok(value);
    }

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
    customer_tokens: Vec<CustomerToken>,
) -> Result<CustomerGlobalData, ModelError> {
    let value = get_value_of(
        provider.clone(),
        &customer_tokens,
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
        total_deposited: SlotValue::from_blockchain(value, data.value_decimals),
        total_released: Erc20::from_blockchain(
            releaseable_of,
            data.payment_decimals,
            data.payment_symbol.clone(),
        ),
        total_claimable: Mass::<U256>::from_blockchain(claimable_of, data.ton_equivalent),
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
/// TODO: Update APR calculation with latest provision. Everything about vesting was commented out
/// in this PR.
///
fn get_project_current_apr(
    snapshots: Vec<Snapshot>,
    total_value: U256,
) -> Result<ProjectApr, ModelError> {
    if snapshots.is_empty() {
        return Ok(ProjectApr::None);
    }
    let snapshot = match snapshots
        .iter()
        // .filter(|s| s.time < last_vesting.time)
        .last()
    {
        Some(s) => s,
        None => return Err(ModelError::InvalidDataSet("snapshots".to_string())),
    };
    let diff_time = snapshot.time - snapshot.previous_time;
    let apr = U256(crypto_bigint::U256::from_u8(100))
        // * last_vesting.amount
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
    data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    snapshots: Vec<Snapshot>,
    total_value: U256,
) -> Result<UnconnectedFarmingData, ModelError> {
    // times, absorptions, ton_equivalent, unit_price, payment_decimals

    let apr = get_project_current_apr(snapshots, total_value)?;
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
        tvl: Erc20::from_blockchain(
            data.unit_price * (total_offseted + total_yielded),
            farming_data.payment_decimals,
            farming_data.payment_symbol,
        )
        .into(),
        total_removal: Mass::<U256>::from_blockchain(last_absorptions, farming_data.ton_equivalent)
            .into(),
    })
}

/// Get listing data per project computed with customer specific data
pub async fn get_customer_listing_project_data(
    project_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    wallet: &str,
    customer_tokens: Vec<CustomerToken>,
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
        &customer_tokens,
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
    total_value: U256,
    customer_tokens: Vec<CustomerToken>,
) -> Result<CustomerDetailsProjectData, ModelError> {
    let mut customer_details_project_data = CustomerDetailsProjectData::default();

    let apr = get_project_current_apr(snapshots, total_value)?;
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
    ];

    let value_of = get_value_of(
        provider.clone(),
        &customer_tokens,
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
