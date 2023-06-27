pub mod event_bus;
pub mod minter;
pub mod offseter;
pub mod project;
pub mod transaction;
pub mod yielder;

use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

use crate::infrastructure::{
    postgres::PostgresError,
    starknet::{model::ModelError, SequencerError},
};

use self::{
    minter::MinterEvents, offseter::OffseterEvents, project::ProjectEvents, yielder::YielderEvents,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    pub(crate) id: String,
    pub(crate) metadata: HashMap<String, String>,
    pub(crate) payload: HashMap<String, String>,
    pub(crate) r#type: Event,
}
impl DomainEvent {
    pub fn with_metadata(mut self, metadata: &BlockMetadata) -> Self {
        self.metadata
            .insert("block_hash".to_owned(), metadata.hash.to_string());
        self.metadata
            .insert("block_number".to_owned(), metadata.number.to_string());
        self.metadata
            .insert("timestamp".to_owned(), metadata.timestamp.to_string());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockMetadata {
    pub(crate) hash: String,
    pub(crate) timestamp: OffsetDateTime,
    pub(crate) number: u64,
}

impl BlockMetadata {
    pub fn get_block(&self) -> u64 {
        self.number
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Project(ProjectEvents),
    Minter(MinterEvents),
    Offseter(OffseterEvents),
    Yielder(YielderEvents),
}

impl From<Event> for &str {
    fn from(value: Event) -> Self {
        match value {
            Event::Project(v) => match v {
                ProjectEvents::Upgraded => "project.upgraded",
                ProjectEvents::AbsorptionUpdate => "project.absorption_update",
                ProjectEvents::Transfer => "project.transfer",
                ProjectEvents::TransferValue => "project.transfer_value",
                ProjectEvents::Approval => "project.approval",
                ProjectEvents::ApprovalValue => "project.approval_value",
                ProjectEvents::ApprovalForAll => "project.approval_for_all",
                ProjectEvents::SlotChanged => "project.slot_changed",
                ProjectEvents::MetadataUpdate => "project.metadata_update",
                ProjectEvents::BatchMetadataUpdate => "project.batch_metadata_update",
                ProjectEvents::ProjectValueUpdate => "project.project_value_update",
            },
            Event::Minter(v) => match v {
                MinterEvents::Upgraded => "minter.upgraded",
                MinterEvents::Airdrop => "minter.airdrop",
                MinterEvents::Buy => "minter.buy",
                MinterEvents::SoldOut => "minter.sold_out",
                MinterEvents::Migration => "minter.migration",
                MinterEvents::PreSaleOpen => "minter.pre_sale_open",
                MinterEvents::PreSaleClosed => "minter.pre_sale_closed",
                MinterEvents::PublicSaleOpen => "minter.public_sale_open",
                MinterEvents::PublicSaleClosed => "minter.public_sale_closed",
            },
            Event::Offseter(v) => match v {
                OffseterEvents::Upgraded => "offseter.upgraded",
                OffseterEvents::Deposit => "offseter.deposit",
                OffseterEvents::Withdraw => "offseter.withdraw",
                OffseterEvents::Claim => "offseter.claim",
            },
            Event::Yielder(v) => match v {
                YielderEvents::Upgraded => "yielder.upgraded",
                YielderEvents::Claim => "yielder.claim",
                YielderEvents::Deposit => "yielder.deposit",
                YielderEvents::Withdraw => "yielder.withdraw",
                YielderEvents::PriceUpdate => "yielder.price_update",
            },
        }
    }
}
impl From<&str> for Event {
    fn from(value: &str) -> Self {
        match value {
            "project.upgraded" => Event::Project(ProjectEvents::Upgraded),
            "project.absorption_update" => Event::Project(ProjectEvents::AbsorptionUpdate),
            "project.transfer" => Event::Project(ProjectEvents::Transfer),
            "project.transfer_value" => Event::Project(ProjectEvents::TransferValue),
            "project.approval" => Event::Project(ProjectEvents::Approval),
            "project.approval_value" => Event::Project(ProjectEvents::ApprovalValue),
            "project.approval_for_all" => Event::Project(ProjectEvents::ApprovalForAll),
            "project.slot_changed" => Event::Project(ProjectEvents::SlotChanged),
            "project.metadata_update" => Event::Project(ProjectEvents::MetadataUpdate),
            "project.batch_metadata_update" => Event::Project(ProjectEvents::BatchMetadataUpdate),
            "project.project_value_update" => Event::Project(ProjectEvents::ProjectValueUpdate),
            "minter.upgraded" => Event::Minter(MinterEvents::Upgraded),
            "minter.airdrop" => Event::Minter(MinterEvents::Airdrop),
            "minter.buy" => Event::Minter(MinterEvents::Buy),
            "minter.sold_out" => Event::Minter(MinterEvents::SoldOut),
            "minter.migration" => Event::Minter(MinterEvents::Migration),
            "minter.pre_sale_open" => Event::Minter(MinterEvents::PreSaleOpen),
            "minter.pre_sale_closed" => Event::Minter(MinterEvents::PreSaleClosed),
            "minter.public_sale_open" => Event::Minter(MinterEvents::PublicSaleOpen),
            "minter.public_sale_closed" => Event::Minter(MinterEvents::PublicSaleClosed),
            "offseter.upgraded" => Event::Offseter(OffseterEvents::Upgraded),
            "offseter.deposit" => Event::Offseter(OffseterEvents::Deposit),
            "offseter.withdraw" => Event::Offseter(OffseterEvents::Withdraw),
            "offseter.claim" => Event::Offseter(OffseterEvents::Claim),
            "yielder.claim" => Event::Yielder(YielderEvents::Claim),
            "yielder.upgraded" => Event::Yielder(YielderEvents::Upgraded),
            "yielder.deposit" => Event::Yielder(YielderEvents::Deposit),
            "yielder.withdraw" => Event::Yielder(YielderEvents::Withdraw),
            "yielder.price_udpate" => Event::Yielder(YielderEvents::PriceUpdate),
            &_ => panic!("Unknown event {value}"),
        }
    }
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error(transparent)]
    PoolError(#[from] deadpool_postgres::PoolError),
    #[error(transparent)]
    TokioError(#[from] tokio_postgres::Error),
    #[error(transparent)]
    PostgresError(#[from] PostgresError),
    #[error("feature not available there")]
    NotAvailable,
    #[error("contract with address {0} not found inside db")]
    ContractNotFound(String),
    #[error(transparent)]
    SequencerError(#[from] SequencerError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
}

#[async_trait::async_trait]
pub trait StorageClientPool {
    type Client<'a>
    where
        Self: 'a;

    async fn get(&self) -> Result<Self::Client<'_>, DomainError>;
}

/// Implement this trait to enable specific filtering.
pub trait Filterable: Debug {
    /// Maps a single `contract_address` to `selector_hash`
    fn to_filters(&self) -> Vec<(String, String)>;

    /// Tries to find event in current filter.
    fn get_event(&mut self, contract_address: &str, event_key: &str) -> Option<Event>;

    /// Build filter item from configuration filter
    fn hydrate_from_file(&mut self, address_list: Vec<HashMap<String, String>>);

    /// Extract from file data
    fn extract_addresses(
        &self,
        contract_addresses: Vec<HashMap<String, String>>,
        keys: &[&str],
    ) -> Vec<String> {
        let mut addresses = Vec::new();
        for list in contract_addresses {
            for (k, addr) in list.iter() {
                if keys.contains(&k.as_str()) {
                    addresses.push(addr.to_string());
                }
            }
        }
        addresses
    }
}

/// Common function for [`Filterable::to_filters`] trait implementation
/// * filters: &HashMap<contract_address, Vec<(selector_hash, Event)>>
///
pub(crate) fn to_filters(filters: &HashMap<String, Vec<(String, Event)>>) -> Vec<(String, String)> {
    filters
        .iter()
        .flat_map(|(k, v)| {
            v.iter()
                .map(|(selector_hash, _)| (k.to_owned(), selector_hash.to_owned()))
        })
        .collect()
}

/// Common function for [`Filterable::get_event`] trait implementation
/// * filters: &mut HashMap<contract_address, Vec<(selector_hash, Event)>>
/// * contract_address: &str
/// * event_key: &str
///
pub(crate) fn get_event(
    filters: &mut HashMap<String, Vec<(String, Event)>>,
    contract_address: &str,
    event_key: &str,
) -> Option<Event> {
    match filters.entry(contract_address.to_string()) {
        Entry::Occupied(e) => e
            .get()
            .iter()
            .find(|(k, _)| &event_key.to_string() == k)
            .map(|(_, ev)| ev.clone()),
        Entry::Vacant(_) => None,
    }
}
