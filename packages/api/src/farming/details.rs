use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::infrastructure::{
    postgres::{customer::PostgresCustomer, farming::PostgresFarming},
    starknet::{ensure_starknet_wallet, farming::get_customer_details_project_data},
    view_model::farming::UnconnectedFarmingData,
};

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

/// Get the details of a project for a given wallet.
/// * `route_params`: wallet address and project slug
/// * `data`: AppDependencies
pub async fn project_details(
    route_params: web::Path<(String, String)>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let (mut wallet, slug) = route_params.into_inner();
    ensure_starknet_wallet(&mut wallet);
    let project_model = PostgresFarming::new(data.db_client_pool.clone());
    let customer_token_model = PostgresCustomer::new(data.db_client_pool.clone());
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
    let mut customer_tokens = customer_token_model
        .get_customer_tokens(&wallet, &project.project_address, &project.slot)
        .await?;
    let customer_farm = farming_model
        .get_customer_farm(&wallet, &project.project_address, &project.slot)
        .await?;

    let customer_project_data = get_customer_details_project_data(
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
