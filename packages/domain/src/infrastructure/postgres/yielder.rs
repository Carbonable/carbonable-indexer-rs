use std::{collections::HashMap, sync::Arc};

use deadpool_postgres::Pool;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use uuid::Uuid;

use crate::infrastructure::starknet::model::{StarknetValue, StarknetValueResolver};

use super::{entity::YielderIden, PostgresError};

#[derive(Debug)]
pub struct PostgresYielder {
    db_client_pool: Arc<Pool>,
}

impl PostgresYielder {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        project_id: Option<Uuid>,
        vester_id: Option<Uuid>,
        implementation_id: Option<Uuid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = uuid::Uuid::new_v4();
        let (sql, values) = Query::insert()
            .into_table(YielderIden::Table)
            .columns([
                YielderIden::Id,
                YielderIden::Address,
                YielderIden::TotalDeposited,
                YielderIden::TotalAbsorption,
                YielderIden::SnapshotTime,
                YielderIden::ProjectId,
                YielderIden::VesterId,
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
                data.get_mut("getSnapshotTime")
                    .expect("should have snapshotTime")
                    .resolve("datetime")
                    .into(),
                project_id.into(),
                vester_id.into(),
                implementation_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);
        let _res = client.execute(sql.as_str(), &values.as_params()).await?;

        Ok(())
    }
}
