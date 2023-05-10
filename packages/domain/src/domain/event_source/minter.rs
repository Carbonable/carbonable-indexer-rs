use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::FieldElement;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;

use super::{
    event_bus::Consumer, get_event, to_filters, DomainError, DomainEvent, Event, Filterable,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinterEvents {
    Upgraded,
    Airdrop,
    Buy,
    SoldOut,
    Migration,
    PreSaleOpen,
    PreSaleClosed,
    PublicSaleOpen,
    PublicSaleClosed,
}

/// Base struct for [`Minter`] to enable [`Filterable`] behaviour
#[derive(Debug)]
pub struct MinterFilters {
    contracts: Vec<String>,
    filters: HashMap<String, Vec<(String, Event)>>,
}

impl MinterFilters {
    pub fn new() -> Self {
        Self {
            contracts: vec![],
            filters: HashMap::new(),
        }
    }
}
impl Default for MinterFilters {
    fn default() -> Self {
        Self::new()
    }
}

/// Add [`Filterable`] behaviour on [`Minter`]
impl Filterable for MinterFilters {
    fn to_filters(&self) -> Vec<(String, String)> {
        to_filters(&self.filters)
    }

    fn get_event(&mut self, contract_address: &str, event_key: &str) -> Option<Event> {
        get_event(&mut self.filters, contract_address, event_key)
    }

    fn hydrate_from_file(&mut self, address_list: Vec<HashMap<String, String>>) {
        let contracts = self.extract_addresses(address_list, &["minter"]);
        self.contracts = contracts;
        for contract in self.contracts.iter() {
            self.filters.insert(
                contract.to_string(),
                [
                    (
                        FieldElement::from_bytes(&selector!("Migration").to_bytes_be()).to_string(),
                        Event::Minter(MinterEvents::Migration),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Airdrop").to_bytes_be()).to_string(),
                        Event::Minter(MinterEvents::Airdrop),
                    ),
                    (
                        FieldElement::from_bytes(&selector!("Buy").to_bytes_be()).to_string(),
                        Event::Minter(MinterEvents::Buy),
                    ),
                ]
                .to_vec(),
            );
        }
    }
}

/// Consuming [`Migration`] event emitted from [`Minter`] on chain
#[derive(Default, Debug)]
pub struct MinterMigrationEventConsumer {}
impl MinterMigrationEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for MinterMigrationEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Minter(MinterEvents::Migration))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // Migration(address: felt, tokenId: u256, newTokenId: u256, slot: u256, value: u256);
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}

/// Consuming [`Airdrop`] event emitted from [`Minter`] on chain
#[derive(Default, Debug)]
pub struct MinterAirdropEventConsumer {}
impl MinterAirdropEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for MinterAirdropEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Minter(MinterEvents::Airdrop))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // Airdrop(address: felt, quantity: felt, time: felt)
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}

/// Consuming [`Buy`] event emitted from [`Minter`] on chain
#[derive(Default, Debug)]
pub struct MinterBuyEventConsumer {}
impl MinterBuyEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for MinterBuyEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Minter(MinterEvents::Buy))
    }

    async fn consume(
        &self,
        _event: &DomainEvent,
        _txn: &mut Transaction,
    ) -> Result<(), DomainError> {
        // Airdrop(address: felt, amount: u256, quantity: felt, time: felt)
        // event not handled at the moment but it will be stored in database later on.
        Ok(())
    }
}
