use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::FieldElement;
use bigdecimal::ToPrimitive;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;
use time::OffsetDateTime;

use crate::{
    domain::crypto::U256, infrastructure::postgres::event_source::add_provision_to_yielder,
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
        let contracts = self.extract_addresses(address_list, &["yielder"]);
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
                        FieldElement::from_bytes(&selector!("Vesting").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Provision),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Withdraw").to_bytes_be()).to_string(),
                        Event::Yielder(YielderEvents::Withdraw),
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
        let contract_address = event
            .metadata
            .get("from_address")
            .expect("should have from_address");
        let amount = U256::from(FieldElement::from_hex(event.payload.get("1").unwrap()).unwrap());
        let time_millis =
            U256::from(FieldElement::from_hex(event.payload.get("1").unwrap()).unwrap());

        let date_time =
            OffsetDateTime::from_unix_timestamp(time_millis.to_big_decimal(0).to_i64().unwrap())
                .unwrap();

        add_provision_to_yielder(txn, contract_address, amount, date_time).await?;
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
