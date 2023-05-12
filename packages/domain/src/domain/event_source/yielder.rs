use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::FieldElement;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;
use tracing::error;
use uuid::Uuid;

use crate::{
    domain::crypto::U256,
    infrastructure::{
        postgres::{
            entity::Snapshot,
            event_source::{
                add_provision_to_yielder, add_snapshot_to_yielder, get_yielder_id_from_address,
            },
        },
        starknet::model::felt_to_offset_datetime,
    },
};

use super::{
    event_bus::Consumer, get_event, to_filters, DomainError, DomainEvent, Event, Filterable,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum YielderEvents {
    Claim,
    Upgraded,
    Deposit,
    Withdraw,
    Snapshot,
    Provision,
}

/// Base struct for [`Yielder`] to enable [`Filterable`] behaviour
#[derive(Debug)]
pub struct YieldFilters {
    contracts: Vec<String>,
    filters: HashMap<String, Vec<(String, Event)>>,
}

impl YieldFilters {
    pub fn new() -> Self {
        Self {
            contracts: vec![],
            filters: HashMap::new(),
        }
    }
}
impl Default for YieldFilters {
    fn default() -> Self {
        Self::new()
    }
}

/// Add [`Filterable`] behaviour on [`Yielder`]
impl Filterable for YieldFilters {
    fn to_filters(&self) -> Vec<(String, String)> {
        to_filters(&self.filters)
    }

    fn get_event(&mut self, contract_address: &str, event_key: &str) -> Option<Event> {
        get_event(&mut self.filters, contract_address, event_key)
    }

    fn hydrate_from_file(&mut self, address_list: Vec<HashMap<String, String>>) {
        let contracts = self.extract_addresses(address_list, &["yielder", "yielder_3525"]);
        self.contracts = contracts;
        for contract in self.contracts.iter() {
            self.filters.insert(
                contract.to_string(),
                [
                    (
                        FieldElement::from_bytes(&selector!("Claim").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Claim),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Deposit").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Deposit),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Withdraw").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Withdraw),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Provision").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Provision),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Snapshot").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Snapshot),
                    ),
                ]
                .to_vec(),
            );
        }
    }
}

/// Consuming [`Claim`] event emitted from [`Yielder`] on chain
#[derive(Default, Debug)]
pub struct YielderClaimEventConsumer {}
impl YielderClaimEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for YielderClaimEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Yielder(YielderEvents::Deposit))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}

/// Consuming [`Deposit`] event emitted from [`Yielder`] on chain
#[derive(Default, Debug)]
pub struct YielderDepositEventConsumer {}
impl YielderDepositEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for YielderDepositEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Yielder(YielderEvents::Deposit))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}

/// Consuming [`Provision`] event emitted from [`Yielder`] on chain
#[derive(Default, Debug)]
pub struct YielderProvisionEventConsumer {}
impl YielderProvisionEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for YielderProvisionEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Yielder(YielderEvents::Provision))
    }

    async fn consume(&self, event: &DomainEvent, txn: &mut Transaction) -> Result<(), DomainError> {
        let yielder_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");
        let yielder_id = match get_yielder_id_from_address(txn, yielder_address).await {
            Some(id) => id,
            None => {
                error!(
                    "yielder.provision => did not find yielder matching address: {}",
                    yielder_address
                );
                return Err(DomainError::ContractNotFound(yielder_address.to_string()));
            }
        };

        let amount = U256::from(FieldElement::from_hex(event.payload.get("1").unwrap()).unwrap());
        let time = felt_to_offset_datetime(event.payload.get("2").expect("should have time"));

        add_provision_to_yielder(txn, yielder_id, amount, time).await?;
        Ok(())
    }
}

/// Consuming [`Snapshot`] [`Yielder`] on chain
#[derive(Default, Debug)]
pub struct YielderSnapshotEventConsumer {}
impl YielderSnapshotEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for YielderSnapshotEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Yielder(YielderEvents::Snapshot))
    }

    async fn consume(&self, event: &DomainEvent, txn: &mut Transaction) -> Result<(), DomainError> {
        let yielder_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");
        let yielder_id = match get_yielder_id_from_address(txn, yielder_address).await {
            Some(id) => id,
            None => {
                error!(
                    "yielder.provision => did not find yielder matching address: {}",
                    yielder_address
                );
                return Err(DomainError::ContractNotFound(yielder_address.to_string()));
            }
        };

        let previous_time =
            felt_to_offset_datetime(event.payload.get("1").expect("should have previous_time"));
        let current_time =
            felt_to_offset_datetime(event.payload.get("5").expect("should have current_time"));

        let snapshot = Snapshot {
            id: Uuid::new_v4(),
            previous_time,
            previous_project_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("2")
                    .expect("should have previous_project_absorption"),
            )
            .unwrap()
            .into(),
            previous_offseter_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("3")
                    .expect("should have previous_offseter_absorption"),
            )
            .unwrap()
            .into(),
            previous_yielder_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("4")
                    .expect("should have previous_yielder_absorption"),
            )
            .unwrap()
            .into(),
            time: current_time,
            current_project_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("6")
                    .expect("should have current_project_absorption"),
            )
            .unwrap()
            .into(),
            current_offseter_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("7")
                    .expect("should have current_offseter_absorption"),
            )
            .unwrap()
            .into(),
            current_yielder_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("8")
                    .expect("should have current_yielder_absorption"),
            )
            .unwrap()
            .into(),
            project_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("9")
                    .expect("should have project_absorption"),
            )
            .unwrap()
            .into(),
            offseter_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("10")
                    .expect("should have offseter_absorption"),
            )
            .unwrap()
            .into(),
            yielder_absorption: FieldElement::from_hex(
                event
                    .payload
                    .get("11")
                    .expect("should have yielder_absorption"),
            )
            .unwrap()
            .into(),
            yielder_id: Some(yielder_id),
        };
        let _snapshot_added = add_snapshot_to_yielder(txn, &snapshot).await;
        Ok(())
    }
}

/// Consuming [`Withdraw`] event emitted from [`Yielder`] on chain
#[derive(Default, Debug)]
pub struct YielderWithdrawEventConsumer {}
impl YielderWithdrawEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for YielderWithdrawEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Yielder(YielderEvents::Withdraw))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}
