pub mod badge;
pub mod migrator;
pub mod minter;
pub mod offseter;
pub mod project;
pub mod yielder;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use crate::infrastructure::{flatten, postgres::PostgresError as InfraPostgresError};
use starknet::providers::{ProviderError, SequencerGatewayProviderError};

use thiserror::Error;
use tracing::{debug, error};

use super::starknet::{model::ModelError, SequencerError};

#[derive(Error, Debug)]
pub enum DataSeederError {
    #[error("Failed to parse json data file")]
    FailedToParseFile,
    #[error("No seeder found for key {0}")]
    NoSeederForKey(String),
    #[error(transparent)]
    FailedToQueryBLockchain(#[from] ProviderError<SequencerGatewayProviderError>),
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    PostgresError(#[from] InfraPostgresError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error("failed to get starknet rpc provider from env")]
    SequencerError(#[from] SequencerError),
}

#[async_trait::async_trait]
pub trait SeederManager {
    async fn handle(&self, data: HashMap<String, String>) -> Result<(), DataSeederError>;
}

#[async_trait::async_trait]
pub trait Seeder {
    async fn seed(&self, address: String) -> Result<String, DataSeederError>;
    fn can_process(&self, seeder_type: String) -> bool;
}

pub struct DataSeeder<T: SeederManager + Send + Sync> {
    data: Vec<HashMap<String, String>>,
    inner: Arc<T>,
}

/// Read data json file with simple key-value format
pub fn read_data_content<P: AsRef<std::path::Path>>(
    file_path: P,
) -> Result<Vec<HashMap<String, String>>, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(file_path)?;
    let reader = std::io::BufReader::new(file);

    let content = serde_json::from_reader(reader)?;
    Ok(content)
}

impl DataSeeder<SqlSeederManager> {
    pub async fn feed_from_data<P: AsRef<std::path::Path>>(
        file_path: P,
        seeders: Vec<Arc<dyn Seeder + Send + Sync>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = read_data_content(file_path)?;

        Ok(DataSeeder {
            data: content,
            inner: Arc::new(SqlSeederManager::new(seeders)),
        })
    }

    pub async fn seed(&self) -> Result<(), DataSeederError> {
        debug!("Seeding data.");

        let mut seeds = vec![];
        // Seeds data from blockchain to database
        for data in self.data.iter() {
            let inner = self.inner.clone();
            let data = data.clone();
            let handle = tokio::spawn(async move { inner.handle(data).await });
            seeds.push(flatten(handle));
        }

        match futures::future::try_join_all(seeds).await {
            Ok(_r) => Ok(()),
            Err(e) => {
                error!("{:#?}", e);
                Err(e)
            }
        }
    }
}

#[derive(Default)]
pub struct SqlSeederManager {
    seeders: Vec<Arc<dyn Seeder + Send + Sync>>,
}

impl SqlSeederManager {
    fn new(seeders: Vec<Arc<dyn Seeder + Send + Sync>>) -> Self {
        Self { seeders }
    }
}

#[async_trait::async_trait]
impl SeederManager for SqlSeederManager {
    async fn handle(&self, data: HashMap<String, String>) -> Result<(), DataSeederError> {
        for (key, value) in data.iter() {
            match self.seeders.iter().find(|s| s.can_process(key.to_string())) {
                None => continue,
                Some(seeder) => seeder.seed(value.to_string()).await?,
            };
        }

        Ok(())
    }
}
