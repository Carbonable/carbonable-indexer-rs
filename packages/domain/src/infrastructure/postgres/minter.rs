use std::{collections::HashMap, sync::Arc};

use deadpool_postgres::Pool;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use uuid::Uuid;

use crate::infrastructure::starknet::model::{StarknetValue, StarknetValueResolver};

use super::{entity::MinterIden, PostgresError};

#[derive(Debug)]
pub struct PostgresMinter {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresMinter {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        project_id: Option<Uuid>,
        payment_id: Option<Uuid>,
        implementation_id: Option<Uuid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = uuid::Uuid::new_v4();
        let (sql, values) = Query::insert()
            .into_table(MinterIden::Table)
            .columns([
                MinterIden::Id,
                MinterIden::Address,
                MinterIden::MaxSupply,
                MinterIden::ReservedSupply,
                MinterIden::PreSaleOpen,
                MinterIden::PublicSaleOpen,
                MinterIden::MaxBuyPerTx,
                MinterIden::UnitPrice,
                MinterIden::WhitelistMerkleRoot,
                MinterIden::SoldOut,
                MinterIden::TotalValue,
                MinterIden::ProjectId,
                MinterIden::PaymentId,
                MinterIden::ImplementationId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("getMaxSupplyForMint")
                    .expect("should have getMaxSupplyForMint")
                    .resolve("u64")
                    .into(),
                data.get_mut("getReservedSupplyForMint")
                    .expect("should have getReservedSupplyForMint")
                    .resolve("u64")
                    .into(),
                data.get_mut("isPreSaleOpen")
                    .expect("should have isPreSaleOpen")
                    .resolve("bool")
                    .into(),
                data.get_mut("isPublicSaleOpen")
                    .expect("should have isPublicSaleOpen")
                    .resolve("bool")
                    .into(),
                data.get_mut("getMaxBuyPerTx")
                    .expect("should have getMaxBuyPerTx")
                    .resolve("u64")
                    .into(),
                data.get_mut("getUnitPrice")
                    .expect("should have getUnitPrice")
                    .resolve("u64")
                    .into(),
                data.get_mut("getWhitelistMerkleRoot")
                    .expect("should have getWhitelistMerkleRoot")
                    .resolve("string")
                    .into(),
                data.get_mut("isSoldOut")
                    .expect("should have isSoldOut")
                    .resolve("bool")
                    .into(),
                data.get_mut("getTotalValue")
                    .expect("should have getTotalValue")
                    .resolve("u64")
                    .into(),
                project_id.into(),
                payment_id.into(),
                implementation_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);
        let _res = client.execute(sql.as_str(), &values.as_params()).await?;
        Ok(())
    }
}
