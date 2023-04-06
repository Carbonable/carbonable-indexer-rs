use actix_web::{web, HttpResponse, Responder};

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

pub async fn project_details(
    route_params: web::Path<(String, String)>,
    _data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let (wallet, slug) = route_params.into_inner();

    Ok(HttpResponse::Ok().json(ServerResponse::Data {
        data: format!("Ok this is details slot with {wallet} and {slug}"),
    }))
}
