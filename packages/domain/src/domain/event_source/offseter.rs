use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::FieldElement;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;

use crate::{
    domain::crypto::U256,
    infrastructure::postgres::{
        entity::{ActionType, FarmType},
        event_source::{append_customer_action, find_related_project_address_and_slot},
    },
};

use super::{
    event_bus::Consumer, get_event, to_filters, BlockMetadata, DomainError, DomainEvent, Event,
    Filterable,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OffseterEvents {
    Upgraded,
    Deposit,
    Withdraw,
    Claim,
}

/// Base struct for [`Yielder`] to enable [`Filterable`] behaviour
#[derive(Debug)]
pub struct OffsetFilters {
    contracts: Vec<String>,
    filters: HashMap<String, Vec<(String, Event)>>,
}

impl OffsetFilters {
    pub fn new() -> Self {
        Self {
            contracts: vec![],
            filters: HashMap::new(),
        }
    }
}
impl Default for OffsetFilters {
    fn default() -> Self {
        Self::new()
    }
}

/// Add [`Filterable`] behaviour on [`Yielder`]
impl Filterable for OffsetFilters {
    fn to_filters(&self) -> Vec<(String, String)> {
        to_filters(&self.filters)
    }

    fn get_event(&mut self, contract_address: &str, event_key: &str) -> Option<Event> {
        get_event(&mut self.filters, contract_address, event_key)
    }

    fn hydrate_from_file(&mut self, address_list: Vec<HashMap<String, String>>) {
        let contracts = self.extract_addresses(address_list, &["offseter", "offseter_3525"]);
        self.contracts = contracts;
        for contract in self.contracts.iter() {
            self.filters.insert(
                contract.to_string(),
                [
                    (
                        FieldElement::from_bytes(&selector!("Upgraded").to_bytes_be()).to_string(),
                        Event::Offseter(OffseterEvents::Upgraded),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Deposit").to_bytes_be()).to_string(),
                        Event::Offseter(OffseterEvents::Deposit),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Withdraw").to_bytes_be()).to_string(),
                        Event::Offseter(OffseterEvents::Withdraw),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Claim").to_bytes_be()).to_string(),
                        Event::Offseter(OffseterEvents::Claim),
                    ),
                ]
                .to_vec(),
            );
        }
    }
}

/// Consuming [`Upgraded`] event emitted from [`Offseter`] on chain
#[derive(Default, Debug)]
pub struct OffseterUpgradedEventConsumer {}
impl OffseterUpgradedEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for OffseterUpgradedEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Offseter(OffseterEvents::Upgraded))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _metadata: &BlockMetadata,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}

/// Consuming [`Deposit`] event emitted from [`Offseter`] on chain
#[derive(Default, Debug)]
pub struct OffseterDepositEventConsumer {}
impl OffseterDepositEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for OffseterDepositEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Offseter(OffseterEvents::Deposit))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        let from_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");
        let (project_address, slot) =
            find_related_project_address_and_slot(txn, &from_address, FarmType::Offset).await?;
        let customer_address = event
            .payload
            .get("0")
            .expect("should have customer_address");
        let value = U256::from(
            FieldElement::from_hex(event.payload.get("1").expect("should have value")).unwrap(),
        );

        append_customer_action(
            txn,
            event.id.as_str(),
            metadata.timestamp,
            customer_address,
            &project_address,
            &slot,
            &value,
            FarmType::Offset,
            ActionType::Deposit,
        )
        .await?;

        Ok(())
    }
}

/// Consuming [`Withdraw`] event emitted from [`Offseter`] on chain
#[derive(Default, Debug)]
pub struct OffseterWithdrawEventConsumer {}
impl OffseterWithdrawEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for OffseterWithdrawEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Offseter(OffseterEvents::Withdraw))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        let from_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");
        let (project_address, slot) =
            find_related_project_address_and_slot(txn, &from_address, FarmType::Offset).await?;
        let customer_address = event
            .payload
            .get("0")
            .expect("should have customer_address");
        let value = U256::from(
            FieldElement::from_hex(event.payload.get("1").expect("should have value")).unwrap(),
        );

        append_customer_action(
            txn,
            event.id.as_str(),
            metadata.timestamp,
            customer_address,
            &project_address,
            &slot,
            &value,
            FarmType::Offset,
            ActionType::Withdraw,
        )
        .await?;

        Ok(())
    }
}

/// Consuming [`Claim`] event emitted from [`Offseter`] on chain
#[derive(Default, Debug)]
pub struct OffseterClaimEventConsumer {}
impl OffseterClaimEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for OffseterClaimEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Offseter(OffseterEvents::Claim))
    }

    async fn consume(
        &self,
        event: &DomainEvent,
        metadata: &BlockMetadata,
        txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        let from_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");
        let (project_address, slot) =
            find_related_project_address_and_slot(txn, &from_address, FarmType::Offset).await?;
        let customer_address = event
            .payload
            .get("0")
            .expect("should have customer_address");
        let value = U256::from(
            FieldElement::from_hex(event.payload.get("1").expect("should have value")).unwrap(),
        );

        append_customer_action(
            txn,
            event.id.as_str(),
            metadata.timestamp,
            customer_address,
            &project_address,
            &slot,
            &value,
            FarmType::Offset,
            ActionType::Claim,
        )
        .await?;

        Ok(())
    }
}
