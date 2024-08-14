use actix_web::{web, HttpResponse, Responder};
use carbonable_domain::{
    domain::Erc721,
    infrastructure::{
        postgres::project::PostgresProject,
        view_model::project::{Project, ProjectViewModel},
    },
};
use reqwest::Client;

use crate::{
    common::{ApiError, ServerResponse},
    AppDependencies,
};

/// Aggregate metadata from uri
/// * `project` - project view model
async fn aggregate_metadata(mut project: ProjectViewModel) -> Result<ProjectViewModel, ApiError> {
    let client = Client::new();
    let uri = match &project {
        ProjectViewModel::Erc721(p) => &p.uri.uri,
        ProjectViewModel::Erc3525(p) => &p.uri.uri,
    };

    let data: serde_json::Value = if uri.starts_with("data:application/json") {
        serde_json::from_str(uri.replace("data:application/json,", "").as_str())?
    } else {
        client
            .get(format!("{}/token", uri))
            .send()
            .await?
            .json()
            .await
            .expect("failed to parse json")
    };

    match &mut project {
        ProjectViewModel::Erc721(p) => p.uri.data = data,
        ProjectViewModel::Erc3525(p) => p.uri.data = data,
    }

    Ok(project)
}

/// Get project by slug
/// * `data` - app dependencies
/// * `slug_param` - slug
pub async fn get_by_slug(
    data: web::Data<AppDependencies>,
    slug_param: web::Path<String>,
) -> impl Responder {
    let slug = slug_param.into_inner();
    let postgres_model: PostgresProject<Erc721> = PostgresProject::new(data.db_client_pool.clone());
    match postgres_model.find_by_slug(&slug).await {
        Ok(Some(p)) => {
            let with_uri = aggregate_metadata(p).await.unwrap();
            HttpResponse::Ok().json(ServerResponse::Data { data: with_uri })
        }
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
