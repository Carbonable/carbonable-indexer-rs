pub mod badge;
pub mod entity;
pub mod implementation;
pub mod minter;
pub mod offseter;
pub mod payment;
pub mod project;
pub mod uri;
mod vester;
pub mod yielder;

use crate::infrastructure::starknet::model::StarknetModel;

use deadpool::managed::PoolError;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use starknet::{
    core::types::FieldElement,
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider,
    },
};
use std::sync::Arc;
use thiserror::Error;
use tokio_postgres::{Config, NoTls};

use self::{
    badge::PostgresBadge,
    entity::{Implementation, Payment, Project, Uri, Vester},
    implementation::PostgresImplementation,
    minter::PostgresMinter,
    offseter::PostgresOffseter,
    payment::PostgresPayment,
    project::PostgresProject,
    uri::PostgresUri,
    vester::PostgresVester,
    yielder::PostgresYielder,
};

use super::{
    seed::{project::ProjectSeeder, vester::VesterSeeder, Seeder},
    starknet::{get_proxy_abi, model::ModelError, payment::PaymentModel, uri::UriModel},
};

#[derive(Error, Debug)]
pub enum PostgresError<T: Provider> {
    #[error(transparent)]
    ParseConfigError(#[from] tokio_postgres::Error),
    #[error("you have to provide 'DATABASE_URI' environment variable")]
    NoEnvVarProvided(#[from] std::env::VarError),
    #[error(transparent)]
    PoolError(#[from] PoolError<tokio_postgres::Error>),
    #[error(transparent)]
    SeaQueryError(#[from] sea_query::error::Error),
    #[error("unexpected database error")]
    UnexpectedError,
    #[error(transparent)]
    ModelError(#[from] ModelError<T>),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("failed to seed project")]
    FailedToSeedProject,
    #[error("failed to seed vester")]
    FailedToSeedVester,
}

pub async fn get_connection(database_uri: Option<&str>) -> Result<Pool, PostgresError> {
    let db_env_uri = std::env::var("DATABASE_URI")?;
    let config = database_uri.unwrap_or(&db_env_uri).parse::<Config>()?;
    let manager_config = ManagerConfig {
        recycling_method: RecyclingMethod::Verified,
    };
    let manager = Manager::from_config(config, NoTls, manager_config);
    let pool = Pool::builder(manager).max_size(16).build().unwrap();

    Ok(pool)
}

#[derive(Clone, Debug)]
pub struct PostgresModels {
    pub project: Arc<PostgresProject>,
    pub implementation: Arc<PostgresImplementation>,
    pub uri: Arc<PostgresUri>,
    pub badge: Arc<PostgresBadge>,
    pub minter: Arc<PostgresMinter>,
    pub payment: Arc<PostgresPayment>,
    pub vester: Arc<PostgresVester>,
    pub offseter: Arc<PostgresOffseter>,
    pub yielder: Arc<PostgresYielder>,
}

impl PostgresModels {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        let project = Arc::new(PostgresProject::new(db_client_pool.clone()));
        let implementation = Arc::new(PostgresImplementation::new(db_client_pool.clone()));
        let uri = Arc::new(PostgresUri::new(db_client_pool.clone()));
        let badge = Arc::new(PostgresBadge::new(db_client_pool.clone()));
        let minter = Arc::new(PostgresMinter::new(db_client_pool.clone()));
        let payment = Arc::new(PostgresPayment::new(db_client_pool.clone()));
        let vester = Arc::new(PostgresVester::new(db_client_pool.clone()));
        let offseter = Arc::new(PostgresOffseter::new(db_client_pool.clone()));
        let yielder = Arc::new(PostgresYielder::new(db_client_pool));

        Self {
            project,
            implementation,
            uri,
            badge,
            minter,
            payment,
            vester,
            offseter,
            yielder,
        }
    }
}

pub async fn find_or_create_project(
    db_models: Arc<PostgresModels>,
    address: &str,
) -> Result<Project, PostgresError> {
    match db_models.project.find_by_address(address).await? {
        Some(p) => Ok(p),
        None => {
            let seeder = ProjectSeeder {
                db_models: db_models.clone(),
            };
            match seeder.seed(address.to_string()).await {
                Ok(_p) => Ok(db_models
                    .project
                    .find_by_address(address)
                    .await?
                    .expect("project should have been created")),
                Err(_e) => Err(PostgresError::FailedToSeedProject),
            }
        }
    }
}
pub async fn find_or_create_vester(
    db_models: Arc<PostgresModels>,
    address: &str,
) -> Result<Vester, PostgresError> {
    match db_models.vester.find_by_address(address).await? {
        Some(v) => Ok(v),
        None => {
            let seeder = VesterSeeder {
                db_models: db_models.clone(),
            };
            match seeder.seed(address.to_string()).await {
                Ok(_v) => Ok(db_models
                    .vester
                    .find_by_address(address)
                    .await?
                    .expect("vester should have been created")),
                Err(_e) => Err(PostgresError::FailedToSeedVester),
            }
        }
    }
}

pub async fn find_or_create_payment(
    db_models: Arc<PostgresModels>,
    address: &str,
) -> Result<Payment, PostgresError> {
    match db_models.payment.find_by_address(address).await? {
        Some(p) => Ok(p),
        None => {
            let payment_model = PaymentModel::new(FieldElement::from_hex_be(address).unwrap())?;
            let data = payment_model.load().await?;
            let payment = db_models.payment.create(address, data).await?;
            Ok(payment)
        }
    }
}

pub async fn find_or_create_uri(
    db_model: Arc<PostgresUri>,
    address: &str,
    project_uri: &str,
) -> Result<Uri, PostgresError> {
    match db_model.find_by_uri(address).await? {
        Some(u) => Ok(u),
        None => {
            let uri_model = UriModel::new(project_uri.to_string())?;
            let metadata = uri_model.load().await?;

            Ok(db_model
                .create(address, serde_json::to_value(&metadata)?)
                .await?)
        }
    }
}
pub async fn find_or_create_implementation(
    db_model: Arc<PostgresImplementation>,
    provider: Arc<JsonRpcClient<HttpTransport>>,
    address: &str,
    implementation_hash: &str,
) -> Result<Implementation, PostgresError> {
    let abi = get_proxy_abi(
        provider,
        FieldElement::from_hex_be(implementation_hash).unwrap(),
    )
    .await?;
    match db_model.find_by_address(address).await? {
        Some(i) => Ok(i),
        None => Ok(db_model.create(address, abi).await?),
    }
}
