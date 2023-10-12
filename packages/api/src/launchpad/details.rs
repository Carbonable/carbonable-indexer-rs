use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{
    domain::{crypto::U256, Erc721, HumanComprehensibleU256, SlotValue},
    infrastructure::{
        postgres::project::PostgresProject,
        starknet::{
            get_starknet_rpc_from_env,
            model::{felt_to_u256, parallelize_blockchain_rpc_calls, u256_to_felt},
        },
    },
};
use starknet::core::types::FieldElement;
use std::sync::Arc;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

#[derive(Debug)]
struct ProjectValue {
    total_value: HumanComprehensibleU256<U256>,
    remaining_value: HumanComprehensibleU256<U256>,
}

async fn get_project_value(project_address: &str, slot: &U256) -> Result<ProjectValue, ApiError> {
    let provider = match get_starknet_rpc_from_env() {
        Ok(p) => p,
        Err(_) => return Err(ApiError::FailedToAcquireSequencerConnection),
    };
    let slot_felt = u256_to_felt(slot);
    let calldata = [
        (
            project_address.to_owned(),
            "total_value",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (
            project_address.to_owned(),
            "get_project_value",
            vec![slot_felt, FieldElement::ZERO],
        ),
    ];

    let data = parallelize_blockchain_rpc_calls(Arc::new(provider), calldata.to_vec()).await?;

    let total_value = felt_to_u256(data[0].clone()[0]);
    let project_value = felt_to_u256(data[1].clone()[0]);

    Ok(ProjectValue {
        total_value: HumanComprehensibleU256::from(SlotValue::from_blockchain(
            project_value,
            6_u64.into(),
        )),
        remaining_value: HumanComprehensibleU256::from(SlotValue::from_blockchain(
            project_value - total_value,
            6_u64.into(),
        )),
    })
}

pub async fn launchpad_details(
    slug_param: web::Path<String>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let slug = slug_param.into_inner();
    let project_model: PostgresProject<Erc721> = PostgresProject::new(data.db_client_pool.clone());
    let mut project = project_model.get_launchpad_details(&slug).await?;
    if let Some(p) = &mut project {
        let project_value = get_project_value(&p.project.address, &p.project.slot).await?;
        if let Some(mint) = &mut p.mint {
            mint.total_value = Some(project_value.total_value);
            mint.remaining_value = Some(project_value.remaining_value);
        }
        return Ok(HttpResponse::Ok().json(ServerResponse::Data { data: p }));
    }

    return Ok(HttpResponse::NotFound().into());
}
