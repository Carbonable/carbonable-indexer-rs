use std::{collections::HashMap, sync::Arc};

use apibara_core::starknet::v1alpha2::FieldElement;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;

use crate::{
    domain::crypto::U256,
    infrastructure::{
        postgres::event_source::{migrate_customer_token, update_project_total_value},
        starknet::{
            get_starknet_rpc_from_env,
            model::{parallelize_blockchain_rpc_calls, u256_to_felt},
        },
    },
};

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

    async fn consume(&self, event: &DomainEvent, txn: &mut Transaction) -> Result<(), DomainError> {
        // Migration(address: felt, tokenId: u256, newTokenId: u256, slot: u256, value: u256);
        // Catch migration to update total_supply on project if
        let minter_721 = event
            .metadata
            .get("from_address")
            .expect("should have from address");
        let customer_address = event.payload.get("0").expect("should have slot");
        let token_id = U256::from(
            FieldElement::from_hex(event.payload.get("1").expect("should have token_id")).unwrap(),
        );
        let new_token_id = U256::from(
            FieldElement::from_hex(event.payload.get("3").expect("should have new_token_id"))
                .unwrap(),
        );
        let slot = U256::from(
            FieldElement::from_hex(event.payload.get("5").expect("should have slot")).unwrap(),
        );
        let value = U256::from(
            FieldElement::from_hex(event.payload.get("7").expect("should have value")).unwrap(),
        );

        let provider = Arc::new(get_starknet_rpc_from_env()?);

        let data = parallelize_blockchain_rpc_calls(
            provider.clone(),
            [
                (minter_721.to_string(), "getMigrationTargetAddress", vec![]),
                (minter_721.to_string(), "getMigrationSourceAddress", vec![]),
            ]
            .to_vec(),
        )
        .await?;
        let project_address =
            FieldElement::from_bytes(&data[0].clone().first().unwrap().to_bytes_be());
        let from_project_address =
            FieldElement::from_bytes(&data[1].clone().first().unwrap().to_bytes_be());

        let data = parallelize_blockchain_rpc_calls(
            provider.clone(),
            [(
                project_address.to_hex(),
                "totalValue",
                vec![
                    u256_to_felt(&slot),
                    starknet::core::types::FieldElement::ZERO,
                ],
            )]
            .to_vec(),
        )
        .await?;

        let total_value = U256::from(FieldElement::from_bytes(
            &data[0].clone().first().unwrap().to_bytes_be(),
        ));

        let _ =
            update_project_total_value(txn, &project_address.to_hex(), &slot, &total_value).await?;

        let _ = migrate_customer_token(
            txn,
            &project_address.to_hex(),
            &from_project_address.to_hex(),
            customer_address,
            &token_id,
            &new_token_id,
            &slot,
            &value,
        )
        .await?;

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
