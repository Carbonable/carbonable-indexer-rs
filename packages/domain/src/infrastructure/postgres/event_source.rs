use deadpool_postgres::Transaction;
use deadpool_postgres::{GenericClient, Object, Pool};
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use serde_json::json;
use time::OffsetDateTime;
use tokio_postgres::error::SqlState;
use tracing::{debug, error};
use uuid::Uuid;

use crate::domain::event_source::BlockMetadata;
use crate::domain::{
    crypto::U256,
    event_source::{DomainError, DomainEvent, StorageClientPool},
};
use std::sync::Arc;

use super::entity::{EventStoreIden, ProvisionIden, Snapshot, SnapshotIden, YielderIden};
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
            EventStoreIden::RType,
            EventStoreIden::RecordedAt,
        ])
        .values([
            id.into(),
            event.id.clone().into(),
            block_number.into(),
            metadata.hash.clone().into(),
            sea_query::Value::Json(Some(Box::new(json!(&event.metadata)))).into(),
            sea_query::Value::Json(Some(Box::new(json!(&event.payload)))).into(),
            event.r#type.clone().into(),
            metadata.timestamp.into(),
        ])?
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("event_store.domain_event.create: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            if e.code().eq(&Some(&SqlState::UNIQUE_VIOLATION)) {
                debug!("event_store.domain_event.create: ignored due to duplication");
                return Ok(());
            }
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

/// From blockchain `Provision` event updates database
///
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`&str`]
/// * amount: [`U256`]
/// * time: [`OffsetDateTime`]
///
pub async fn add_provision_to_yielder<'a>(
    tx: &Transaction<'a>,
    yielder_id: Uuid,
    amount: U256,
    time: OffsetDateTime,
) -> Result<(), PostgresError> {
    let id = uuid::Uuid::new_v4();
    let (sql, values) = Query::insert()
        .into_table(ProvisionIden::Table)
        .columns([
            ProvisionIden::Id,
            ProvisionIden::Amount,
            ProvisionIden::Time,
            ProvisionIden::YielderId,
        ])
        .values([id.into(), amount.into(), time.into(), yielder_id.into()])?
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("yielder.provision: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("yielder.provision: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

/// From blockchain `Snapshot` event updates database
///
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`&str`]
/// * amount: [`U256`]
/// * time: [`OffsetDateTime`]
///
pub async fn add_snapshot_to_yielder<'a>(
    tx: &Transaction<'a>,
    snapshot: &Snapshot,
) -> Result<(), PostgresError> {
    let id = uuid::Uuid::new_v4();
    let (sql, values) = Query::insert()
        .into_table(SnapshotIden::Table)
        .columns([
            SnapshotIden::Id,
            SnapshotIden::PreviousTime,
            SnapshotIden::PreviousProjectAbsorption,
            SnapshotIden::PreviousOffseterAbsorption,
            SnapshotIden::PreviousYielderAbsorption,
            SnapshotIden::CurrentProjectAbsorption,
            SnapshotIden::CurrentOffseterAbsorption,
            SnapshotIden::CurrentYielderAbsorption,
            SnapshotIden::ProjectAbsorption,
            SnapshotIden::OffseterAbsorption,
            SnapshotIden::YielderAbsorption,
            SnapshotIden::Time,
            SnapshotIden::YielderId,
        ])
        .values([
            id.into(),
            snapshot.previous_time.into(),
            snapshot.previous_project_absorption.into(),
            snapshot.previous_offseter_absorption.into(),
            snapshot.previous_yielder_absorption.into(),
            snapshot.current_project_absorption.into(),
            snapshot.current_offseter_absorption.into(),
            snapshot.current_yielder_absorption.into(),
            snapshot.project_absorption.into(),
            snapshot.offseter_absorption.into(),
            snapshot.yielder_absorption.into(),
            snapshot.time.into(),
            snapshot.yielder_id.into(),
        ])?
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(sql.as_str(), &values.as_params()).await {
        Ok(res) => {
            debug!("yielder.snapshot: {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("yielder.snapshot: {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

/// Get yielder id from blockchain address
///
/// * tx: [`deadpool_postgres::Object`]
/// * yielder_address: [`&str`]
///
pub async fn get_yielder_id_from_address<'a>(
    tx: &Transaction<'a>,
    yielder_address: &str,
) -> Option<Uuid> {
    let (sql, values) = Query::select()
        .from(YielderIden::Table)
        .column(YielderIden::Id)
        .and_where(Expr::col(YielderIden::Address).eq(yielder_address))
        .build_postgres(PostgresQueryBuilder);

    match tx.query_one(&sql, &values.as_params()).await {
        Ok(res) => {
            let yielder_id: Uuid = res.get(0);
            Some(yielder_id)
        }
        Err(_) => None,
    }
}
