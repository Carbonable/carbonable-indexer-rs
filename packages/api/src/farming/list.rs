use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::infrastructure::{
    postgres::{customer::PostgresCustomer, farming::PostgresFarming},
    starknet::{
        ensure_starknet_wallet,
        farming::{
            get_customer_global_farming_data, get_customer_listing_project_data,
            get_unconnected_project_data,
        },
    },
    view_model::farming::{FarmingProjectsViewModel, UnconnectedFarmingData},
};
use reqwest::Client;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

/// Aggregates metadata for projects.
/// Supports both IPFS and HTTP URIs metadata for erc721 or 3525
/// * `projects`: Vec<FarmingProjectsViewModel>
async fn aggregate_metadata(
    mut projects: Vec<FarmingProjectsViewModel>,
) -> Result<Vec<FarmingProjectsViewModel>, ApiError> {
    let client = Client::new();
    for p in projects.iter_mut() {
        // If uri is a json string encoded, we parse it
        if p.uri.uri.starts_with("data:application/json") {
            let metadata =
                serde_json::from_str(p.uri.uri.replace("data:application/json,", "").as_str())?;
            p.uri.data = metadata;
            continue;
        }
        // else this is an ipfs uri we fetch the metadata
        let data = client
            .get(format!("{}/token", p.uri.uri))
            .send()
            .await?
            .json()
            .await
            .expect("failed to parse json");
        p.uri.data = data;
    }
    Ok(projects)
}

/// Get a list of all projects on which farming is available.
/// * `data`: AppDependencies
pub async fn farming_list(data: web::Data<AppDependencies>) -> Result<impl Responder, ApiError> {
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let mut projects = project_model.get_farming_projects().await?;
    projects = aggregate_metadata(projects).await?;
    Ok(HttpResponse::Ok().json(ServerResponse::Data { data: projects }))
}

/// Get global data (displayed on top of the farming list page)
/// This represents the sum of all projects for a given wallet
/// * `wallet_param`: wallet address
/// * `data`: AppDependencies
pub async fn global(
    wallet_param: web::Path<String>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let mut wallet = wallet_param.into_inner();
    ensure_starknet_wallet(&mut wallet);

    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let data = project_model.get_data_for_farming(None).await?;

    let customer_global_data = get_customer_global_farming_data(wallet, data).await?;

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: customer_global_data,
    }))
}

/// Get unconnected data
/// * `slug_param`: project slug
/// * `data`: AppDependencies
pub async fn unconnected(
    slug_param: web::Path<String>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let slug = slug_param.into_inner();
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let mut project_data = project_model
        .get_data_for_farming(Some(slug.to_string()))
        .await?;
    let farming_data = match project_model.get_complete_farming_data(slug).await? {
        Some(id) => id,
        None => {
            return Ok(HttpResponse::NotFound().json(
                ServerResponse::<UnconnectedFarmingData>::Error {
                    code: 404,
                    error_message: "Not found".to_string(),
                    message: "Project not found".to_string(),
                },
            ))
        }
    };

    if project_data.is_empty() || farming_data.absorptions.is_empty() {
        return Ok(HttpResponse::NotFound().json(
            ServerResponse::<UnconnectedFarmingData>::Error {
                code: 404,
                error_message: "Not found".to_string(),
                message: "Project not found".to_string(),
            },
        ));
    }
    let project = project_data.pop().unwrap();

    let unconnected_data_project = get_unconnected_project_data(project, farming_data).await?;
    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: unconnected_data_project,
    }))
}

/// Get data for wallet and project
/// * `route_params`: wallet address and project slug
/// * `data`: AppDependencies
pub async fn connected(
    route_params: web::Path<(String, String)>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let (mut wallet, slug) = route_params.into_inner();
    ensure_starknet_wallet(&mut wallet);
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let customer_model = PostgresCustomer::new(data.db_client_pool.clone());
    let farming_model = PostgresFarming::new(data.db_client_pool.clone());

    let mut project_data = project_model
        .get_data_for_farming(Some(slug.to_string()))
        .await?;
    let farming_data = match project_model.get_complete_farming_data(slug).await? {
        Some(d) => d,
        None => {
            return Ok(HttpResponse::NotFound().json(
                ServerResponse::<UnconnectedFarmingData>::Error {
                    code: 404,
                    error_message: "Not found".to_string(),
                    message: "Project not found".to_string(),
                },
            ))
        }
    };

    if project_data.is_empty() {
        return Ok(HttpResponse::NotFound().json(
            ServerResponse::<UnconnectedFarmingData>::Error {
                code: 404,
                error_message: "Not found".to_string(),
                message: "Project not found".to_string(),
            },
        ));
    }
    let project = project_data.pop().unwrap();
    let mut customer_tokens = customer_model
        .get_customer_tokens(&wallet, &project.project_address, &project.slot)
        .await?;
    let customer_farm = farming_model
        .get_customer_farm(&wallet, &project.project_address, &project.slot)
        .await?;

    let customer_project_data = get_customer_listing_project_data(
        project,
        farming_data,
        &wallet,
        &mut customer_tokens,
        &customer_farm,
    )
    .await?;

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: customer_project_data,
    }))
}
