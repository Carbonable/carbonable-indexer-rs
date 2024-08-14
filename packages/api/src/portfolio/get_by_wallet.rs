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
            ensure_starknet_wallet, get_starknet_rpc_from_env,
            model::{parallelize_blockchain_rpc_calls, u256_to_felt, ModelError},
            portfolio::{load_erc_3525_portfolio, load_erc_721_portfolio},
        },
        view_model::{
            customer::CustomerToken,
            portfolio::{PortfolioAbi, ProjectWithMinterAndPaymentViewModel, ProjectWithTokens},
        },
    },
};
use starknet::core::types::FieldElement;
use std::sync::Arc;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

const HA_IN_M2: u64 = 10000;
const TON_IN_G: u64 = 1000000;

/// Aggregate image from slot_uri
/// * `slot_uri` - slot uri as string. This can be a json string or a IPFS uri
async fn aggregate_image_from_slot_uri(slot_uri: &str) -> serde_json::Value {
    let client = Client::new();
    if slot_uri.starts_with("data:application/json") {
        let metadata: serde_json::Value =
            serde_json::from_str(slot_uri.replace("data:application/json,", "").as_str())
                .expect("failed to parse json");
        if let Some(image) = metadata.get("image") {
            return image.clone();
        }
        return metadata
            .get("image_data")
            .expect("failed to query field image_data on metadata")
            .clone();
    }
    let uri = slot_uri.replace('\"', "");
    let data: serde_json::Value = client
        .get(uri)
        .send()
        .await
        .expect("failed to query slot_uri")
        .json()
        .await
        .expect("failed to parse json");

    if let Some(image) = data.get("image") {
        return image.clone();
    }
    data.get("image_data")
        .expect("failed to query field image_data on metadata")
        .clone()
}

/// Based on project and wallet address, fetch 721 tokens associated to wallet to compute invested
/// amount.
/// * `model` - postgres farming model
/// * `project` - project with minter and payment view model
/// * `wallet` - wallet address
async fn aggregate_721_tokens(
    model: Arc<PostgresFarming>,
    project: ProjectWithMinterAndPaymentViewModel,
    wallet: String,
) -> Result<Option<ProjectWithTokens>, ProjectError> {
    let tokens = match load_erc_721_portfolio(&project, &wallet).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("failed to load 721 portfolio {:#?}", e);
            return Err(ProjectError::ModelError(e));
        }
    };
    if tokens.is_empty() {
        return Ok(None);
    }
    if project.unit_price.is_none() {
        return Err(ProjectError::UnitPriceNotSet);
    }
    let migrator_address = model.get_project_migrator_address(&project.address).await?;
    let total_amount = total_amount(
        project.unit_price.unwrap(),
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
        migrator_address,
    };

    Ok(Some(project))
}

/// Based on project and wallet address, fetch 3525 tokens associated to wallet to compute invested
/// amount.
/// * `farming_model` - postgres farming model
/// * `project` - project with minter and payment view model
/// * `wallet` - wallet address
/// * `customer_tokens` - customer tokens
async fn aggregate_3525_tokens(
    farming_model: Arc<PostgresFarming>,
    project: ProjectWithMinterAndPaymentViewModel,
    wallet: String,
    customer_tokens: Vec<CustomerToken>,
) -> Result<Option<ProjectWithTokens>, ProjectError> {
    let tokens = match load_erc_3525_portfolio(
        &project,
        &project.address,
        &project.slot.expect("slot is required here"),
        &customer_tokens.as_slice(),
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("failed to load 3525 portfolio {:#?}", e);
            return Err(ProjectError::ModelError(e));
        }
    };

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
    let total_amount = total_amount(project.unit_price.unwrap(), project.payment_decimals, value);

    let image = match &project.slot_uri {
        Some(uri) => aggregate_image_from_slot_uri(&uri.as_str()).await,
        None => serde_json::Value::String("<deprecated>".to_owned()),
    };
    let slot = project.slot.expect("slot is required here");
    let (asset_area, asset_carbon_unit) =
        get_asset_area_and_carbon_unit(farming_model.clone(), &project.address, &slot, &value)
            .await;

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
        asset_area: Some(asset_area),
        asset_carbon_unit: Some(asset_carbon_unit),
    };

    Ok(Some(project))
}

/// Get asset area and carbon unit for a given project
/// Calls 2 helper functions that fetches logic from smart contract
/// * `model` - postgres farming model
/// * `project_address` - project address
/// * `slot` - slot
/// * `value` - value
async fn get_asset_area_and_carbon_unit(
    model: Arc<PostgresFarming>,
    project_address: &str,
    slot: &U256,
    value: &U256,
) -> (String, String) {
    let uri_data: serde_json::Value = match model.get_project_metadata(project_address, slot).await
    {
        Ok(d) => d,
        Err(_) => return ("".to_owned(), "".to_owned()),
    };
    // Project Area (ha)
    let project_area = extract_from_uri(&uri_data, "8");
    // Total Carbon Units
    let project_carbon_unit = extract_from_uri(&uri_data, "10");

    let total_value: U256 = match get_project_total_value(project_address, slot).await {
        Ok(v) => v,
        Err(_) => return ("".to_owned(), "".to_owned()),
    };

    return (
        get_asset_area(value, &project_area, &total_value),
        get_asset_carbon_unit(value, &project_carbon_unit, &total_value),
    );
}
/// Get project total value i.e the total value available in project
/// * `project_address` - project address
/// * `slot` - slot
async fn get_project_total_value(project_address: &str, slot: &U256) -> Result<U256, ModelError> {
    let provider = get_starknet_rpc_from_env()?;
    let calldata = [(
        project_address.to_owned(),
        "get_project_value",
        vec![u256_to_felt(slot), FieldElement::ZERO],
    )];
    let data = parallelize_blockchain_rpc_calls(provider.into(), calldata.to_vec()).await?;
    let total_value = U256::try_from(data[0].clone())?;
    Ok(total_value)
}

/// Extract value from uri data
/// * `uri_data` - uri data
/// * `selector` - string selector with the `selector` attribute
fn extract_from_uri(uri_data: &serde_json::Value, selector: &str) -> U256 {
    let attribute = uri_data.pointer(&format!("/attributes/{selector}/value"));
    match attribute {
        Some(v) => U256::from(v.as_u64().expect("failed to parse u64 string from json")),
        None => U256::zero(),
    }
}

/// Calculates asset area against project value
/// * `value` - asset value
/// * `project_area` - project area
/// * `project_value` - project value
fn get_asset_area(value: &U256, project_area: &U256, project_value: &U256) -> String {
    let asset_area = (*value * (*project_area * U256::from(HA_IN_M2))) / *project_value;
    format_area(asset_area)
}

/// Calculates asset carbon unit against project value
/// * `value` - asset value
/// * `project_carbon_unit` - project carbon unit
/// * `project_value` - project value
fn get_asset_carbon_unit(value: &U256, project_carbon_unit: &U256, project_value: &U256) -> String {
    let asset_carbon_unit =
        (*value * (*project_carbon_unit * U256::from(TON_IN_G))) / *project_value;
    format_capacity(asset_carbon_unit)
}

/// Format area with the correct unit
/// * `value` - area value
fn format_area(value: U256) -> String {
    if value == U256::zero() {
        return "N/A".to_owned();
    }
    let ha_in_m2 = U256::from(HA_IN_M2);
    if value > ha_in_m2 {
        return format!("{}ha", (value / ha_in_m2).to_big_decimal(0));
    }

    return format!("{}m²", value.to_big_decimal(0));
}

/// Format carbon unit with the correct unit
/// * `value` - carbon unit value
fn format_capacity(value: U256) -> String {
    if value == U256::zero() {
        return "N/A".to_owned();
    }
    if value < U256::from(1000u64) {
        return format!("{}g", value.to_big_decimal(0));
    }
    if value < U256::from(1000000u64) {
        return format!("{}kg", (value / U256::from(1000u64)).to_big_decimal(0));
    }

    return format!("{}t", (value / U256::from(1000000u64)).to_big_decimal(0));
}

fn total_amount(unit_price: U256, _payment_decimals: U256, amount: U256) -> U256 {
    // TODO: replace f64 with bigdecimal
    unit_price * amount
}

/// For each project, fetch the tokens associated to the wallet based on the erc implementation of
/// the token
/// * `farming_model` - postgres farming model
/// * `projects_data` - projects with minter and payment view model
/// * `wallet` - wallet address
/// * `customer_tokens` - customer tokens
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
        let handle =
            match &project.erc_implementation {
                ErcImplementation::Enum => {
                    return Err(ApiError::ProjectError(
                        ProjectError::InvalidErcImplementation,
                    ));
                }
                ErcImplementation::Erc721 => tokio::spawn(async move {
                    aggregate_721_tokens(model, project, wallet_address).await
                }),
                ErcImplementation::Erc3525 => tokio::spawn(async move {
                    aggregate_3525_tokens(model, project, wallet_address, tokens.to_vec()).await
                }),
            };
        handles.push(flatten(handle));
    }

    match futures::future::try_join_all(handles).await {
        Ok(data) => Ok(data),
        Err(e) => {
            tracing::error!("failed to aggregate project data {:#?}", e);
            Err(ApiError::ProjectError(e))
        }
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

/// Get projects associated to a given wallet
/// * `data` - app dependencies
/// * `wallet_param` - wallet address
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
