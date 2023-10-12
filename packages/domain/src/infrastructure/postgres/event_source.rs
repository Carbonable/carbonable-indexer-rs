use crate::domain::Ulid;
use deadpool_postgres::Transaction;
use deadpool_postgres::{GenericClient, Object, Pool};
use sea_query::{Expr, Func, Iden, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use serde_json::json;
use time::OffsetDateTime;
use tokio_postgres::error::SqlState;
use tracing::{debug, error};

use crate::domain::event_source::BlockMetadata;
use crate::domain::{
    crypto::U256,
    event_source::{DomainError, DomainEvent, StorageClientPool},
};
use std::sync::Arc;

use super::entity::{
    ActionType, CustomerFarmIden, EventStoreIden, FarmType, OffseterIden, ProjectIden,
    ProvisionIden, Snapshot, SnapshotIden, YielderIden,
};
use super::{entity::CustomerTokenIden, PostgresError};

pub struct PgDecodeFn;
impl Iden for PgDecodeFn {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "decode").unwrap();
    }
}

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
    let id = Ulid::new();
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

/// Check if event was already processed by indexer.
/// * client - [`&deadpool_postgres::Object`]
/// * event_id - [`&str`]
///
pub async fn event_was_processed(client: &Object, event_id: &str) -> bool {
    match client
        .query_one(
            r#"select id from event_store es where es.event_id = $1"#,
            &[&event_id],
        )
        .await
    {
        Ok(_r) => true,
        Err(_e) => false,
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
    let id = Ulid::new();
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
            token_id.into(),
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
    match tx.execute(
        r#"UPDATE "customer_token" SET "address" = $1 WHERE "token_id" = decode($2, $3) AND "project_address" = $4 AND "address" = $5"#
        , &[&to.to_string(), &token_id.to_string(), &"hex".to_string(), &contract_address.to_string(), &from.to_string()]).await {
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

/// From blockchain `Migration` event migrates customer token
///
/// * tx: [`deadpool_postgres::Object`]
/// * project_address: [`&str`]
/// * from_project_address: [`&str`]
/// * customer_address: [`&str`]
/// * token_id: [`U256`]
/// * new_token_id: [`U256`]
/// * slot: [`U256`]
/// * value: [`U256`]
///
pub async fn migrate_customer_token<'a>(
    tx: &Transaction<'a>,
    project_address: &str,
    from_project_address: &str,
    customer_address: &str,
    token_id: &U256,
    new_token_id: &U256,
    slot: &U256,
    value: &U256,
) -> Result<(), PostgresError> {
    let (sql, values) = Query::update()
        .table(CustomerTokenIden::Table)
        .and_where(
            Expr::col((CustomerTokenIden::Table, CustomerTokenIden::Address)).eq(customer_address),
        )
        .and_where(
            Expr::col((CustomerTokenIden::Table, CustomerTokenIden::ProjectAddress))
                .eq(from_project_address),
        )
        .and_where(Expr::col((CustomerTokenIden::Table, CustomerTokenIden::TokenId)).eq(token_id))
        .values([
            (CustomerTokenIden::ProjectAddress, project_address.into()),
            (CustomerTokenIden::TokenId, new_token_id.into()),
            (CustomerTokenIden::Slot, slot.into()),
            (CustomerTokenIden::Value, value.into()),
        ])
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(&sql, &values.as_params()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("minter.migration.error : {:#?}", e);
            Err(PostgresError::TokioPostgresError(e))
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
    to_token_id: &U256,
    value: U256,
) -> Result<(), PostgresError> {
    match tx.query_one(
        r#"SELECT "customer_token"."value" FROM "customer_token" WHERE "token_id" = decode($1, $2) AND "project_address" = $3"#
        , &[&to_token_id.to_string(), &"hex".to_string(), &contract_address.to_string()]).await {
        Ok(res) => {
            let val: Option<U256> = res.get(0);
            let new_value = match val {
                Some(v) => v + value,
                None => U256::zero() + value,
            };

            match tx.execute(
                r#"UPDATE "customer_token" set "value" = decode($1,$2) WHERE "token_id" = decode($3,$4) AND "project_address" = $5"#
                , &[&new_value.to_string(), &"hex".to_string(), &to_token_id.to_string(), &"hex".to_string(), &contract_address.to_owned()]).await {
                Ok(res) => {
                    debug!("project.transfer_value.update: {:#?}", res);
                    Ok(())
                }
                Err(e) => {
                    error!(
                        "project.transfer_value.update: Failed to update value {:#?}",
                        e
                    );

                    Err(PostgresError::from(e))
                }
            }
        }
        Err(e) => {
            error!("project.transfer_value.update: select token to update {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

pub async fn decrease_token_value<'a>(
    tx: &Transaction<'a>,
    contract_address: &str,
    token_id: &U256,
    value: U256,
) -> Result<(), PostgresError> {
    match tx.query_one(
        r#"SELECT "customer_token"."value" FROM "customer_token" WHERE "token_id" = decode($1, $2) AND "project_address" = $3"#
        , &[&token_id.to_string(), &"hex".to_string(), &contract_address.to_string()]).await {
        Ok(r) => {
            let old_value: U256 = r.get(0);
            let mut new_value = U256::zero();
            if old_value > value {
                new_value = old_value - value;
            }

            match tx.execute(
                r#"UPDATE "customer_token" set "value" = decode($1,$2) WHERE "token_id" = decode($3,$4) AND "project_address" = $5"#
                , &[&new_value.to_string(), &"hex".to_string(), &token_id.to_string(), &"hex".to_string(), &contract_address.to_string()]).await {
                Ok(res) => {
                    debug!("project.transfer_value.decrease_token_value: {:#?}", res);
                    Ok(())
                }
                Err(e) => {
                    error!("project.transfer_value.update: decreate_token_value {:#?}", e);
                    Err(PostgresError::from(e))
                }
            }
        }
        Err(e) => {
            error!("project.transfer_value.decrease_token_value: {:#?}", e);
            // This case might happen because transfer can happen on our own contracts (yielder
            // & offseter)
            Ok(())
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
    match tx.execute(
        r#"UPDATE "customer_token" SET "slot" = decode($1,$2) WHERE "token_id" = decode($3, $4) AND "project_address" = $5"#
        , &[&slot.to_string(), &"hex".to_string(), &token_id.to_string(), &"hex".to_string(), &contract_address.to_string()]).await {
        Ok(res) => {
            debug!("project.transfer_value.update: update_token_slot {:#?}", res);
            Ok(())
        }
        Err(e) => {
            error!("project.transfer_value.update: update_token_slot {:#?}", e);
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
    yielder_id: Ulid,
    amount: U256,
    time: OffsetDateTime,
) -> Result<(), PostgresError> {
    let id = Ulid::new();
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
    let id = Ulid::new();
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
) -> Option<Ulid> {
    let (sql, values) = Query::select()
        .from(YielderIden::Table)
        .column(YielderIden::Id)
        .and_where(Expr::col(YielderIden::Address).eq(yielder_address))
        .build_postgres(PostgresQueryBuilder);

    match tx.query_one(&sql, &values.as_params()).await {
        Ok(res) => {
            let yielder_id: Ulid = res.get(0);
            Some(yielder_id)
        }
        Err(_) => None,
    }
}

/// Update project total_supply when token is migrated
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`&str`]
/// * amount: [`U256`]
///
pub async fn update_project_total_value<'a>(
    tx: &Transaction<'a>,
    project_address: &str,
    slot: &U256,
    total_supply: &U256,
) -> Result<(), PostgresError> {
    let (sql, values) = Query::update()
        .table(ProjectIden::Table)
        .and_where(
            Expr::expr(Func::lower(Expr::col((
                ProjectIden::Table,
                ProjectIden::Address,
            ))))
            .eq(Func::lower(project_address)),
        )
        .and_where(Expr::col((ProjectIden::Table, ProjectIden::Slot)).eq(slot))
        .values([(ProjectIden::TotalSupply, total_supply.into())])
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(&sql, &values.as_params()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("minter.migration.error : {:#?}", e);
            Err(PostgresError::TokioPostgresError(e))
        }
    }
}

/// Update project project_value when `ProjectValueUpdate` is emitted
/// * tx: [`deadpool_postgres::Object`]
/// * contract_address: [`&str`]
/// * amount: [`U256`]
///
pub async fn update_project_project_value<'a>(
    tx: &Transaction<'a>,
    project_address: &str,
    slot: &U256,
    project_value: &U256,
) -> Result<(), PostgresError> {
    let (sql, values) = Query::update()
        .table(ProjectIden::Table)
        .and_where(
            Expr::expr(Func::lower(Expr::col((
                ProjectIden::Table,
                ProjectIden::Address,
            ))))
            .eq(Func::lower(project_address)),
        )
        .and_where(Expr::col((ProjectIden::Table, ProjectIden::Slot)).eq(slot))
        .values([(ProjectIden::ProjectValue, project_value.into())])
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(&sql, &values.as_params()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("project.project_value.error : {:#?}", e);
            Err(PostgresError::TokioPostgresError(e))
        }
    }
}

/// Append customer action on farms
/// * tx: [`deadpool_postgres::Object`]
/// * customer_address: [`&str`]
/// * project_address: [`&str`]
/// * slot: [`&U256`]
/// * value: [`&U256`]
/// * farm_type: [`FarmType`]
/// * action_type: [`ActionType`]
///
pub async fn append_customer_action<'a>(
    tx: &Transaction<'a>,
    event_id: &str,
    event_timestamp: OffsetDateTime,
    customer_address: &str,
    project_address: &str,
    slot: &U256,
    value: &U256,
    farm_type: FarmType,
    action_type: ActionType,
) -> Result<(), PostgresError> {
    let id = Ulid::new();
    let (sql, values) = Query::insert()
        .into_table(CustomerFarmIden::Table)
        .columns([
            CustomerFarmIden::Id,
            CustomerFarmIden::CustomerAddress,
            CustomerFarmIden::ProjectAddress,
            CustomerFarmIden::Slot,
            CustomerFarmIden::Value,
            CustomerFarmIden::FarmType,
            CustomerFarmIden::ActionType,
            CustomerFarmIden::EventId,
            CustomerFarmIden::EventTimestamp,
        ])
        .values([
            id.into(),
            Func::lower(customer_address).into(),
            project_address.into(),
            slot.into(),
            value.into(),
            sea_query::Value::String(Some(Box::new(Iden::to_string(&farm_type)))).into(),
            sea_query::Value::String(Some(Box::new(Iden::to_string(&action_type)))).into(),
            event_id.into(),
            event_timestamp.into(),
        ])?
        .build_postgres(PostgresQueryBuilder);

    match tx.execute(&sql, &values.as_params()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("customer_farm.{farm_type}.{action_type}.error : {:#?}", e);
            if e.code().eq(&Some(&SqlState::UNIQUE_VIOLATION)) {
                return Ok(());
            }
            Err(PostgresError::TokioPostgresError(e))
        }
    }
}

/// Find related project data for given contract
/// * tx: [`deadpool_postgres::Object`]
/// * addrses: [`&str`]
/// * farm_type: [`FarmType`]
///
pub async fn find_related_project_address_and_slot<'a>(
    tx: &Transaction<'a>,
    address: &str,
    farm_type: FarmType,
) -> Result<(String, U256), PostgresError> {
    match farm_type {
        FarmType::Enum => panic!("Should not use enum as a value"),
        FarmType::Yield => {
            let (sql, values) = Query::select()
                .columns([
                    (ProjectIden::Table, ProjectIden::Address),
                    (ProjectIden::Table, ProjectIden::Slot),
                ])
                .from(ProjectIden::Table)
                .inner_join(
                    YielderIden::Table,
                    Expr::col((YielderIden::Table, YielderIden::ProjectId))
                        .equals((ProjectIden::Table, ProjectIden::Id)),
                )
                .and_where(Expr::col((YielderIden::Table, YielderIden::Address)).eq(address))
                .build_postgres(PostgresQueryBuilder);

            match tx.query_one(sql.as_str(), &values.as_params()).await {
                Ok(res) => Ok((res.get(0), res.get(1))),
                Err(e) => Err(PostgresError::TokioPostgresError(e)),
            }
        }
        FarmType::Offset => {
            let (sql, values) = Query::select()
                .columns([
                    (ProjectIden::Table, ProjectIden::Address),
                    (ProjectIden::Table, ProjectIden::Slot),
                ])
                .from(ProjectIden::Table)
                .inner_join(
                    OffseterIden::Table,
                    Expr::col((OffseterIden::Table, OffseterIden::ProjectId))
                        .equals((ProjectIden::Table, ProjectIden::Id)),
                )
                .and_where(Expr::col((OffseterIden::Table, OffseterIden::Address)).eq(address))
                .build_postgres(PostgresQueryBuilder);

            match tx.query_one(sql.as_str(), &values.as_params()).await {
                Ok(res) => Ok((res.get(0), res.get(1))),
                Err(e) => Err(PostgresError::TokioPostgresError(e)),
            }
        }
    }
}
