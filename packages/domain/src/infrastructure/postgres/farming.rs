use std::sync::Arc;

use deadpool_postgres::Pool;
use sea_query::{Alias, Expr, JoinType, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use tracing::error;
use uuid::Uuid;

use crate::infrastructure::view_model::farming::{
    CompleteFarmingData, CustomerGlobalDataForComputation, FarmingProjectsViewModel,
};

use super::{
    entity::{
        ErcImplementation, ImplementationIden, MinterIden, OffseterIden, PaymentIden, ProjectIden,
        Provision, ProvisionIden, Snapshot, SnapshotIden, UriIden, YielderIden,
    },
    PostgresError,
};
use crate::domain::crypto::U256;

#[derive(Debug)]
pub struct PostgresFarming {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresFarming {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn get_farming_projects(
        &self,
    ) -> Result<Vec<FarmingProjectsViewModel>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let (sql, values) = Query::select()
            .from(ProjectIden::Table)
            .columns([
                (ProjectIden::Table, ProjectIden::Id),
                (ProjectIden::Table, ProjectIden::Address),
                (ProjectIden::Table, ProjectIden::Name),
                (ProjectIden::Table, ProjectIden::Slug),
            ])
            .columns([
                (UriIden::Table, UriIden::Uri),
                (UriIden::Table, UriIden::Address),
                (UriIden::Table, UriIden::Data),
            ])
            .left_join(
                UriIden::Table,
                Expr::col((UriIden::Table, UriIden::Id))
                    .equals((ProjectIden::Table, ProjectIden::UriId)),
            )
            .and_where(
                Expr::col((ProjectIden::Table, ProjectIden::ErcImplementation))
                    .eq(Expr::val::<&str>(ErcImplementation::Erc3525.into())
                        .as_enum(ErcImplementation::Enum)),
            )
            .build_postgres(PostgresQueryBuilder);

        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }

    pub async fn get_data_for_farming(
        &self,
        slug: Option<String>,
    ) -> Result<Vec<CustomerGlobalDataForComputation>, PostgresError> {
        let client = self.db_client_pool.get().await?;
        let mut query = Query::select()
            .column((ProjectIden::Table, ProjectIden::Id))
            .column((MinterIden::Table, MinterIden::UnitPrice))
            .column((PaymentIden::Table, PaymentIden::Decimals))
            .column((PaymentIden::Table, PaymentIden::Symbol))
            .column((ProjectIden::Table, ProjectIden::Slot))
            .column((ProjectIden::Table, ProjectIden::Address))
            .column((ProjectIden::Table, ProjectIden::ValueDecimals))
            .column((ProjectIden::Table, ProjectIden::TonEquivalent))
            .column((YielderIden::Table, YielderIden::Address))
            .column((OffseterIden::Table, OffseterIden::Address))
            .from(ProjectIden::Table)
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
            .left_join(
                MinterIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .left_join(
                PaymentIden::Table,
                Expr::col((MinterIden::Table, MinterIden::PaymentId))
                    .equals((PaymentIden::Table, PaymentIden::Id)),
            )
            .and_where(
                Expr::col((ProjectIden::Table, ProjectIden::ErcImplementation))
                    .eq(Expr::val::<&str>(ErcImplementation::Erc3525.into())
                        .as_enum(ErcImplementation::Enum)),
            )
            .to_owned();

        if let Some(slug) = slug {
            query.and_where(Expr::col((ProjectIden::Table, ProjectIden::Slug)).eq(slug));
        }

        let (sql, values) = query.build_postgres(PostgresQueryBuilder);
        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }

    pub async fn get_complete_farming_data(
        &self,
        slug: String,
    ) -> Result<Option<CompleteFarmingData>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .columns([
                (ProjectIden::Table, ProjectIden::Id),
                (ProjectIden::Table, ProjectIden::Address),
                (ProjectIden::Table, ProjectIden::Times),
                (ProjectIden::Table, ProjectIden::Absorptions),
                (ProjectIden::Table, ProjectIden::TonEquivalent),
                (ProjectIden::Table, ProjectIden::ValueDecimals),
            ])
            .columns([
                (PaymentIden::Table, PaymentIden::Decimals),
                (PaymentIden::Table, PaymentIden::Symbol),
                (PaymentIden::Table, PaymentIden::Address),
            ])
            .column((OffseterIden::Table, OffseterIden::Address))
            .columns([
                (YielderIden::Table, YielderIden::Id),
                (YielderIden::Table, YielderIden::Address),
            ])
            .columns([
                (MinterIden::Table, MinterIden::Id),
                (MinterIden::Table, MinterIden::TotalValue),
            ])
            .column((
                Alias::new("project_implementation"),
                ImplementationIden::Abi,
            ))
            .column((Alias::new("minter_implementation"), ImplementationIden::Abi))
            .column((
                Alias::new("offseter_implementation"),
                ImplementationIden::Abi,
            ))
            .column((
                Alias::new("yielder_implementation"),
                ImplementationIden::Abi,
            ))
            .column((
                Alias::new("payment_implementation"),
                ImplementationIden::Abi,
            ))
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
            .join_as(
                JoinType::LeftJoin,
                ImplementationIden::Table,
                Alias::new("project_implementation"),
                Expr::col((ProjectIden::Table, ProjectIden::Address)).equals((
                    Alias::new("project_implementation"),
                    ImplementationIden::Address,
                )),
            )
            .join_as(
                JoinType::LeftJoin,
                ImplementationIden::Table,
                Alias::new("minter_implementation"),
                Expr::col((MinterIden::Table, MinterIden::Address)).equals((
                    Alias::new("minter_implementation"),
                    ImplementationIden::Address,
                )),
            )
            .join_as(
                JoinType::LeftJoin,
                ImplementationIden::Table,
                Alias::new("offseter_implementation"),
                Expr::col((OffseterIden::Table, OffseterIden::Address)).equals((
                    Alias::new("offseter_implementation"),
                    ImplementationIden::Address,
                )),
            )
            .join_as(
                JoinType::LeftJoin,
                ImplementationIden::Table,
                Alias::new("yielder_implementation"),
                Expr::col((YielderIden::Table, YielderIden::Address)).equals((
                    Alias::new("yielder_implementation"),
                    ImplementationIden::Address,
                )),
            )
            .join_as(
                JoinType::LeftJoin,
                ImplementationIden::Table,
                Alias::new("payment_implementation"),
                Expr::col((PaymentIden::Table, PaymentIden::Address)).equals((
                    Alias::new("payment_implementation"),
                    ImplementationIden::Address,
                )),
            )
            .and_where(Expr::col((ProjectIden::Table, ProjectIden::Slug)).eq(slug))
            .and_where(
                Expr::col((ProjectIden::Table, ProjectIden::ErcImplementation))
                    .eq(Expr::val::<&str>(ErcImplementation::Erc3525.into())
                        .as_enum(ErcImplementation::Enum)),
            )
            .from(ProjectIden::Table)
            .build_postgres(PostgresQueryBuilder);

        match client.query_opt(sql.as_str(), &values.as_params()).await {
            Ok(None) => Ok(None),
            Ok(Some(res)) => Ok(Some(res.into())),
            Err(e) => {
                error!("{:#?}", e);
                Ok(None)
            }
        }
    }

    pub async fn get_snapshots(&self, yielder: Uuid) -> Result<Vec<Snapshot>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .from(SnapshotIden::Table)
            .columns([
                SnapshotIden::Id,
                SnapshotIden::PreviousTime,
                SnapshotIden::PreviousProjectAbsorption,
                SnapshotIden::PreviousYielderAbsorption,
                SnapshotIden::PreviousOffseterAbsorption,
                SnapshotIden::CurrentProjectAbsorption,
                SnapshotIden::CurrentYielderAbsorption,
                SnapshotIden::CurrentOffseterAbsorption,
                SnapshotIden::ProjectAbsorption,
                SnapshotIden::YielderAbsorption,
                SnapshotIden::OffseterAbsorption,
                SnapshotIden::Time,
            ])
            .and_where(Expr::col((SnapshotIden::Table, SnapshotIden::YielderId)).eq(yielder))
            .build_postgres(PostgresQueryBuilder);

        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }

    pub async fn get_provisions(&self, yielder: Uuid) -> Result<Vec<Provision>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .from(ProvisionIden::Table)
            .columns([
                ProvisionIden::Id,
                ProvisionIden::Amount,
                ProvisionIden::Time,
            ])
            .and_where(Expr::col((ProvisionIden::Table, ProvisionIden::YielderId)).eq(yielder))
            .build_postgres(PostgresQueryBuilder);

        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }

    pub async fn get_total_value(&self, project_id: Uuid) -> Result<U256, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;

        let (sql, values) = Query::select()
            .from(MinterIden::Table)
            .column((MinterIden::Table, MinterIden::TotalValue))
            .and_where(Expr::col((MinterIden::Table, MinterIden::ProjectId)).eq(project_id))
            .build_postgres(PostgresQueryBuilder);

        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res
                .iter()
                .map(|r| r.get(0))
                .fold(U256::zero(), |acc, x: Option<U256>| {
                    acc + x.unwrap_or(U256::zero())
                })),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }
}
