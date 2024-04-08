use bigdecimal::BigDecimal;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use time::OffsetDateTime;

use crate::infrastructure::{
    flatten,
    view_model::{
        customer::CustomerToken,
        farming::{
            CompleteFarmingData, CustomerDetailsProjectData, CustomerFarm, CustomerGlobalData,
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
};

/// Get cumulated value of tokens in slot for customer
pub async fn get_value_of(
    _provider: Arc<JsonRpcClient<HttpTransport>>,
    _address: String,
    customer_tokens: &mut [CustomerToken],
) -> Result<U256, ModelError> {
    let mut value = U256::zero();

    for token_index in customer_tokens {
        value += token_index.value;
    }

    Ok(value)
}

async fn customer_farming_data(
    provider: Arc<JsonRpcClient<HttpTransport>>,
    wallet: String,
    data: CustomerGlobalDataForComputation,
) -> Result<CustomerGlobalData, ModelError> {
    let yielder_address = match &data.yielder_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let offseter_address = match &data.offseter_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let calldata = [
        (
            offseter_address.to_string(),
            "get_deposited_of",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
        (
            yielder_address.to_string(),
            "get_deposited_of",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
        (
            offseter_address.to_string(),
            "get_claimable_of",
            vec![FieldElement::from_hex_be(&wallet).unwrap()],
        ),
        (
            yielder_address.to_string(),
            "get_claimable_of",
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
) -> Result<DisplayableCustomerGlobalData, ModelError> {
    let mut handles = vec![];
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    for data in addresses.into_iter() {
        if data.yielder_address.is_none() || data.offseter_address.is_none() {
            continue;
        }
        let provider = provider.clone();
        let wallet = wallet.to_string();
        let handle =
            tokio::spawn(async move { customer_farming_data(provider, wallet, data).await });
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
async fn get_project_current_apr(
    yielder_address: &str,
    minter_address: &str,
) -> Result<ProjectApr, ModelError> {
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let calldata = [(
        yielder_address.to_owned(),
        "get_apr",
        vec![FieldElement::from_hex_be(minter_address).unwrap()],
    )];

    let data = match parallelize_blockchain_rpc_calls(provider, calldata.to_vec()).await {
        Ok(d) => d,
        Err(_) => return Ok(ProjectApr::None),
    };
    let num = felt_to_u256(data[0].clone()[0]).to_big_decimal(3);
    let den = felt_to_u256(data[0].clone()[1]).to_big_decimal(3);
    Ok(ProjectApr::Value(apr_from_felt(&num, &den)))
}

fn apr_from_felt(numerator: &BigDecimal, denominator: &BigDecimal) -> BigDecimal {
    if BigDecimal::from(0).eq(denominator) {
        return BigDecimal::from(0);
    }
    ((numerator / denominator) * BigDecimal::from(100)).with_scale(4)
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
) -> Result<UnconnectedFarmingData, ModelError> {
    let yielder_address = match global_data.yielder_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let offseter_address = match global_data.offseter_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };

    let apr = get_project_current_apr(&yielder_address, &global_data.minter_address).await?;
    let status = get_project_status(&farming_data);

    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (offseter_address.to_string(), "get_total_deposited", vec![]),
        (yielder_address.to_string(), "get_total_deposited", vec![]),
        (
            global_data.project_address.to_string(),
            "get_current_absorption",
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
    customer_tokens: &mut [CustomerToken],
    customer_farm: &CustomerFarm,
) -> Result<CustomerListingProjectData, ModelError> {
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let felt_wallet = match FieldElement::from_hex_be(wallet) {
        Ok(w) => w,
        Err(_) => return Err(ModelError::InvalidWalletAddress(wallet.to_owned())),
    };
    let yielder_address = match &project_data.yielder_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let offseter_address = match &project_data.offseter_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let values = [
        (
            yielder_address.to_string(),
            "get_claimable_of",
            vec![felt_wallet],
        ),
        (
            offseter_address.to_string(),
            "get_claimable_of",
            vec![felt_wallet],
        ),
    ];

    let data = parallelize_blockchain_rpc_calls(provider.clone(), values.to_vec()).await?;

    let value_of = get_value_of(
        provider.clone(),
        project_data.project_address.to_string(),
        customer_tokens,
    )
    .await?;

    Ok(CustomerListingProjectData::from((
        data,
        project_data,
        farming_data,
        value_of,
        customer_farm,
    )))
}

/// Customer project data for project with slug
pub async fn get_customer_details_project_data(
    project_data: CustomerGlobalDataForComputation,
    farming_data: CompleteFarmingData,
    wallet: &str,
    customer_tokens: &mut [CustomerToken],
    customer_farm: &CustomerFarm,
) -> Result<CustomerDetailsProjectData, ModelError> {
    let mut customer_details_project_data = CustomerDetailsProjectData::default();

    let yielder_address = match &project_data.yielder_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let offseter_address = match &project_data.offseter_address {
        Some(s) => s.to_owned(),
        None => return Err(ModelError::NotReadyForFarming),
    };
    let apr = match get_project_current_apr(&yielder_address, &project_data.minter_address).await {
        Ok(a) => a,
        Err(_) => ProjectApr::None,
    };
    let mut builder = customer_details_project_data
        .with_contracts(&project_data, &farming_data)
        .with_apr(apr);

    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let values = [
        (
            project_data.project_address.to_string(),
            "get_current_absorption",
            vec![u256_to_felt(&project_data.project_slot), FieldElement::ZERO],
        ),
        (
            offseter_address.to_string(),
            "get_claimable_of",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (
            yielder_address.to_string(),
            "get_claimable_of",
            vec![FieldElement::from_hex_be(wallet).unwrap()],
        ),
        (offseter_address.to_string(), "get_total_deposited", vec![]),
        (yielder_address.to_string(), "get_total_deposited", vec![]),
        // (
        //     project_data.offseter_address.to_string(),
        //     "get_min_claimable",
        //     vec![],
        // ),
    ];

    let value_of = get_value_of(
        provider.clone(),
        project_data.project_address.to_string(),
        customer_tokens,
    )
    .await?;
    let data = parallelize_blockchain_rpc_calls(provider.clone(), values.to_vec()).await?;
    builder = builder.compute_blockchain_data(
        data,
        &farming_data,
        &project_data,
        &value_of,
        customer_tokens,
        customer_farm,
    );

    let customer_details_project_data = builder.build();

    Ok(customer_details_project_data)
}

#[cfg(test)]
mod tests {
    use starknet::core::types::FieldElement;

    use super::apr_from_felt;

    #[test]
    fn test_calculate_apr_works() {
        let num = FieldElement::from_hex_be("0x8b99d8c758")
            .unwrap()
            .to_big_decimal(3);
        let den = FieldElement::from_hex_be("0x4190ab000000")
            .unwrap()
            .to_big_decimal(3);

        let res = apr_from_felt(&num, &den);
        assert_eq!(res.to_string(), "0.8317");
    }
}
