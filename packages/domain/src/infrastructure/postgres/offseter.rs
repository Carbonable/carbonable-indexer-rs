use std::{collections::HashMap, sync::Arc};

use crate::domain::Ulid;
use deadpool_postgres::Pool;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;

use crate::{
    domain::{Contract, Erc3525, Erc721},
    infrastructure::starknet::model::{StarknetValue, StarknetValueResolver},
};

use super::{entity::OffseterIden, PostgresError};

#[derive(Debug)]
pub struct PostgresOffseter<C: Contract> {
    pub db_client_pool: Arc<Pool>,
    pub contract: std::marker::PhantomData<C>,
}

impl<C> PostgresOffseter<C>
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

impl PostgresOffseter<Erc721> {
    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        implementation_id: Option<Ulid>,
        project_id: Option<Ulid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = Ulid::new();
        let (sql, values) = Query::insert()
            .into_table(OffseterIden::Table)
            .columns([
                OffseterIden::Id,
                OffseterIden::Address,
                OffseterIden::TotalClaimable,
                OffseterIden::TotalDeposited,
                OffseterIden::TotalClaimed,
                OffseterIden::MinClaimable,
                OffseterIden::ProjectId,
                OffseterIden::ImplementationId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("getTotalClaimable")
                    .expect("should have totalClaimable")
                    .resolve("u64")
                    .into(),
                data.get_mut("getTotalDeposited")
                    .expect("should have totalDeposited")
                    .resolve("u64")
                    .into(),
                data.get_mut("getTotalClaimed")
                    .expect("should have totalClaimed")
                    .resolve("u64")
                    .into(),
                data.get_mut("getMinClaimable")
                    .expect("should have minClaimable")
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

impl PostgresOffseter<Erc3525> {
    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        implementation_id: Option<Ulid>,
        project_id: Option<Ulid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = Ulid::new();
        let (sql, values) = Query::insert()
            .into_table(OffseterIden::Table)
            .columns([
                OffseterIden::Id,
                OffseterIden::Address,
                OffseterIden::TotalClaimable,
                OffseterIden::TotalDeposited,
                OffseterIden::TotalClaimed,
                OffseterIden::ProjectId,
                OffseterIden::ImplementationId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("get_total_claimable")
                    .expect("should have totalClaimable")
                    .resolve("u64")
                    .into(),
                data.get_mut("get_total_deposited")
                    .expect("should have totalDeposited")
                    .resolve("u64")
                    .into(),
                data.get_mut("get_total_claimed")
                    .expect("should have totalClaimed")
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
