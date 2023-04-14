use deadpool_postgres::{Object, Transaction};
use serde::{Deserialize, Serialize};

use crate::infrastructure::postgres::event_source::PostgresStorageManager;

use super::{Consumer, DomainError, DomainEvent, Event};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectEvents {
    Upgraded,
    AbsorptionUpdate,
    Transfer,
    TransferValue,
    Approval,
    ApprovalValue,
    ApprovalForAll,
    SlotChanged,
    MetadataUpdate,
    BatchMetadataUpdate,
}

#[derive(Default)]
pub struct ProjectTransferEventConsumer {}

impl ProjectTransferEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<PostgresStorageManager> for ProjectTransferEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        match event {
            Event::Project(ProjectEvents::Transfer) => true,
            _ => false,
        }
    }

    async fn consume(&self, event: DomainEvent) -> Result<(), DomainError> {
        println!("{:#?}", event);
        todo!()
    }
}

#[derive(Default)]
pub struct ProjectTransferValueEventConsumer {}

impl ProjectTransferValueEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<PostgresStorageManager> for ProjectTransferValueEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        match event {
            Event::Project(ProjectEvents::TransferValue) => true,
            _ => false,
        }
    }

    async fn consume(&self, event: DomainEvent) -> Result<(), DomainError> {
        println!("{:#?}", event);
        todo!()
    }
}
