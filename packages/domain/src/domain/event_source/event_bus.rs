use deadpool_postgres::{Pool, Transaction};
use thiserror::Error;
use tracing::{debug, error};

use crate::infrastructure::postgres::event_source::{
    event_was_processed, insert_last_domain_event,
};

use super::{transaction::TransactionManager, BlockMetadata, DomainError, DomainEvent, Event};
use std::{fmt::Debug, sync::Arc};

#[derive(Debug, Error)]
pub enum EventBusError {
    #[error(transparent)]
    PoolError(#[from] deadpool_postgres::PoolError),
    #[error(transparent)]
    DomainError(#[from] DomainError),
    #[error(transparent)]
    TokioError(#[from] tokio_postgres::Error),
}

#[async_trait::async_trait]
pub trait Consumer<Txn>: Debug
where
    Txn: TransactionManager,
{
    fn can_consume(&self, e: &Event) -> bool;
    async fn consume(
        &self,
        e: &DomainEvent,
        metadata: &BlockMetadata,
        txn: &mut Txn,
    ) -> Result<(), DomainError>;
}

#[derive(Debug)]
pub struct EventBus<Store, Consumer> {
    pub(crate) client_pool: Arc<Store>,
    pub(crate) consumers: Vec<Consumer>,
}

impl EventBus<Pool, Box<dyn for<'a> Consumer<Transaction<'a>>>> {
    pub fn new(client_pool: Arc<Pool>) -> Self {
        Self {
            client_pool,
            consumers: vec![],
        }
    }

    /// Add event consumer
    /// * `consumer` - [`Consumer`]
    ///
    pub fn add_consumer(&mut self, consumer: Box<dyn for<'a> Consumer<Transaction<'a>>>) {
        self.consumers.push(consumer);
    }

    /// Forward event to consumers.
    /// Add logic for pre.event and post.event
    ///
    /// Create db.tx commit if success
    /// * `event` - [`DomainEvent`]
    /// * `event` - [`BlockMetadata`]
    pub async fn register(
        &self,
        event: &DomainEvent,
        metadata: &BlockMetadata,
    ) -> Result<(), DomainError> {
        let mut client = self.client_pool.clone().get().await?;
        if event_was_processed(&client, event.id.as_str()).await {
            return Ok(());
        }
        let tx = client.transaction().await?;

        // Rollback transaction if storing domain event fails
        match insert_last_domain_event(&tx, event, metadata).await {
            Ok(_) => match tx.commit().await {
                Ok(_) => Ok(()),
                Err(_) => Err(DomainError::FailedToPersistEvent),
            },
            Err(err) => {
                error!("event_store.committing.error: {:#?}", err);
                let _ = tx.rollback().await;
                Err(DomainError::FailedToRollback)
            }
        }
    }

    pub async fn consume_event_store(
        &self,
        event: &DomainEvent,
        metadata: &BlockMetadata,
    ) -> Result<(), DomainError> {
        let mut client = self.client_pool.clone().get().await?;
        let mut tx = client.transaction().await?;
        for consumer in &self.consumers {
            if consumer.can_consume(&event.r#type) {
                debug!(
                    "Dispatching event: {:?} with id : {:?}",
                    &event.r#type, &event.id
                );
                consumer.consume(event, metadata, &mut tx).await?;
            }
        }

        match tx.commit().await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("event_store.domain_event.committing.error: {:#?}", e);
                Ok(())
            }
        }
    }
}
