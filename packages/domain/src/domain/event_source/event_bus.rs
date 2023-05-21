use deadpool_postgres::{Pool, Transaction};
use thiserror::Error;
use tracing::{debug, error};

use crate::infrastructure::postgres::event_source::insert_last_domain_event;

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
    async fn consume(&self, e: &DomainEvent, txn: &mut Txn) -> Result<(), DomainError>;
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
    pub async fn dispatch(
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
                consumer.consume(event, &mut tx).await?;
            }
        }

        // Rollback transaction if storing domain event fails
        match insert_last_domain_event(&tx, event, metadata).await {
            Ok(_) => &tx.commit().await?,
            Err(err) => {
                error!("error while commiting to db : {:#?}", err);
                &tx.rollback().await?
            }
        };

        Ok(())
    }
}
