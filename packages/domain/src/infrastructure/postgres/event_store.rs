use deadpool_postgres::{Object, Pool};
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use tracing::error;

use crate::{
    domain::{crypto::U256, Ulid},
    infrastructure::{
        postgres::{entity::EventStoreIden, PostgresError},
        view_model::DomainEventViewModel,
    },
};

/// Get last block from event_store
///
/// * `client_pool` - Postgres client pool
/// * `cla_starting_block` - Starting block number defined by command line argument
///
pub async fn get_last_dispatched_block(
    client_pool: &Pool,
    cla_starting_block: &u64,
) -> Result<u64, PostgresError> {
    let client = client_pool.get().await?;
    let (sql, values) = Query::select()
        .from(EventStoreIden::Table)
        .column((EventStoreIden::Table, EventStoreIden::BlockNumber))
        .order_by(
            (EventStoreIden::Table, EventStoreIden::RecordedAt),
            sea_query::Order::Desc,
        )
        .limit(1)
        .build_postgres(PostgresQueryBuilder);

    match client.query_one(&sql, &values.as_params()).await {
        Ok(res) => {
            let last_block: U256 = res.get(0);
            let last_block: u64 = last_block.into();
            if *cla_starting_block > last_block {
                return Ok(*cla_starting_block);
            }
            Ok(last_block)
        }
        Err(_e) => Ok(*cla_starting_block),
    }
}

pub async fn batch_events(
    client: &Object,
    limit: i8,
    key: Option<Ulid>,
) -> Result<Vec<DomainEventViewModel>, PostgresError> {
    let key = match key {
        Some(k) => k.to_string(),
        None => "".to_owned(),
    };
    match client
        .query(
            &format!(
                "SELECT * FROM event_store WHERE id > $1 ORDER BY id LIMIT {}",
                limit
            ),
            &[&key],
        )
        .await
    {
        Ok(res) => Ok(res.into_iter().map(|x| x.into()).collect()),
        Err(e) => {
            error!("{:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

pub async fn store_last_handled_event(
    client: &Object,
    key: Option<Ulid>,
) -> Result<(), PostgresError> {
    let key = match key {
        Some(k) => k.to_string(),
        None => "".to_owned(),
    };
    match client
        .execute("UPDATE last_stored_event set id = $1", &[&key])
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to store last handled event : {:#?}", e);
            Err(PostgresError::from(e))
        }
    }
}

pub async fn get_last_stored_event_block(client: &Object) -> Result<u64, PostgresError> {
    match client.query_one("SELECT es.block_number from last_stored_event lse INNER JOIN event_store es on es.id = lse.id", &[]).await {
        Ok(row) => Ok(row.get::<usize, U256>(0).into()),
        Err(_) => Err(PostgresError::FailedToFetchLastBlockNumber)
    }
}

pub async fn clear_view_models(client: &Object) -> Result<(), PostgresError> {
    let _ = client.execute(r#"TRUNCATE TABLE customer_farm"#, &[]).await;
    let _ = client
        .execute(r#"TRUNCATE TABLE customer_token"#, &[])
        .await;
    let _ = client
        .execute(r#"UPDATE last_stored_event set id = ''"#, &[])
        .await;
    Ok(())
}
