use deadpool_postgres::{GenericClient, Object, Pool};

use crate::domain::event_source::{DomainError, StorageManager};

use std::sync::Arc;

pub struct PostgresStorageManager {
    pub db_client_pool: Arc<Pool>,
}

#[async_trait::async_trait]
impl StorageManager for PostgresStorageManager {
    type Client = Object;
    type Transaction<'a> = deadpool_postgres::Transaction<'a>;

    async fn get_last_block(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_client(&self) -> Self::Client {
        self.db_client_pool.get().await.unwrap()
    }
    async fn store_events(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn build_transaction(&self) -> Self::Transaction<'_> {
        todo!()
    }
}
