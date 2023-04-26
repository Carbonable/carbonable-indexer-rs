use deadpool_postgres::Pool;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;

use crate::{
    domain::crypto::U256,
    infrastructure::postgres::{entity::EventStoreIden, PostgresError},
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
