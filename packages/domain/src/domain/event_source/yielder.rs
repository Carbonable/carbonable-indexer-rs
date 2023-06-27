use std::collections::HashMap;

use apibara_core::starknet::v1alpha2::FieldElement;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;
use tracing::error;

use crate::infrastructure::{
    postgres::event_source::{get_yielder_id_from_address, update_yielder_prices},
    starknet::{
        get_starknet_rpc_from_env,
        model::{parallelize_blockchain_rpc_calls, StarknetValue, StarknetValueResolver},
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
    PriceUpdate,
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
                        FieldElement::from_bytes(&selector!("PriceUpdate").to_bytes_be())
                            .to_string(),
                        Event::Yielder(YielderEvents::PriceUpdate),
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

/// Consuming [`PriceUpdate`] event emitted from [`Yielder`] on chain
#[derive(Default, Debug)]
pub struct YielderPriceUpdateEventConsumer {}
impl YielderPriceUpdateEventConsumer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl Consumer<Transaction<'_>> for YielderPriceUpdateEventConsumer {
    fn can_consume(&self, event: &Event) -> bool {
        matches!(event, Event::Yielder(YielderEvents::PriceUpdate))
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

        let provider = get_starknet_rpc_from_env()?;
        let values = [(yielder_address.to_string(), "getPrices", vec![])];
        let data = parallelize_blockchain_rpc_calls(provider.into(), values.to_vec()).await?;
        let prices = StarknetValue::new(data[0].clone()).resolve("u256_array");

        update_yielder_prices(txn, yielder_id, prices).await?;
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
