use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{
    domain::{crypto::U256, Erc721, HumanComprehensibleU256, SlotValue},
    infrastructure::{
        postgres::project::PostgresProject,
        starknet::{
            get_starknet_rpc_from_env,
            model::{felt_to_u256, parallelize_blockchain_rpc_calls, u256_to_felt},
        },
        view_model::launchpad::{CurrentMilestone, ProjectMetadata},
    },
};
use starknet::core::types::FieldElement;
use std::sync::Arc;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

#[derive(Debug, Clone)]
struct ProjectValue {
    total_value: HumanComprehensibleU256<U256>,
    remaining_value: HumanComprehensibleU256<U256>,
    current_value: U256,
}

async fn get_project_value(
    project_address: &str,
    minter_address: &str,
    slot: &U256,
) -> Result<ProjectValue, ApiError> {
    let provider = match get_starknet_rpc_from_env() {
        Ok(p) => p,
        Err(_) => return Err(ApiError::FailedToAcquireSequencerConnection),
    };
    let slot_felt = u256_to_felt(slot);
    let calldata = [
        (
            project_address.to_owned(),
            "get_project_value",
            vec![slot_felt, FieldElement::ZERO],
        ),
        (minter_address.to_owned(), "get_remaining_value", vec![]),
    ];

    let data = parallelize_blockchain_rpc_calls(Arc::new(provider), calldata.to_vec()).await?;

    let total_value = felt_to_u256(data[0].clone()[0]);
    let remaining_value = felt_to_u256(data[1].clone()[0]);

    Ok(ProjectValue {
        total_value: HumanComprehensibleU256::from(SlotValue::from_blockchain(
            total_value,
            6_u64.into(),
        )),
        remaining_value: HumanComprehensibleU256::from(SlotValue::from_blockchain(
            remaining_value,
            6_u64.into(),
        )),
        current_value: total_value - remaining_value,
    })
}

async fn aggregate_current_milestone(
    project_value: &ProjectValue,
    metadata: &serde_json::Value,
) -> Option<CurrentMilestone> {
    let project_metadata: ProjectMetadata = metadata.into();
    let current_ceil = project_metadata
        .milestones
        .iter()
        .filter(|m| {
            // NOTE: multiply by ton equivalent (which is already the case on
            // project_value.current_value) this can also be added in database but I find it
            // easier to do so here
            U256::from(m.ceil * 1000000_u64) > project_value.current_value
        })
        .rev()
        .last();
    let milestone_ceil = current_ceil.map(|m| m.ceil * 1000000).unwrap_or(0);
    let remaining = U256::from(milestone_ceil) - project_value.current_value;

    Some(CurrentMilestone {
        remaining: HumanComprehensibleU256::from(SlotValue::from_blockchain(
            remaining,
            6_u64.into(),
        )),
        milestone_ceil,
        boost: current_ceil.and_then(|m| m.boost.clone()),
        id: current_ceil.map(|m| m.id).unwrap_or(0),
        ha: current_ceil.and_then(|m| m.ha.clone()),
        ton: current_ceil.and_then(|m| m.ton.clone()),
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
        let project_value = get_project_value(
            &p.project.address,
            p.launchpad.minter_contract.address.as_str(),
            &p.project.slot,
        )
        .await?;
        if let Some(mint) = &mut p.mint {
            mint.total_value = Some(project_value.clone().total_value);
            mint.remaining_value = Some(project_value.clone().remaining_value);
        }
        if let Some(m) = &p.project.metadata {
            p.project.current_milestone = aggregate_current_milestone(&project_value, m).await
        }
        return Ok(HttpResponse::Ok().json(ServerResponse::Data { data: p }));
    }

    return Ok(HttpResponse::NotFound().into());
}
