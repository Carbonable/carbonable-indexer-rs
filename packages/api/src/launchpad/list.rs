use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{domain::Erc721, infrastructure::postgres::project::PostgresProject};

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

/// Get a list of all projects available throudh launchpad
/// * `data`: AppDependencies
pub async fn lauchpad_list(data: web::Data<AppDependencies>) -> Result<impl Responder, ApiError> {
    let project_model: PostgresProject<Erc721> = PostgresProject::new(data.db_client_pool.clone());
    let project_list = project_model.get_launchpad_list().await?;

    Ok(HttpResponse::Ok().json(ServerResponse::Data { data: project_list }))
}
