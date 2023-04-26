use std::{collections::HashMap, sync::Mutex};

use crate::domain::event_source::{DomainError, DomainEvent, StorageClientPool};

#[derive(Debug)]
pub struct InMemoryDomainClientPool {
    pub client: Mutex<HashMap<String, Vec<DomainEvent>>>,
}

impl InMemoryDomainClientPool {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryDomainClientPool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StorageClientPool for InMemoryDomainClientPool {
    type Client<'a> = HashMap<String, Vec<DomainEvent>>;

    async fn get<'a>(&'a self) -> Result<Self::Client<'a>, DomainError> {
        Ok(self.client.lock().unwrap().clone())
    }
}
