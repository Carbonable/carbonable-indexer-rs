use deadpool_postgres::Transaction;
use deadpool_postgres::{GenericClient, Object, Pool};
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use serde_json::json;
use time::OffsetDateTime;
use tracing::{debug, error};

use crate::domain::event_source::BlockMetadata;
use crate::domain::{
    crypto::U256,
    event_source::{DomainError, DomainEvent, StorageClientPool},
};
use std::sync::Arc;

use super::entity::EventStoreIden;
use super::{entity::CustomerTokenIden, PostgresError};

#[derive(Debug)]
pub struct PostgresStorageClientPool {
    pub db_client_pool: Arc<Pool>,
}
impl PostgresStorageClientPool {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }
}

#[async_trait::async_trait]
impl StorageClientPool for PostgresStorageClientPool {
    type Client<'a> = Object;

    async fn get(&self) -> Result<Self::Client<'_>, DomainError> {
        Ok(self.db_client_pool.get().await?)
    }
}

/// Insert last domain event after all consumers did properly handled last domain event
///
/// * tx: [`deadpool_postgres::Transaction`]
/// * event: [`DomainEvent`]
pub async fn insert_last_domain_event<'a>(
    tx: &Transaction<'a>,
    event: &DomainEvent,
    metadata: &BlockMetadata,
) -> Result<(), PostgresError> {
    let id = uuid::Uuid::new_v4();
    let block_number = U256::from(metadata.number);
    let (sql, values) = Query::insert()
        .into_table(EventStoreIden::Table)
        .columns([
            EventStoreIden::Id,
            EventStoreIden::EventId,
            EventStoreIden::BlockNumber,
            EventStoreIden::BlockHash,
            EventStoreIden::Metadata,
            EventStoreIden::Payload,
            EventStoreIden::RecordedAt,
        ])
        .values([
            id.into(),
            event.id.clone().into(),
            block_number.into(),
            metadata.hash.clone().into(),
            sea_query::Value::Json(Some(Box::new(json!(&event.metadata)))).into(),
            sea_query::Value::Json(Some(Box::new(json!(&event.payload)))).into(),
            OffsetDateTime::now_utc().into(),
        ])?
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("event_store.domain_event.create: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("event_store.domain_event.create: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

/// From blockchain `Transfer` event feeds data into database
///
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`String`]
/// * to: [`String`]
/// * token_id: [`String`]
///
pub async fn create_token_for_customer<'a>(
    tx: &Transaction<'a>,
    contract_address: &str,
    to: &str,
    token_id: &U256,
) -> Result<(), PostgresError> {
    let id = uuid::Uuid::new_v4();
    let (sql, values) = Query::insert()
        .into_table(CustomerTokenIden::Table)
        .columns([
            CustomerTokenIden::Id,
            CustomerTokenIden::Address,
            CustomerTokenIden::ProjectAddress,
            CustomerTokenIden::TokenId,
        ])
        .values([
            id.into(),
            to.into(),
            contract_address.into(),
            (<U256 as Into<sea_query::SimpleExpr>>::into(*token_id)),
        ])?
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("project.transfer.create_result: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("project.transfer.create: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

/// From blockchain `Transfer` event updates database
///
/// * tx: [`deadpool_postgres::Object`]
/// * from: [`String`]
/// * contract_address: [`String`]
/// * to: [`String`]
/// * token_id: [`U256`]
///
pub async fn update_token_owner<'a>(
    tx: &Transaction<'a>,
    from: &str,
    contract_address: &str,
    to: &str,
    token_id: &U256,
) -> Result<(), PostgresError> {
    let (sql, values) = Query::update()
        .table(CustomerTokenIden::Table)
        .and_where(Expr::col(CustomerTokenIden::Address).eq(from))
        .and_where(Expr::col(CustomerTokenIden::TokenId).eq(*token_id))
        .and_where(Expr::col(CustomerTokenIden::ProjectAddress).eq(contract_address))
        .values([(CustomerTokenIden::Address, to.into())])
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("project.transfer.update: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("project.transfer.update: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

/// From blockchain `TransferValue` event updates database
///
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`String`]
/// * to: [`U256`]
/// * token_id: [`U256`]
///
pub async fn update_token_value<'a>(
    tx: &Transaction<'a>,
    contract_address: &str,
    token_id: &U256,
    value: &U256,
) -> Result<(), PostgresError> {
    let (sql, values) = Query::update()
        .table(CustomerTokenIden::Table)
        .and_where(Expr::col(CustomerTokenIden::TokenId).eq(*token_id))
        .and_where(Expr::col(CustomerTokenIden::ProjectAddress).eq(contract_address))
        .values([
            (CustomerTokenIden::TokenId, token_id.into()),
            (CustomerTokenIden::Value, value.into()),
        ])
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("project.transfer_value.update: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("project.transfer_value.update: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

/// From blockchain `SlotChanged` event updates database
///
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`String`]
/// * token_id: [`U256`]
/// * slot: [`U256`]
///
pub async fn update_token_slot<'a>(
    tx: &Transaction<'a>,
    contract_address: &str,
    token_id: &U256,
    slot: &U256,
) -> Result<(), PostgresError> {
    let (sql, values) = Query::update()
        .table(CustomerTokenIden::Table)
        .and_where(Expr::col(CustomerTokenIden::TokenId).eq(*token_id))
        .and_where(Expr::col(CustomerTokenIden::ProjectAddress).eq(contract_address))
        .values([(CustomerTokenIden::Slot, slot.into())])
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("project.transfer_value.update: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("project.transfer_value.update: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}
