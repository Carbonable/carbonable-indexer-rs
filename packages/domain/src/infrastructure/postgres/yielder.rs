use std::{collections::HashMap, sync::Arc};

use crate::domain::Ulid;
use deadpool_postgres::Pool;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;

use crate::{
    domain::{Contract, Erc3525, Erc721},
    infrastructure::starknet::model::{StarknetValue, StarknetValueResolver},
};

use super::{entity::YielderIden, PostgresError};

#[derive(Debug)]
pub struct PostgresYielder<C: Contract> {
    db_client_pool: Arc<Pool>,
    contract: std::marker::PhantomData<C>,
}

impl<C> PostgresYielder<C>
where
    C: Contract + Send + Sync,
{
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self {
            db_client_pool,
            contract: std::marker::PhantomData::<C>,
        }
    }
}

impl PostgresYielder<Erc721> {
    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        project_id: Option<Ulid>,
        implementation_id: Option<Ulid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = Ulid::new();
        let (sql, values) = Query::insert()
            .into_table(YielderIden::Table)
            .columns([
                YielderIden::Id,
                YielderIden::Address,
                YielderIden::TotalDeposited,
                YielderIden::TotalAbsorption,
                YielderIden::SnapshotTime,
                YielderIden::ProjectId,
                YielderIden::ImplementationId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("getTotalDeposited")
                    .expect("should have totalDeposited")
                    .resolve("u64")
                    .into(),
                data.get_mut("getTotalAbsorption")
                    .expect("should have totalAbsorption")
                    .resolve("u64")
                    .into(),
                data.get_mut("getSnapshotedTime")
                    .expect("should have snapshotTime")
                    .resolve("datetime")
                    .into(),
                project_id.into(),
                implementation_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);
        let _res = client.execute(sql.as_str(), &values.as_params()).await?;

        Ok(())
    }
}

impl PostgresYielder<Erc3525> {
    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        project_id: Option<Ulid>,
        implementation_id: Option<Ulid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = Ulid::new();
        let (sql, values) = Query::insert()
            .into_table(YielderIden::Table)
            .columns([
                YielderIden::Id,
                YielderIden::Address,
                YielderIden::TotalDeposited,
                YielderIden::TotalAbsorption,
                YielderIden::ProjectId,
                YielderIden::ImplementationId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("get_total_deposited")
                    .expect("should have totalDeposited")
                    .resolve("u64")
                    .into(),
                data.get_mut("get_total_absorption")
                    .expect("should have totalAbsorption")
                    .resolve("u64")
                    .into(),
                project_id.into(),
                implementation_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);
        let _res = client.execute(sql.as_str(), &values.as_params()).await?;

        Ok(())
    }
}
