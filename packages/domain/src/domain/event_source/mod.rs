pub mod minter;
pub mod offseter;
pub mod project;
pub mod vester;
pub mod yielder;

use std::{collections::HashMap, sync::Arc};

use deadpool::managed::Manager;
use deadpool_postgres::{Object, Pool, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::infrastructure::postgres::event_source::PostgresStorageManager;

use self::{
    minter::MinterEvents, offseter::OffseterEvents, project::ProjectEvents, vester::VesterEvents,
    yielder::YielderEvents,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    pub(crate) id: String,
    pub(crate) metadata: HashMap<String, String>,
    pub(crate) payload: HashMap<String, String>,
    pub(crate) r#type: Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Project(ProjectEvents),
    Minter(MinterEvents),
    Offseter(OffseterEvents),
    Yielder(YielderEvents),
    Vester(VesterEvents),
}

#[derive(Debug, Error)]
pub enum DomainError {}

#[derive(Debug, Error)]
pub enum EventBusError {
    #[error(transparent)]
    PoolError(#[from] deadpool_postgres::PoolError),
    #[error(transparent)]
    DomainError(#[from] DomainError),
}

pub struct EventBus<Sm> {
    pub(crate) db_client_pool: Arc<Pool>,
    pub(crate) consumers: Vec<Box<dyn Consumer<Sm>>>,
}

impl EventBus<PostgresStorageManager> {
    /// Creates a new event bus instance
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self {
            db_client_pool,
            consumers: vec![],
        }
    }

    /// Add event consumer
    pub fn add_consumer(&mut self, consumer: Box<dyn Consumer<PostgresStorageManager>>) {
        self.consumers.push(consumer);
    }

    /// Forward event to consumers.
    /// Add logic for pre.event and post.event
    ///
    /// Create db.tx commit if success
    pub async fn dispatch(&self, event: DomainEvent) -> Result<(), EventBusError> {
        let client = self.db_client_pool.get().await?;

        for consumer in &self.consumers {
            if consumer.can_consume(&event.r#type) {
                consumer.consume(event.clone()).await?;
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
pub trait Consumer<Sm>
where
    Sm: StorageManager,
{
    fn can_consume(&self, e: &Event) -> bool;
    async fn consume(&self, e: DomainEvent) -> Result<(), DomainError>;
}

#[async_trait::async_trait]
pub trait StorageManager {
    type Client;
    type Transaction<'a>
    where
        Self: 'a;

    async fn get_last_block(&self) -> Result<(), DomainError>;
    async fn get_client(&self) -> Self::Client;
    async fn store_events(&self) -> Result<(), DomainError>;
    async fn build_transaction(&self) -> Self::Transaction<'_>;
}
