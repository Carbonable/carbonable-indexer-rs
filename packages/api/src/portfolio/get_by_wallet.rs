use reqwest::Client;
use serde::Serialize;

use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{
    domain::{crypto::U256, project::ProjectError, Erc721, SlotValue},
    infrastructure::{
        flatten,
        postgres::{
            customer::PostgresCustomer, entity::ErcImplementation, farming::PostgresFarming,
            project::PostgresProject,
        },
        starknet::{
            ensure_starknet_wallet,
            portfolio::{load_erc_3525_portfolio, load_erc_721_portfolio},
        },
        view_model::{
            customer::CustomerToken,
            portfolio::{PortfolioAbi, ProjectWithMinterAndPaymentViewModel, ProjectWithTokens},
        },
    },
};
use std::sync::Arc;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

async fn aggregate_image_from_slot_uri(slot_uri: &str) -> serde_json::Value {
    let client = Client::new();
    if slot_uri.starts_with("data:application/json") {
        let metadata: serde_json::Value =
            serde_json::from_str(slot_uri.replace("data:application/json,", "").as_str())
                .expect("failed to parse json");
        return metadata["image"].clone();
    }
    let uri = slot_uri.replace("\"", "");
    let data: serde_json::Value = client
        .get(uri)
        .send()
        .await
        .expect("failed to query slot_uri")
        .json()
        .await
        .expect("failed to parse json");
    return data["image"].clone();
}

async fn aggregate_721_tokens(
    project: ProjectWithMinterAndPaymentViewModel,
    wallet: String,
) -> Result<Option<ProjectWithTokens>, ProjectError> {
    let tokens = load_erc_721_portfolio(&project, &wallet).await?;
    if tokens.is_empty() {
        return Ok(None);
    }
    let total_amount = total_amount(
        project.unit_price,
        project.payment_decimals,
        U256::from(tokens.len()),
    );

    let image = match &project.slot_uri {
        Some(uri) => aggregate_image_from_slot_uri(&uri.as_str()).await,
        None => serde_json::Value::String("<deprecated>".to_owned()),
    };

    let project = ProjectWithTokens::Erc721 {
        id: project.id,
        name: project.name,
        address: project.address,
        minter_address: project.minter_address,
        tokens,
        total_amount,
        abi: PortfolioAbi {
            project: project.abi,
            minter: project.minter_abi,
        },
        image,
    };

    Ok(Some(project))
}
async fn aggregate_3525_tokens(
    farming_model: Arc<PostgresFarming>,
    project: ProjectWithMinterAndPaymentViewModel,
    wallet: String,
    customer_tokens: Vec<CustomerToken>,
) -> Result<Option<ProjectWithTokens>, ProjectError> {
    let tokens = load_erc_3525_portfolio(
        &project,
        &project.address,
        &project.slot.expect("slot is required here"),
        &customer_tokens.as_slice(),
    )
    .await?;

    let customer_farm = farming_model
        .get_customer_farm(
            &wallet,
            &project.address,
            &project.slot.expect("slot is required here"),
        )
        .await?;

    let total_offseted: U256 = customer_farm.offseter_deposited.inner();
    let total_yielded: U256 = customer_farm.yielder_deposited.inner();

    let value = tokens
        .iter()
        .flatten()
        .fold(U256::zero() + total_yielded + total_offseted, |acc, e| {
            acc + e.value
        });
    let total_amount = total_amount(project.unit_price, project.payment_decimals, value);

    let image = match &project.slot_uri {
        Some(uri) => aggregate_image_from_slot_uri(&uri.as_str()).await,
        None => serde_json::Value::String("<deprecated>".to_owned()),
    };

    let project = ProjectWithTokens::Erc3525 {
        id: project.id,
        name: project.name,
        address: project.address,
        minter_address: project.minter_address,
        tokens: tokens.into_iter().flatten().collect(),
        total_amount,
        total_deposited_value: SlotValue::from_blockchain(
            total_offseted + total_yielded,
            project.value_decimals,
        )
        .into(),
        abi: PortfolioAbi {
            project: project.abi,
            minter: project.minter_abi,
        },
        image,
    };

    Ok(Some(project))
}

fn total_amount(unit_price: U256, _payment_decimals: U256, amount: U256) -> U256 {
    // TODO: replace f64 with bigdecimal
    unit_price * amount
}

async fn aggregate_tokens_with_project(
    farming_model: Arc<PostgresFarming>,
    projects_data: Vec<ProjectWithMinterAndPaymentViewModel>,
    wallet: String,
    customer_tokens: Vec<CustomerToken>,
) -> Result<Vec<Option<ProjectWithTokens>>, ApiError> {
    let mut handles = vec![];
    for project in projects_data.into_iter() {
        let wallet_address = wallet.clone();
        let tokens = customer_tokens.clone();
        let model = farming_model.clone();
        let handle = match &project.erc_implementation {
            ErcImplementation::Enum => {
                return Err(ApiError::ProjectError(
                    ProjectError::InvalidErcImplementation,
                ));
            }
            ErcImplementation::Erc721 => {
                tokio::spawn(async move { aggregate_721_tokens(project, wallet_address).await })
            }
            ErcImplementation::Erc3525 => tokio::spawn(async move {
                aggregate_3525_tokens(model, project, wallet_address, tokens.to_vec()).await
            }),
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
    total: bigdecimal::BigDecimal,
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
    let mut wallet = wallet_param.into_inner();
    ensure_starknet_wallet(&mut wallet);

    let project_model: PostgresProject<Erc721> = PostgresProject::new(data.db_client_pool.clone());
    let customer_token_model = PostgresCustomer::new(data.db_client_pool.clone());
    let farming_model = Arc::new(PostgresFarming::new(data.db_client_pool.clone()));

    let customer_tokens = customer_token_model
        .get_customer_erc3525_tokens(&wallet)
        .await?;

    let projects_data = project_model
        .find_projects_with_minter_and_payment()
        .await?;
    let projects = aggregate_tokens_with_project(
        farming_model.clone(),
        projects_data,
        wallet,
        customer_tokens,
    )
    .await?;
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
            global: Global {
                total: total.to_big_decimal(6),
            },
            projects: filtered_projects,
            badges: vec![],
        },
    }))
}
