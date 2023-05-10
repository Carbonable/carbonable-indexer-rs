pub mod badge;
pub mod customer;
pub mod entity;
pub mod event_source;
pub mod event_store;
pub mod farming;
pub mod implementation;
pub mod minter;
pub mod offseter;
pub mod payment;
pub mod project;
pub mod uri;
pub mod yielder;

use crate::{
    domain::{Contract, Erc3525, Erc721},
    infrastructure::starknet::model::StarknetModel,
};

use deadpool::managed::PoolError;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{HttpTransport, JsonRpcClient},
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio_postgres::{error::SqlState, Config, NoTls};
use tracing::error;

use self::{
    badge::PostgresBadge,
    entity::{Implementation, Payment, Project, Uri},
    implementation::PostgresImplementation,
    minter::PostgresMinter,
    offseter::PostgresOffseter,
    payment::PostgresPayment,
    project::PostgresProject,
    uri::PostgresUri,
    yielder::PostgresYielder,
};

use super::{
    seed::{project::ProjectSeeder, Seeder},
    starknet::{
        get_proxy_abi,
        model::ModelError,
        payment::PaymentModel,
        uri::{BadgeMetadata, BadgeUriModel, Erc3525Metadata, Metadata, UriModel},
    },
};

#[derive(Error, Debug)]
pub enum PostgresError {
    #[error(transparent)]
    TokioPostgresError(#[from] tokio_postgres::Error),
    #[error("you have to provide 'DATABASE_URI' environment variable")]
    NoEnvVarProvided(#[from] std::env::VarError),
    #[error(transparent)]
    PoolError(#[from] PoolError<tokio_postgres::Error>),
    #[error(transparent)]
    SeaQueryError(#[from] sea_query::error::Error),
    #[error("unexpected database error")]
    UnexpectedError,
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("failed to seed project")]
    FailedToSeedProject,
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
pub struct PostgresModels<C: Contract> {
    pub project: Arc<PostgresProject>,
    pub implementation: Arc<PostgresImplementation>,
    pub uri: Arc<PostgresUri>,
    pub badge: Arc<PostgresBadge>,
    pub minter: Arc<PostgresMinter<C>>,
    pub payment: Arc<PostgresPayment>,
    pub offseter: Arc<PostgresOffseter>,
    pub yielder: Arc<PostgresYielder>,
}

impl<C> PostgresModels<C>
where
    C: Contract + Send + Sync,
{
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        let project = Arc::new(PostgresProject::new(db_client_pool.clone()));
        let implementation = Arc::new(PostgresImplementation::new(db_client_pool.clone()));
        let uri = Arc::new(PostgresUri::new(db_client_pool.clone()));
        let badge = Arc::new(PostgresBadge::new(db_client_pool.clone()));
        let minter = Arc::new(PostgresMinter::<C>::new(db_client_pool.clone()));
        let payment = Arc::new(PostgresPayment::new(db_client_pool.clone()));
        let offseter = Arc::new(PostgresOffseter::new(db_client_pool.clone()));
        let yielder = Arc::new(PostgresYielder::new(db_client_pool));

        Self {
            project,
            implementation,
            uri,
            badge,
            minter,
            payment,
            offseter,
            yielder,
        }
    }
}

pub async fn find_or_create_project(
    db_models: Arc<PostgresModels<Erc721>>,
    address: &str,
) -> Result<Project, PostgresError> {
    match db_models.project.find_by_address(address).await? {
        Some(p) => Ok(p),
        None => {
            let seeder = ProjectSeeder::<Erc721>::new(db_models.clone());
            match seeder.seed(address.to_string()).await {
                Ok(_p) => Ok(db_models
                    .project
                    .find_by_address(address)
                    .await?
                    .expect("erc721 project should have been created")),
                Err(e) => {
                    error!("error: {:#?}", e);
                    Err(PostgresError::FailedToSeedProject)
                }
            }
        }
    }
}

#[async_recursion::async_recursion]
pub async fn find_or_create_3525_project(
    db_models: Arc<PostgresModels<Erc3525>>,
    address: &str,
    slot: &u64,
) -> Result<Project, PostgresError> {
    match db_models
        .project
        .find_by_address_and_slot(address, slot)
        .await?
    {
        Some(p) => Ok(p),
        None => {
            let seeder = ProjectSeeder::<Erc3525>::new(db_models.clone());
            match seeder.seed_from_slot(address.to_string(), slot).await {
                Ok(_p) => match db_models
                    .project
                    .find_by_address_and_slot(address, slot)
                    .await?
                {
                    Some(p) => Ok(p),
                    None => {
                        error!("project not created yet");
                        while db_models
                            .project
                            .find_by_address_and_slot(address, slot)
                            .await?
                            .is_none()
                        {
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            let _seed_res = seeder.seed_from_slot(address.to_string(), slot).await;
                        }

                        Ok(db_models
                            .project
                            .find_by_address(address)
                            .await?
                            .expect("erc3525 project should have been created"))
                    }
                },
                Err(e) => {
                    error!("error: {:#?}", e);
                    Err(PostgresError::FailedToSeedProject)
                }
            }
        }
    }
}

pub async fn find_or_create_payment<C>(
    db_models: Arc<PostgresModels<C>>,
    address: &str,
) -> Result<Payment, PostgresError>
where
    C: Contract + Send + Sync,
{
    match db_models.payment.find_by_address(address).await? {
        Some(p) => Ok(p),
        None => {
            let payment_model = PaymentModel::new(FieldElement::from_hex_be(address).unwrap())?;
            let data = payment_model.load().await?;
            match db_models.payment.create(address, data).await {
                Ok(payment) => Ok(payment),
                Err(e) => match e {
                    PostgresError::TokioPostgresError(e) => {
                        if e.code().eq(&Some(&SqlState::UNIQUE_VIOLATION)) {
                            return Ok(db_models
                                .payment
                                .find_by_address(address)
                                .await?
                                .expect("payment should have been created there"));
                        }
                        Err(e.into())
                    }
                    _ => Err(e),
                },
            }
        }
    }
}

/// Fetches ipfs metadata from [`contractURI`] method from [`address`]
/// * db_models: [`PostgresModels`]
/// * address: [`&str`] Blockchain contract address
/// * badge_uri: [`&str`] Badge ipfs uri fetched from blockchain
///
pub async fn find_or_create_badge_uri(
    db_model: Arc<PostgresUri>,
    address: &str,
    badge_uri: &str,
) -> Result<Uri, PostgresError> {
    match db_model.find_by_uri(address).await? {
        Some(u) => Ok(u),
        None => {
            let uri_model = BadgeUriModel::new(badge_uri.to_string())?;
            let metadata: BadgeMetadata = uri_model.load().await?;

            Ok(db_model
                .create(badge_uri, address, serde_json::to_value(&metadata)?)
                .await?)
        }
    }
}

// TODO: Find a better way to deduplicate this
pub async fn find_or_create_uri_721(
    db_model: Arc<PostgresUri>,
    address: &str,
    project_uri: &str,
) -> Result<Uri, PostgresError> {
    match db_model.find_by_uri(address).await? {
        Some(u) => Ok(u),
        None => {
            let uri_model = UriModel::<Erc721>::new(project_uri.to_string())?;
            let metadata: Metadata = uri_model.load().await?;

            Ok(db_model
                .create(project_uri, address, serde_json::to_value(&metadata)?)
                .await?)
        }
    }
}
pub async fn find_or_create_uri_3525(
    db_model: Arc<PostgresUri>,
    address: &str,
    project_uri: &str,
) -> Result<Uri, PostgresError> {
    match db_model.find_by_uri(address).await? {
        Some(u) => Ok(u),
        None => {
            let uri_model = UriModel::<Erc3525>::new(project_uri.to_string())?;
            let metadata: Erc3525Metadata = uri_model.load().await?;

            Ok(db_model
                .create(project_uri, address, serde_json::to_value(&metadata)?)
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
