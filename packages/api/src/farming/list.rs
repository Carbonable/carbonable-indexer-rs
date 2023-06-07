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

async fn aggregate_metadata(
    mut projects: Vec<FarmingProjectsViewModel>,
) -> Result<Vec<FarmingProjectsViewModel>, ApiError> {
    let client = Client::new();
    for p in projects.iter_mut() {
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

pub async fn farming_list(data: web::Data<AppDependencies>) -> Result<impl Responder, ApiError> {
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let mut projects = project_model.get_farming_projects().await?;
    projects = aggregate_metadata(projects).await?;
    Ok(HttpResponse::Ok().json(ServerResponse::Data { data: projects }))
}

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
    let yielder_id = match farming_data.yielder_id {
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

    let snapshots = project_model.get_snapshots(yielder_id).await?;
    let provisions = project_model.get_provisions(yielder_id).await?;

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
    let total_value = project_model
        .get_project_value_times_unit_price(project.id)
        .await?;

    let unconnected_data_project =
        get_unconnected_project_data(project, farming_data, snapshots, provisions, total_value)
            .await?;
    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: unconnected_data_project,
    }))
}

pub async fn connected(
    route_params: web::Path<(String, String)>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let (mut wallet, slug) = route_params.into_inner();
    ensure_starknet_wallet(&mut wallet);
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let customer_model = PostgresCustomer::new(data.db_client_pool.clone());

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

    let customer_project_data =
        get_customer_listing_project_data(project, farming_data, &wallet, &mut customer_tokens)
            .await?;

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: customer_project_data,
    }))
}
