use crate::{domain::crypto::U256, infrastructure::view_model::customer::CustomerToken};

use super::{entity::CustomerTokenIden, PostgresError};
use deadpool_postgres::Pool;
use sea_query::{Expr, Func, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use std::sync::Arc;

#[derive(Debug)]
pub struct PostgresCustomer {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresCustomer {
    /// Creates a new [`PostgresCustomer`].
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    /// Get customer tokens from project given wallet and project address.
    /// * `wallet` - [`&str`] The wallet address.
    /// * `project_address` - [`&str`] The project address.
    ///
    /// # Errors
    /// * [`PostgresError`] - If query fails or if cannot get client pool.
    ///
    pub async fn get_customer_tokens(
        &self,
        wallet: &str,
        project_address: &str,
        slot: &U256,
    ) -> Result<Vec<CustomerToken>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .from(CustomerTokenIden::Table)
            .columns([
                (CustomerTokenIden::Table, CustomerTokenIden::Address),
                (CustomerTokenIden::Table, CustomerTokenIden::ProjectAddress),
                (CustomerTokenIden::Table, CustomerTokenIden::Slot),
                (CustomerTokenIden::Table, CustomerTokenIden::TokenId),
                (CustomerTokenIden::Table, CustomerTokenIden::Value),
                (CustomerTokenIden::Table, CustomerTokenIden::ValueDecimals),
            ])
            .and_where(
                Expr::expr(Func::lower(Expr::col((
                    CustomerTokenIden::Table,
                    CustomerTokenIden::Address,
                ))))
                .eq(Func::lower(wallet)),
            )
            .and_where(
                Expr::col((CustomerTokenIden::Table, CustomerTokenIden::ProjectAddress))
                    .eq(project_address),
            )
            .and_where(Expr::col((CustomerTokenIden::Table, CustomerTokenIden::Slot)).eq(slot))
            .build_postgres(PostgresQueryBuilder);
        match client.query(&sql, &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                tracing::error!("error while fetching customer tokens {:#?}", e);
                Err(e.into())
            }
        }
    }
    ///
    /// Get customer tokens
    /// * `wallet` - [`&str`] The wallet address.
    ///
    /// # Errors
    /// * [`PostgresError`] - If query fails or if cannot get client pool.
    ///
    pub async fn get_customer_erc3525_tokens(
        &self,
        wallet: &str,
    ) -> Result<Vec<CustomerToken>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .from(CustomerTokenIden::Table)
            .columns([
                (CustomerTokenIden::Table, CustomerTokenIden::Address),
                (CustomerTokenIden::Table, CustomerTokenIden::ProjectAddress),
                (CustomerTokenIden::Table, CustomerTokenIden::Slot),
                (CustomerTokenIden::Table, CustomerTokenIden::TokenId),
                (CustomerTokenIden::Table, CustomerTokenIden::Value),
                (CustomerTokenIden::Table, CustomerTokenIden::ValueDecimals),
            ])
            .and_where(
                Expr::expr(Func::lower(Expr::col((
                    CustomerTokenIden::Table,
                    CustomerTokenIden::Address,
                ))))
                .eq(Func::lower(wallet)),
            )
            .and_where(Expr::col((CustomerTokenIden::Table, CustomerTokenIden::Slot)).is_not_null())
            .build_postgres(PostgresQueryBuilder);
        match client.query(&sql, &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                tracing::error!("error while fetching customer tokens {:#?}", e);
                Err(e.into())
            }
        }
    }
}
