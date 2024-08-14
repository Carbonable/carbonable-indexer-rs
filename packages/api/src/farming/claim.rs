use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{
    domain::crypto::U256,
    infrastructure::{
        postgres::farming::PostgresFarming,
        starknet::{
            ensure_starknet_wallet, get_starknet_rpc_from_env,
            model::{felt_to_u256, parallelize_blockchain_rpc_calls},
        },
    },
};
use starknet::core::types::FieldElement;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};
use std::sync::Arc;

const MIN_CLAIMABLE: u64 = 10000;

/// Get the claimable amount of a given yielder for a given wallet.
/// * `wallet`: wallet address
/// * `yielder`: yielder address
async fn get_customer_claimable(wallet: &str, yielder: &str) -> Result<U256, ApiError> {
    let provider = Arc::new(get_starknet_rpc_from_env()?);
    let calldata = [(
        yielder.to_owned(),
        "get_claimable_of",
        vec![FieldElement::from_hex_be(wallet).unwrap()],
    )];

    let data = match parallelize_blockchain_rpc_calls(provider, calldata.to_vec()).await {
        Ok(d) => d,
        Err(_) => return Ok(U256::from(0_u64)),
    };
    let claimable = felt_to_u256(data[0].clone()[0]);

    Ok(claimable)
}

/// Claim all yielders for a given wallet having at least `MIN_CLAIMABLE` amount of claimable yield.
/// * `wallet_param`: wallet address
/// * `data`: AppDependencies
pub async fn claim_all(
    wallet_param: web::Path<String>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let mut wallet = wallet_param.into_inner();
    ensure_starknet_wallet(&mut wallet);

    let project_model = PostgresFarming::new(data.db_client_pool.clone());

    let mut projects = project_model.get_project_address_and_slot().await?;
    let mut customer_farm: Vec<String> = Vec::new();
    let min_claimable = U256::from(MIN_CLAIMABLE);
    for p in projects.iter_mut() {
        if let Some(yielder) = &p.yielder {
            let claimable = get_customer_claimable(&wallet, yielder.as_str()).await?;
            if claimable >= min_claimable {
                customer_farm.push(yielder.to_owned());
            }
        }
    }

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: customer_farm,
    }))
}
