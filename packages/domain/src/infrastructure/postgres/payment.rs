use std::{collections::HashMap, sync::Arc};

use deadpool_postgres::Pool;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;

use crate::{
    domain::Ulid,
    infrastructure::starknet::model::{StarknetValue, StarknetValueResolver},
};

use super::{
    entity::{Payment, PaymentIden},
    PostgresError,
};

#[derive(Debug)]
pub struct PostgresPayment {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresPayment {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn find_by_address(&self, address: &str) -> Result<Option<Payment>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .from(PaymentIden::Table)
            .columns([
                PaymentIden::Id,
                PaymentIden::Address,
                PaymentIden::Name,
                PaymentIden::Symbol,
                PaymentIden::Decimals,
            ])
            .and_where(Expr::col(PaymentIden::Address).eq(address))
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
    ) -> Result<Payment, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = Ulid::new();
        let name: String = data
            .get_mut("name")
            .expect("should have name")
            .resolve("string")
            .into();
        let symbol: String = data
            .get_mut("symbol")
            .expect("should have name")
            .resolve("string")
            .into();
        let decimals = data
            .get_mut("decimals")
            .expect("should have name")
            .resolve("u256");
        let (sql, values) = Query::insert()
            .into_table(PaymentIden::Table)
            .columns([
                PaymentIden::Id,
                PaymentIden::Address,
                PaymentIden::Name,
                PaymentIden::Symbol,
                PaymentIden::Decimals,
            ])
            .values([
                id.into(),
                address.into(),
                name.clone().into(),
                symbol.clone().into(),
                decimals.clone().into(),
            ])?
            .build_postgres(PostgresQueryBuilder);

        let _res = client.execute(sql.as_str(), &values.as_params()).await?;
        Ok(Payment {
            id,
            address: address.to_string(),
            name: name.to_string(),
            decimals: decimals.clone().into(),
            symbol: symbol.to_string(),
            implementation_id: None,
        })
    }
}
