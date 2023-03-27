use std::{collections::HashMap, sync::Arc};

use deadpool_postgres::Pool;
use tokio_postgres::error::SqlState;

use crate::infrastructure::{
    postgres::entity::ProjectIden, starknet::model::StarknetValueResolver,
};
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use uuid::Uuid;

use crate::infrastructure::starknet::model::StarknetValue;

use super::{
    entity::{ErcImplementation, Project},
    PostgresError,
};

#[derive(Debug)]
pub struct PostgresProject {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresProject {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn find_by_address(&self, address: &str) -> Result<Option<Project>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .from(ProjectIden::Table)
            .columns([
                ProjectIden::Id,
                ProjectIden::Slug,
                ProjectIden::Address,
                ProjectIden::Name,
                ProjectIden::Slot,
                ProjectIden::Symbol,
                ProjectIden::TotalSupply,
                ProjectIden::Owner,
                ProjectIden::TonEquivalent,
                ProjectIden::Times,
                ProjectIden::Absorptions,
                ProjectIden::Setup,
                ProjectIden::ErcImplementation,
            ])
            .and_where(Expr::col(ProjectIden::Address).eq(address))
            .build_postgres(PostgresQueryBuilder);

        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(Some(res.into())),
            Err(_) => Ok(None),
        }
    }

    pub async fn find_by_address_and_slot(
        &self,
        address: &str,
        slot: &u64,
    ) -> Result<Option<Project>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .from(ProjectIden::Table)
            .columns([
                ProjectIden::Id,
                ProjectIden::Slug,
                ProjectIden::Address,
                ProjectIden::Name,
                ProjectIden::Slot,
                ProjectIden::Symbol,
                ProjectIden::TotalSupply,
                ProjectIden::Owner,
                ProjectIden::TonEquivalent,
                ProjectIden::Times,
                ProjectIden::Absorptions,
                ProjectIden::Setup,
                ProjectIden::ErcImplementation,
            ])
            .and_where(Expr::col(ProjectIden::Address).eq(address))
            .and_where(Expr::col(ProjectIden::Slot).eq(*slot))
            .build_postgres(PostgresQueryBuilder);

        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(Some(res.into())),
            Err(_) => Ok(None),
        }
    }

    pub async fn create(
        &self,
        mut data: HashMap<String, StarknetValue>,
        erc_implementation: ErcImplementation,
        implementation_id: Option<Uuid>,
        uri_id: Option<Uuid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let total_supply_key = match data.get("totalSupply") {
            None => "tokenSupplyInSlot",
            Some(_) => "totalSupply",
        };
        let (sql, values) = Query::insert()
            .into_table(ProjectIden::Table)
            .columns([
                ProjectIden::Id,
                ProjectIden::Slug,
                ProjectIden::Address,
                ProjectIden::Name,
                ProjectIden::Slot,
                ProjectIden::Symbol,
                ProjectIden::TotalSupply,
                ProjectIden::Owner,
                ProjectIden::TonEquivalent,
                ProjectIden::Times,
                ProjectIden::Absorptions,
                ProjectIden::Setup,
                ProjectIden::ErcImplementation,
                ProjectIden::ImplementationId,
                ProjectIden::UriId,
            ])
            .values([
                Uuid::new_v4().into(),
                data.get_mut("slug")
                    .expect("slug should be provided")
                    .resolve("string")
                    .into(),
                data.get_mut("address")
                    .expect("address has to be provided")
                    .resolve("address")
                    .into(),
                data.get_mut("name")
                    .expect("name has to be provided")
                    .resolve("string")
                    .into(),
                None::<i64>.into(),
                data.get_mut("symbol")
                    .expect("should have symbol")
                    .resolve("string")
                    .into(),
                data.get_mut(total_supply_key)
                    .expect("total supply has to be provided")
                    .resolve("u64")
                    .into(),
                data.get_mut("owner")
                    .expect("owner has to be provided")
                    .resolve("address")
                    .into(),
                data.get_mut("getTonEquivalent")
                    .expect("ton equivalent has to be provided")
                    .resolve("u64")
                    .into(),
                data.get_mut("getTimes")
                    .expect("getTimes has to be provided")
                    .resolve("u64_array")
                    .into(),
                data.get_mut("getAbsorptions")
                    .expect("getAbsorptions has to be provided")
                    .resolve("u64_array")
                    .into(),
                data.get_mut("isSetup")
                    .expect("isSetup has to be provided")
                    .resolve("bool")
                    .into(),
                Expr::val::<&str>(erc_implementation.into()).as_enum(ErcImplementation::Enum),
                implementation_id.into(),
                uri_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);
        let _result = match client.execute(sql.as_str(), &values.as_params()).await {
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
