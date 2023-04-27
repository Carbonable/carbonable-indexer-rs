pub mod event_bus;
pub mod minter;
pub mod offseter;
pub mod project;
pub mod transaction;
pub mod vester;
pub mod yielder;

use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::infrastructure::postgres::PostgresError;

use self::{
    minter::MinterEvents, offseter::OffseterEvents, project::ProjectEvents, vester::VesterEvents,
    yielder::YielderEvents,
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
    pub(crate) timestamp: String,
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
    Vester(VesterEvents),
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
