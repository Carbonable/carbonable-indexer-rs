use std::{collections::HashMap, sync::Arc};

use deadpool_postgres::Pool;
use tokio_postgres::error::SqlState;

use crate::infrastructure::{
    postgres::entity::ProjectIden,
    starknet::model::StarknetValueResolver,
    view_model::{
        launchpad::LaunchpadProject, portfolio::ProjectWithMinterAndPaymentViewModel,
        project::ProjectViewModel,
    },
};
use sea_query::{Alias, Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use tracing::error;
use uuid::Uuid;

use crate::domain::crypto::U256;
use crate::infrastructure::starknet::model::StarknetValue;

use super::{
    entity::{
        ErcImplementation, ImplementationIden, MinterIden, OffseterIden, PaymentIden, Project,
        UriIden, YielderIden,
    },
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
                ProjectIden::ValueDecimals,
                ProjectIden::ErcImplementation,
            ])
            .and_where(Expr::col(ProjectIden::Address).eq(address))
            .build_postgres(PostgresQueryBuilder);

        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(Some(res.into())),
            Err(_) => Ok(None),
        }
    }

    pub async fn find_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<ProjectViewModel>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .columns([
                (ProjectIden::Table, ProjectIden::Id),
                (ProjectIden::Table, ProjectIden::Address),
                (ProjectIden::Table, ProjectIden::Name),
                (ProjectIden::Table, ProjectIden::Slug),
                (ProjectIden::Table, ProjectIden::ErcImplementation),
            ])
            .columns([
                (UriIden::Table, UriIden::Id),
                (UriIden::Table, UriIden::Uri),
                (UriIden::Table, UriIden::Data),
            ])
            .from(ProjectIden::Table)
            .left_join(
                UriIden::Table,
                Expr::col((ProjectIden::Table, ProjectIden::UriId))
                    .equals((UriIden::Table, UriIden::Id)),
            )
            .and_where(Expr::col((ProjectIden::Table, ProjectIden::Slug)).eq(slug))
            .and_where(
                Expr::col((ProjectIden::Table, ProjectIden::ErcImplementation))
                    .eq(Expr::val::<&str>(ErcImplementation::Erc3525.into())
                        .as_enum(ErcImplementation::Enum)),
            )
            .build_postgres(PostgresQueryBuilder);
        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(Some(res.into())),
            Err(e) => {
                error!("{:#?}", e);
                Ok(None)
            }
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
                ProjectIden::ValueDecimals,
                ProjectIden::ErcImplementation,
            ])
            .and_where(Expr::col(ProjectIden::Address).eq(address))
            .and_where(Expr::col(ProjectIden::Slot).eq(U256::from(*slot)))
            .build_postgres(PostgresQueryBuilder);

        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(Some(res.into())),
            Err(e) => {
                error!("{:#?}", e);
                Ok(None)
            }
        }
    }

    pub async fn find_projects_with_minter_and_payment(
        &self,
    ) -> Result<Vec<ProjectWithMinterAndPaymentViewModel>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .columns([
                (ProjectIden::Table, ProjectIden::Id),
                (ProjectIden::Table, ProjectIden::Address),
                (ProjectIden::Table, ProjectIden::Name),
                (ProjectIden::Table, ProjectIden::Slug),
                (ProjectIden::Table, ProjectIden::Slot),
                (ProjectIden::Table, ProjectIden::ErcImplementation),
                (ProjectIden::Table, ProjectIden::ValueDecimals),
            ])
            .columns([
                (MinterIden::Table, MinterIden::Id),
                (MinterIden::Table, MinterIden::UnitPrice),
            ])
            .column((PaymentIden::Table, PaymentIden::Symbol))
            .column((MinterIden::Table, MinterIden::Address))
            .columns([
                (PaymentIden::Table, PaymentIden::Id),
                (PaymentIden::Table, PaymentIden::Decimals),
            ])
            .column((ImplementationIden::Table, ImplementationIden::Abi))
            .column((Alias::new("minter_implementation"), ImplementationIden::Abi))
            .column((YielderIden::Table, YielderIden::Address))
            .column((OffseterIden::Table, OffseterIden::Address))
            .from(ProjectIden::Table)
            .left_join(
                MinterIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .left_join(
                PaymentIden::Table,
                Expr::col((PaymentIden::Table, PaymentIden::Id))
                    .equals((MinterIden::Table, MinterIden::PaymentId)),
            )
            .left_join(
                ImplementationIden::Table,
                Expr::col((ProjectIden::Table, ProjectIden::Address))
                    .equals((ImplementationIden::Table, ImplementationIden::Address)),
            )
            .left_join(
                YielderIden::Table,
                Expr::col((YielderIden::Table, YielderIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .left_join(
                OffseterIden::Table,
                Expr::col((OffseterIden::Table, OffseterIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .join_as(
                sea_query::JoinType::LeftJoin,
                ImplementationIden::Table,
                Alias::new("minter_implementation"),
                Expr::col((MinterIden::Table, MinterIden::Address)).equals((
                    Alias::new("minter_implementation"),
                    ImplementationIden::Address,
                )),
            )
            .build_postgres(PostgresQueryBuilder);
        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|r| r.into()).collect()),
            Err(e) => {
                error!("{:#?}", e);
                Ok(vec![])
            }
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
            None => "totalValue",
            Some(_) => "totalSupply",
        };
        let value_decimals = match data.get_mut("valueDecimals") {
            None => U256(crypto_bigint::U256::from_u64(0)).into(),
            Some(v) => v.resolve("u256").into(),
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
                ProjectIden::ValueDecimals,
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
                data.get_mut("slot")
                    .unwrap_or(&mut StarknetValue::from_resolved_value(
                        crate::infrastructure::starknet::model::StarknetResolvedValue::U256(U256(
                            crypto_bigint::U256::from_u8(0),
                        )),
                    ))
                    .resolve("u256")
                    .into(),
                data.get_mut("symbol")
                    .expect("should have symbol")
                    .resolve("string")
                    .into(),
                data.get_mut(total_supply_key)
                    .expect("total supply has to be provided")
                    .resolve("u256")
                    .into(),
                data.get_mut("owner")
                    .expect("owner has to be provided")
                    .resolve("address")
                    .into(),
                data.get_mut("getTonEquivalent")
                    .expect("ton equivalent has to be provided")
                    .resolve("u256")
                    .into(),
                data.get_mut("getTimes")
                    .expect("getTimes has to be provided")
                    .resolve("datetime_array")
                    .into(),
                data.get_mut("getAbsorptions")
                    .expect("getAbsorptions has to be provided")
                    .resolve("u256_array")
                    .into(),
                data.get_mut("isSetup")
                    .expect("isSetup has to be provided")
                    .resolve("bool")
                    .into(),
                value_decimals,
                sea_query::Value::String(Some(Box::new(erc_implementation.to_string()))).into(),
                implementation_id.into(),
                uri_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);

        match client.execute(sql.as_str(), &values.as_params()).await {
            Ok(_res) => Ok(()),
            Err(e) => {
                error!("while create project {:#?}", e);
                if e.code().eq(&Some(&SqlState::UNIQUE_VIOLATION)) {
                    return Ok(());
                }
                Err(e.into())
            }
        }
    }

    pub async fn get_launchpad_list(&self) -> Result<Vec<LaunchpadProject>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .from(ProjectIden::Table)
            .columns([
                (ProjectIden::Table, ProjectIden::Id),
                (ProjectIden::Table, ProjectIden::Address),
                (ProjectIden::Table, ProjectIden::Name),
                (ProjectIden::Table, ProjectIden::Slug),
                (ProjectIden::Table, ProjectIden::Setup),
            ])
            .columns([
                (UriIden::Table, UriIden::Id),
                (UriIden::Table, UriIden::Uri),
                (UriIden::Table, UriIden::Data),
            ])
            .columns([
                (MinterIden::Table, MinterIden::SaleDate),
                (MinterIden::Table, MinterIden::Address),
                (MinterIden::Table, MinterIden::PreSaleOpen),
                (MinterIden::Table, MinterIden::PublicSaleOpen),
                (MinterIden::Table, MinterIden::SoldOut),
            ])
            .column((ImplementationIden::Table, ImplementationIden::Abi))
            .inner_join(
                MinterIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .inner_join(
                UriIden::Table,
                Expr::col((UriIden::Table, UriIden::Id))
                    .equals((ProjectIden::Table, ProjectIden::UriId)),
            )
            .inner_join(
                ImplementationIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ImplementationId))
                    .equals((ImplementationIden::Table, ImplementationIden::Id)),
            )
            .and_where(
                Expr::col((ProjectIden::Table, ProjectIden::ErcImplementation))
                    .eq(Expr::val::<&str>(ErcImplementation::Erc3525.into())
                        .as_enum(ErcImplementation::Enum)),
            )
            .build_postgres(PostgresQueryBuilder);

        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|r| r.into()).collect()),
            Err(e) => Err(PostgresError::TokioPostgresError(e)),
        }
    }

    pub async fn get_launchpad_details(
        &self,
        slug: &str,
    ) -> Result<Option<LaunchpadProject>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .from(ProjectIden::Table)
            .columns([
                (ProjectIden::Table, ProjectIden::Id),
                (ProjectIden::Table, ProjectIden::Address),
                (ProjectIden::Table, ProjectIden::Name),
                (ProjectIden::Table, ProjectIden::Slug),
                (ProjectIden::Table, ProjectIden::Setup),
            ])
            .columns([
                (UriIden::Table, UriIden::Id),
                (UriIden::Table, UriIden::Uri),
                (UriIden::Table, UriIden::Data),
            ])
            .columns([
                (MinterIden::Table, MinterIden::SaleDate),
                (MinterIden::Table, MinterIden::Address),
                (MinterIden::Table, MinterIden::PreSaleOpen),
                (MinterIden::Table, MinterIden::PublicSaleOpen),
                (MinterIden::Table, MinterIden::SoldOut),
            ])
            .column((ImplementationIden::Table, ImplementationIden::Abi))
            .column((MinterIden::Table, MinterIden::Whitelist))
            .columns([
                (ProjectIden::Table, ProjectIden::TotalSupply),
                (ProjectIden::Table, ProjectIden::ForecastedApr),
            ])
            .column((MinterIden::Table, MinterIden::UnitPrice))
            .columns([
                (PaymentIden::Table, PaymentIden::Symbol),
                (PaymentIden::Table, PaymentIden::Decimals),
            ])
            .columns([
                (MinterIden::Table, MinterIden::MinValuePerTx),
                (MinterIden::Table, MinterIden::MaxValuePerTx),
                (MinterIden::Table, MinterIden::ReservedSupply),
                (MinterIden::Table, MinterIden::TotalValue),
            ])
            .column((PaymentIden::Table, PaymentIden::Address))
            .inner_join(
                MinterIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .inner_join(
                PaymentIden::Table,
                Expr::col((PaymentIden::Table, PaymentIden::Id))
                    .equals((MinterIden::Table, MinterIden::PaymentId)),
            )
            .inner_join(
                UriIden::Table,
                Expr::col((UriIden::Table, UriIden::Id))
                    .equals((ProjectIden::Table, ProjectIden::UriId)),
            )
            .inner_join(
                ImplementationIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ImplementationId))
                    .equals((ImplementationIden::Table, ImplementationIden::Id)),
            )
            .and_where(Expr::col((ProjectIden::Table, ProjectIden::Slug)).eq(slug))
            .and_where(
                Expr::col((ProjectIden::Table, ProjectIden::ErcImplementation))
                    .eq(Expr::val::<&str>(ErcImplementation::Erc3525.into())
                        .as_enum(ErcImplementation::Enum)),
            )
            .build_postgres(PostgresQueryBuilder);

        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(Some(res.into())),
            Err(_) => Ok(None),
        }
    }
}
