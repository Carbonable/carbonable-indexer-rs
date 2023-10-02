use crate::{
    domain::crypto::U256,
    infrastructure::postgres::event_source::{
        create_token_for_customer, decrease_token_value, update_project_project_value,
        update_token_owner, update_token_slot, update_token_value,
    },
};
use apibara_core::starknet::v1alpha2::FieldElement;
use crypto_bigint::Encoding;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;
use std::{collections::HashMap, sync::Mutex};
use tracing::{error, info};

use super::{
    event_bus::Consumer, get_event, to_filters, BlockMetadata, DomainError, DomainEvent, Event,
    Filterable,
};

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
    ProjectValueUpdate,
}

/// Base struct for [`Project`] to enable [`Filterable`] behaviour
#[derive(Debug)]
pub struct ProjectFilters {
    contracts: Vec<String>,
    filters: HashMap<String, Vec<(String, Event)>>,
}

impl ProjectFilters {
    pub fn new() -> Self {
        Self {
            contracts: vec![],
            filters: HashMap::new(),
        }
    }
}

impl Default for ProjectFilters {
    fn default() -> Self {
        Self::new()
    }
}

/// Add [`Filterable`] behaviour on [`Project`]
impl Filterable for ProjectFilters {
    fn to_filters(&self) -> Vec<(String, String)> {
        to_filters(&self.filters)
    }

    fn get_event(&mut self, contract_address: &str, event_key: &str) -> Option<Event> {
        get_event(&mut self.filters, contract_address, event_key)
    }

    fn hydrate_from_file(&mut self, address_list: Vec<HashMap<String, String>>) {
        let contracts = self.extract_addresses(address_list, &["project", "project_3525"]);
        self.contracts = contracts;
        for contract in self.contracts.iter() {
            self.filters.insert(
                contract.to_string(),
                [
                    (
                        FieldElement::from_bytes(&selector!("Transfer").to_bytes_be()).to_string(),
                        Event::Project(ProjectEvents::Transfer),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("TransferValue").to_bytes_be())
                            .to_string(),
                        Event::Project(ProjectEvents::TransferValue),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("SlotChanged").to_bytes_be())
                            .to_string(),
                        Event::Project(ProjectEvents::SlotChanged),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("ProjectValueUpdate").to_bytes_be())
                            .to_string(),
                        Event::Project(ProjectEvents::ProjectValueUpdate),
                    ),
                ]
                .to_vec(),
            );
        }
    }
}

/// Consuming [`Transfer`] event emitted from [`Project`] on chain
#[derive(Default, Debug)]
pub struct ProjectTransferEventConsumer {}
impl ProjectTransferEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for ProjectTransferEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Project(ProjectEvents::Transfer))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        _metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // if transfer `from`=0 this means token_id is created.
        // when transfer event is emitted in blockchain, it only has data about transfer from one
        // address to an other.
        // We may encounter duplicated data due to this. Be careful when you handle event.
        let from = event.payload.get("0").expect("should have from value set");
        let to = event.payload.get("1").expect("should have to value set");
        let token_id_felt = FieldElement::from_hex(
            event
                .payload
                .get("2")
                .expect("should have token_id value set"),
        )
        .expect("should be able to convert to felt");
        let token_id = U256(crypto_bigint::U256::from_be_bytes(token_id_felt.to_bytes()));
        let contract_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");

        if FieldElement::from_u64(0) == FieldElement::from_hex(from).unwrap() {
            return Ok(create_token_for_customer(txn, contract_address, to, &token_id).await?);
        }

        Ok(update_token_owner(txn, from, contract_address, to, &token_id).await?)
    }
}

#[async_trait::async_trait]
impl Consumer<Mutex<HashMap<String, Vec<DomainEvent>>>> for ProjectTransferEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Project(ProjectEvents::Transfer))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        _metadata: &BlockMetadata,
        tx: &mut Mutex<HashMap<String, Vec<DomainEvent>>>,
    ) -> Result<(), DomainError> {
        let lock = tx.get_mut().unwrap();

        lock.entry("project-transfer".to_string())
            .or_insert(vec![])
            .push(event.clone());

        Ok(())
    }
}

/// Consuming [`TransferValue`] event emitted from [`Project`] on chain
#[derive(Default, Debug)]
pub struct ProjectTransferValueEventConsumer {}
impl ProjectTransferValueEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for ProjectTransferValueEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Project(ProjectEvents::TransferValue))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        _metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        let from_address = event.metadata.get("from_address").unwrap();
        let from_token_id =
            U256::from(FieldElement::from_hex(event.payload.get("0").unwrap()).unwrap());
        let to_token_id =
            U256::from(FieldElement::from_hex(event.payload.get("2").unwrap()).unwrap());
        let value = U256::from(FieldElement::from_hex(event.payload.get("4").unwrap()).unwrap());

        match update_token_value(txn, from_address, &to_token_id, value.clone()).await {
            Ok(_) => info!("project.transfer_value.update: success"),
            Err(e) => error!("project.transfer_value.update: failed {:#?}", e),
        }

        match decrease_token_value(txn, from_address, &from_token_id, value.clone()).await {
            Ok(_) => info!("project.transfer_value.decrease: success"),
            Err(e) => error!("project.transfer_value.decrease: failed {:#?}", e),
        }
        Ok(())
    }
}

/// Consuming [`SlotChanged`] event emitted from [`Project`] on chain
#[derive(Default, Debug)]
pub struct ProjectSlotChangedEventConsumer {}
impl ProjectSlotChangedEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for ProjectSlotChangedEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Project(ProjectEvents::SlotChanged))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        _metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // token_id is unique per contract AND per slot.
        let from_address = event.metadata.get("from_address").unwrap();
        let token_id = U256::from(FieldElement::from_hex(event.payload.get("0").unwrap()).unwrap());
        let old_slot = U256::from(FieldElement::from_hex(event.payload.get("2").unwrap()).unwrap());
        let slot = U256::from(FieldElement::from_hex(event.payload.get("4").unwrap()).unwrap());

        // token created from slot 0
        // if token is moved from a slot to another, it means to us that slot is moved from
        // a project to another one which is not possible at the moment.
        if U256::from(0u64) == old_slot {
            return Ok(update_token_slot(txn, from_address, &token_id, &slot).await?);
        }
        Ok(())
    }
}

/// Consuming [`ProjectValueUpdate`] event emitted from [`Project`] on chain
#[derive(Default, Debug)]
pub struct ProjectProjectValueUpdateEventConsumer {}
impl ProjectProjectValueUpdateEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for ProjectProjectValueUpdateEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Project(ProjectEvents::ProjectValueUpdate))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        _metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // slot: Uint256, projectValue: Uint256
        let from_address = event.metadata.get("from_address").unwrap();
        let slot = U256::from(FieldElement::from_hex(event.payload.get("0").unwrap()).unwrap());
        let value = U256::from(FieldElement::from_hex(event.payload.get("2").unwrap()).unwrap());

        Ok(update_project_project_value(txn, from_address, &slot, &value).await?)
    }
}
