use deadpool_postgres::Pool;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use std::sync::Arc;
use tokio_postgres::error::SqlState;

use super::{
    entity::{Implementation, ImplementationIden},
    PostgresError,
};

#[derive(Debug)]
pub struct PostgresImplementation {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresImplementation {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn find_by_address(
        &self,
        address: &str,
    ) -> Result<Option<Implementation>, PostgresError> {
        let (sql, params) = Query::select()
            .column(ImplementationIden::Id)
            .column(ImplementationIden::Address)
            .column(ImplementationIden::Abi)
            .from(ImplementationIden::Table)
            .and_where(Expr::col(ImplementationIden::Address).eq(address))
            .build_postgres(PostgresQueryBuilder);

        match self
            .db_client_pool
            .get()
            .await?
            .query_one(sql.as_str(), &params.as_params())
            .await
        {
            Ok(v) => Ok(Some(v.into())),
            Err(_) => Ok(None),
        }
    }

    pub async fn create(
        &self,
        address: &str,
        abi: serde_json::Value,
    ) -> Result<Implementation, PostgresError> {
        let id = uuid::Uuid::new_v4();
        let (sql, values) = Query::insert()
            .into_table(ImplementationIden::Table)
            .columns([
                ImplementationIden::Id,
                ImplementationIden::Address,
                ImplementationIden::Abi,
            ])
            .values([id.into(), address.into(), abi.clone().into()])?
            .build_postgres(PostgresQueryBuilder);
        match self
            .db_client_pool
            .get()
            .await?
            .execute(sql.as_str(), &values.as_params())
            .await
        {
            Ok(_res) => Ok(Implementation {
                id,
                address: address.to_string(),
                abi,
            }),
            Err(err) => {
                if let Some(code) = err.code() {
                    if code.eq(&SqlState::UNIQUE_VIOLATION) {
                        return match self.find_by_address(address).await {
                            Ok(None) => Err(PostgresError::UnexpectedError),
                            Ok(Some(v)) => Ok(v),
                            Err(err) => Err(err),
                        };
                    }
                }
                Err(PostgresError::from(err))
            }
        }
    }
}
