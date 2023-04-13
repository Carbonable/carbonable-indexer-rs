use std::{collections::HashMap, sync::Arc};

use deadpool_postgres::Pool;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::infrastructure::starknet::model::{StarknetValue, StarknetValueResolver};

use super::{
    entity::{Vester, VesterIden},
    PostgresError,
};

#[derive(Debug)]
pub struct PostgresVester {
    db_client_pool: Arc<Pool>,
}

impl PostgresVester {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn find_by_address(&self, address: &str) -> Result<Option<Vester>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .from(VesterIden::Table)
            .columns([
                VesterIden::Id,
                VesterIden::Address,
                VesterIden::TotalAmount,
                VesterIden::WithdrawableAmount,
            ])
            .and_where(Expr::col(VesterIden::Address).eq(address))
            .build_postgres(PostgresQueryBuilder);
        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(v) => Ok(Some(v.into())),
            Err(_) => Ok(None),
        }
    }

    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        implementation_id: Option<Uuid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = uuid::Uuid::new_v4();
        let (sql, values) = Query::insert()
            .into_table(VesterIden::Table)
            .columns([
                VesterIden::Id,
                VesterIden::Address,
                VesterIden::TotalAmount,
                VesterIden::WithdrawableAmount,
                VesterIden::ImplementationId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("vestings_total_amount")
                    .expect("should have token")
                    .resolve("u256")
                    .into(),
                data.get_mut("withdrawable_amount")
                    .expect("should have recipient")
                    .resolve("u256")
                    .into(),
                implementation_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);

        let _res = match client.execute(sql.as_str(), &values.as_params()).await {
            Ok(res) => res,
            Err(e) => {
                if e.code().eq(&Some(&SqlState::UNIQUE_VIOLATION)) {
                    return Ok(());
                }
                return Err(e.into());
            }
        };

        Ok(())
    }
}
