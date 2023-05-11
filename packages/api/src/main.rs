use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use carbonable_domain::infrastructure::{
    app::{configure_application, Args},
    postgres::get_connection,
};
use deadpool_postgres::Pool;
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};
use tracing::info;

pub mod common;
pub mod farming;
pub mod launchpad;
pub mod portfolio;
pub mod project;

#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("Pong !")
}

pub struct AppDependencies {
    pub configuration: Arc<Args>,
    pub db_client_pool: Arc<Pool>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    info!("Starting Carbonable API...");
    env_logger::init();
    let configuration = match configure_application().await {
        Ok(c) => c,
        Err(_) => {
            return Err(Error::new(
                ErrorKind::Other,
                "failed to configure application",
            ))
        }
    };
    let db_client_pool = match get_connection(None).await {
        Ok(connection) => Arc::new(connection),
        Err(_) => {
            return Err(Error::new(
                ErrorKind::Other,
                "failed to acquire connection to database",
            ));
        }
    };
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppDependencies {
                configuration: Arc::new(configuration.clone()),
                db_client_pool: db_client_pool.clone(),
            }))
            .service(ping)
            .service(web::scope("/portfolio").route(
                "/{wallet}",
                web::get().to(portfolio::get_by_wallet::get_by_wallet),
            ))
            .service(
                web::scope("/projects")
                    .route("/{slug}", web::get().to(project::get_by_slug::get_by_slug)),
            )
            .service(
                web::scope("/farming")
                    .route("/list", web::get().to(farming::list::farming_list))
                    .route(
                        "/list/global/{wallet}",
                        web::get().to(farming::list::global),
                    )
                    .route(
                        "/list/unconnected/{slug}",
                        web::get().to(farming::list::unconnected),
                    )
                    .route(
                        "/list/{wallet}/{slug}",
                        web::get().to(farming::list::connected),
                    )
                    .route(
                        "/details/{wallet}/{slug}",
                        web::get().to(farming::details::project_details),
                    ),
            )
            .service(
                web::scope("/launchpad")
                    .route("/list", web::get().to(launchpad::list::lauchpad_list))
                    .route(
                        "/details/{slug}",
                        web::get().to(launchpad::details::launchpad_details),
                    ),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
