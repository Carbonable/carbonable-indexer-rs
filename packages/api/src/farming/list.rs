use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::infrastructure::{
    postgres::farming::PostgresFarming,
    starknet::farming::{
        get_customer_global_farming_data, get_customer_listing_project_data,
        get_unconnected_project_data,
    },
    view_model::farming::UnconnectedFarmingData,
};

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

pub async fn farming_list(data: web::Data<AppDependencies>) -> Result<impl Responder, ApiError> {
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let projects = project_model.get_farming_projects().await?;
    Ok(HttpResponse::Ok().json(ServerResponse::Data { data: projects }))
}

pub async fn global(
    wallet_param: web::Path<String>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let wallet = wallet_param.into_inner();

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
    let vestings = project_model.get_vestings(yielder_id).await?;

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
    let total_value = project_model.get_total_value(project.id).await?;

    let unconnected_data_project =
        get_unconnected_project_data(project, farming_data, snapshots, vestings, total_value)
            .await?;
    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: unconnected_data_project,
    }))
}

pub async fn connected(
    route_params: web::Path<(String, String)>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let (wallet, slug) = route_params.into_inner();
    let project_model = PostgresFarming::new(data.db_client_pool.clone());

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

    let customer_project_data =
        get_customer_listing_project_data(project, farming_data, &wallet).await?;

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: customer_project_data,
    }))
}
