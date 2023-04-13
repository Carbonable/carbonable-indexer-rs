use serde::{Deserialize, Serialize};

use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{
    domain::{crypto::U256, project::ProjectError},
    infrastructure::{
        flatten,
        postgres::{entity::ErcImplementation, project::PostgresProject},
        starknet::portfolio::{load_erc_3525_portfolio, load_erc_721_portfolio},
        view_model::portfolio::{ProjectWithMinterAndPaymentViewModel, ProjectWithTokens},
    },
};

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

async fn aggregate_721_tokens(
    project: ProjectWithMinterAndPaymentViewModel,
    wallet: String,
) -> Result<Option<ProjectWithTokens>, ProjectError> {
    let tokens = load_erc_721_portfolio(&project.address, &wallet).await?;
    if tokens.is_empty() {
        return Ok(None);
    }
    let total_amount = total_amount(
        project.unit_price,
        project.payment_decimals,
        U256::from(tokens.len()),
    );

    let project = ProjectWithTokens::Erc721 {
        id: project.id,
        name: project.name,
        address: project.address,
        minter_address: project.minter_address,
        tokens,
        total_amount,
    };

    Ok(Some(project))
}
async fn aggregate_3525_tokens(
    project: ProjectWithMinterAndPaymentViewModel,
    wallet: String,
) -> Result<Option<ProjectWithTokens>, ProjectError> {
    let slot = project.slot.expect("erc3525 should have slot");

    let tokens = load_erc_3525_portfolio(&project, &project.address, &wallet, &slot).await?;
    if tokens.is_empty() {
        return Ok(None);
    }

    let value = tokens
        .iter()
        .flatten()
        .fold(U256::zero(), |acc, e| acc + e.value);
    let total_amount = total_amount(project.unit_price, project.payment_decimals, value);

    let project = ProjectWithTokens::Erc3525 {
        id: project.id,
        name: project.name,
        address: project.address,
        minter_address: project.minter_address,
        tokens: tokens.into_iter().flatten().collect(),
        total_amount,
    };

    Ok(Some(project))
}

fn total_amount(unit_price: U256, payment_decimals: U256, amount: U256) -> U256 {
    // TODO: replace f64 with bigdecimal
    unit_price * amount
}

async fn aggregate_tokens_with_project(
    projects_data: Vec<ProjectWithMinterAndPaymentViewModel>,
    wallet: String,
) -> Result<Vec<Option<ProjectWithTokens>>, ApiError> {
    let mut handles = vec![];
    for project in projects_data.into_iter() {
        let wallet_address = wallet.clone();
        let handle = match &project.erc_implementation {
            ErcImplementation::Enum => {
                return Err(ApiError::ProjectError(
                    ProjectError::InvalidErcImplementation,
                ));
            }
            ErcImplementation::Erc721 => {
                tokio::spawn(async move { aggregate_721_tokens(project, wallet_address).await })
            }
            ErcImplementation::Erc3525 => {
                tokio::spawn(async move { aggregate_3525_tokens(project, wallet_address).await })
            }
        };
        handles.push(flatten(handle));
    }

    match futures::future::try_join_all(handles).await {
        Ok(data) => Ok(data),
        Err(e) => Err(ApiError::ProjectError(e)),
    }
}

#[derive(Serialize)]
pub struct Global {
    total: U256,
}

#[derive(Serialize)]
pub struct GetByWalletResponse {
    global: Global,
    projects: Vec<ProjectWithTokens>,
    badges: Vec<String>,
}
pub async fn get_by_wallet(
    data: web::Data<AppDependencies>,
    wallet_param: web::Path<String>,
) -> Result<impl Responder, ApiError> {
    let wallet = wallet_param.into_inner();

    let project_model = PostgresProject::new(data.db_client_pool.clone());
    let projects_data = project_model
        .find_projects_with_minter_and_payment()
        .await?;
    let projects = aggregate_tokens_with_project(projects_data, wallet).await?;
    let filtered_projects: Vec<ProjectWithTokens> = projects
        .into_iter()
        .filter(|p| p.is_some())
        .flatten()
        .collect();

    let total = filtered_projects
        .iter()
        .fold(U256::zero(), |acc, e| acc + e.get_total_amount());

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: GetByWalletResponse {
            global: Global { total },
            projects: filtered_projects,
            badges: vec![],
        },
    }))
}