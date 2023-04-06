use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::infrastructure::{
    postgres::project::PostgresProject, view_model::project::Project,
};

use crate::{common::ServerResponse, AppDependencies};

pub async fn get_by_slug(
    data: web::Data<AppDependencies>,
    slug_param: web::Path<String>,
) -> impl Responder {
    let slug = slug_param.into_inner();
    let postgres_model = PostgresProject::new(data.db_client_pool.clone());
    match postgres_model.find_by_slug(&slug).await {
        Ok(Some(p)) => HttpResponse::Ok().json(ServerResponse::Data { data: p }),
        Ok(None) => HttpResponse::NotFound().json(ServerResponse::<Project>::Error {
            code: 404,
            error_message: "Not Found".to_string(),
            message: "Project not found".to_string(),
        }),
        Err(_) => HttpResponse::InternalServerError().json(ServerResponse::<Project>::Error {
            code: 500,
            error_message: "Internal Server Error".to_string(),
            message: "Unexpected error occured".to_string(),
        }),
    }
}
