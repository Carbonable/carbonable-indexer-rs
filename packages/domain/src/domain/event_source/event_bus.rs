use deadpool_postgres::{Pool, Transaction};
use thiserror::Error;
use tracing::debug;

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

// #[async_trait::async_trait]
// pub trait DomainEventBus<C, Txm>
// where
//     C: Consumer<Txm>,
//     Txm: TransactionManager,
// {
//     fn add_consumer(&mut self, consumer: C);
//     async fn dispatch(
//         &self,
//         event: &DomainEvent,
//         metadata: &BlockMetadata,
//     ) -> Result<(), DomainError>;
// }

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
    pub fn add_consumer(&mut self, consumer: Box<dyn for<'a> Consumer<Transaction<'a>>>) {
        self.consumers.push(consumer);
    }

    /// Forward event to consumers.
    /// Add logic for pre.event and post.event
    ///
    /// Create db.tx commit if success
    pub async fn dispatch(
        &self,
        event: &DomainEvent,
        metadata: &BlockMetadata,
    ) -> Result<(), DomainError> {
        let mut client = self.client_pool.clone().get().await?;
        let mut tx = client.transaction().await?;

        for consumer in &self.consumers {
            if consumer.can_consume(&event.r#type) {
                debug!("Dispatching event: {:?}", &event.id);
                consumer.consume(event, &mut tx).await?;
            }
        }

        let _ = insert_last_domain_event(&tx, event, metadata).await;
        let _ = &tx.commit().await?;

        Ok(())
    }
}

// TODO: Make this working by implementing a comprehensible trait
// Traits implemented there are not flexible enough to make this work out
//
// impl
//     EventBus<
//         Mutex<HashMap<String, Vec<DomainEvent>>>,
//         Box<dyn Consumer<Mutex<HashMap<String, Vec<DomainEvent>>>>>,
//     >
// {
//     pub fn new() -> Self {
//         Self {
//             client_pool: Arc::new(Mutex::new(HashMap::new())),
//             consumers: vec![],
//         }
//     }
//
//     /// Add event consumer
//     pub fn add_consumer(
//         &mut self,
//         consumer: Box<dyn Consumer<Mutex<HashMap<String, Vec<DomainEvent>>>>>,
//     ) {
//         self.consumers.push(consumer);
//     }
//
//     /// Forward event to consumers.
//     /// Add logic for pre.event and post.event
//     ///
//     /// Create db.tx commit if success
//     pub async fn dispatch(
//         &self,
//         event: &DomainEvent,
//         metadata: &BlockMetadata,
//     ) -> Result<(), DomainError> {
//         let mut client = self.client_pool.clone();
//         for consumer in &self.consumers {
//             if consumer.can_consume(&event.r#type) {
//                 debug!("Dispatching event: {:?}", &event.id);
//                 consumer.consume(&event, &mut client).await?;
//             }
//         }
//
//         Ok(())
//     }
// }
