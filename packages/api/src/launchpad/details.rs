use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{domain::Erc721, infrastructure::postgres::project::PostgresProject};

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

pub async fn launchpad_details(
    slug_param: web::Path<String>,
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let slug = slug_param.into_inner();
    let project_model: PostgresProject<Erc721> = PostgresProject::new(data.db_client_pool.clone());
    let project = project_model.get_launchpad_details(&slug).await?;

    Ok(HttpResponse::Ok().json(ServerResponse::Data { data: project }))
}
