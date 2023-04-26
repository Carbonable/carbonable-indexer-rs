use crate::{
    domain::crypto::U256,
    infrastructure::postgres::event_source::{create_token_for_customer, update_token_owner},
};
use apibara_core::starknet::v1alpha2::FieldElement;
use crypto_bigint::Encoding;
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};

use super::{event_bus::Consumer, DomainError, DomainEvent, Event, Filterable};

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

impl Filterable for ProjectFilters {
    fn to_filters(&self) -> Vec<(String, String)> {
        self.filters
            .iter()
            .flat_map(|(k, v)| {
                v.iter()
                    .map(|(selector_hash, _)| (k.to_owned(), selector_hash.to_owned()))
            })
            .collect()
    }

    fn get_event(&mut self, contract_address: &str, event_key: &str) -> Option<Event> {
        match &self.filters.entry(contract_address.to_string()) {
            Entry::Occupied(e) => e
                .get()
                .iter()
                .find(|(k, _)| &event_key.to_string() == k)
                .map(|(_, ev)| ev.clone()),
            Entry::Vacant(_) => None,
        }
    }

    fn hydrate_from_file<I>(&mut self, address_list: I) -> &mut Self
    where
        I: IntoIterator<Item = HashMap<String, String>>,
    {
        let contracts =
            self.extract_addresses(address_list.into_iter(), &["project", "project_3525"]);
        self.contracts = contracts;
        for contract in self.contracts.iter() {
            self.filters.insert(
                contract.to_string(),
                [
                    (
                        FieldElement::from_bytes(&selector!("Transfer").to_bytes_be()).to_string(),
                        Event::Project(ProjectEvents::Transfer),
                    ),
                    // (
                    //     FieldElement::from_bytes(&selector!("TransferValue").to_bytes_be())
                    //         .to_string(),
                    //     Event::Project(ProjectEvents::TransferValue),
                    // ),
                ]
                .to_vec(),
            );
        }
        self
    }
}

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

    async fn consume(&self, event: &DomainEvent, txn: &mut Transaction) -> Result<(), DomainError> {
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

        if &FieldElement::from_u64(0).to_string() == from {
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
        tx: &mut Mutex<HashMap<String, Vec<DomainEvent>>>,
    ) -> Result<(), DomainError> {
        let lock = tx.get_mut().unwrap();

        lock.entry("project-transfer".to_string())
            .or_insert(vec![])
            .push(event.clone());

        Ok(())
    }
}

// #[derive(Default, Debug)]
// pub struct ProjectTransferValueEventConsumer {}
//
// impl ProjectTransferValueEventConsumer {
//     pub fn new() -> Self {
//         Self {}
//     }
// }
//
// #[async_trait::async_trait]
// impl Consumer<PostgresStorageClientPool, Transaction<'_>> for ProjectTransferValueEventConsumer {
//     fn can_consume(&self, event: &Event, _storage_manager: &PostgresStorageClientPool) -> bool {
//         match event {
//             Event::Project(ProjectEvents::TransferValue) => true,
//             _ => false,
//         }
//     }
//
//     async fn consume(
//         &self,
//         event: &DomainEvent,
//         storage_manager: &Transaction,
//     ) -> Result<(), DomainError> {
//         // let client = storage_manager.get().await?;
//
//         // println!("{:#?}", client);
//         println!("{:#?}", event);
//         todo!()
//     }
// }
//
// #[async_trait::async_trait]
// impl Consumer<InMemoryDomainClientPool, ()> for ProjectTransferValueEventConsumer {
//     fn can_consume(&self, event: &Event, _storage_manager: &InMemoryDomainClientPool) -> bool {
//         match event {
//             Event::Project(ProjectEvents::TransferValue) => true,
//             _ => false,
//         }
//     }
//
//     async fn consume(&self, event: &DomainEvent, storage_manager: &()) -> Result<(), DomainError> {
//         println!(
//             "ProjectTransferValueEventConsumer handling value : {:#?}",
//             event
//         );
//         Ok(())
//     }
// }
