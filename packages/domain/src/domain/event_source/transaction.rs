use std::{collections::HashMap, sync::Mutex};

use deadpool_postgres::Transaction;

use super::DomainEvent;

pub enum TransactionManagerError {}

#[async_trait::async_trait]
pub trait TransactionManager {
    async fn commit(&self) -> Result<(), TransactionManagerError>;
    async fn rollback(&self) -> Result<(), TransactionManagerError>;
    async fn execute<T: Sync, P: Sync>(
        &self,
        statement: &T,
        params: &[P],
    ) -> Result<u64, TransactionManagerError>;
}

#[async_trait::async_trait]
impl TransactionManager for Transaction<'_> {
    async fn commit(&self) -> Result<(), TransactionManagerError> {
        Ok(self.commit().await?)
    }

    async fn rollback(&self) -> Result<(), TransactionManagerError> {
        Ok(self.rollback().await?)
    }

    async fn execute<T: std::marker::Sync, P: std::marker::Sync>(
        &self,
        statement: &T,
        params: &[P],
    ) -> Result<u64, TransactionManagerError> {
        Ok(self.execute(statement, params).await?)
    }
}

#[async_trait::async_trait]
impl TransactionManager for Mutex<HashMap<String, Vec<DomainEvent>>> {
    async fn commit(&self) -> Result<(), TransactionManagerError> {
        todo!()
    }

    async fn rollback(&self) -> Result<(), TransactionManagerError> {
        todo!()
    }

    async fn execute<T: std::marker::Sync, P: std::marker::Sync>(
        &self,
        _statement: &T,
        _params: &[P],
    ) -> Result<u64, TransactionManagerError> {
        todo!()
    }
}
