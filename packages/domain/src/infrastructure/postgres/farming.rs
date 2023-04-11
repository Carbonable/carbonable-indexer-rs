use std::sync::Arc;

use deadpool_postgres::Pool;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use tracing::error;
use uuid::Uuid;

use crate::infrastructure::view_model::farming::{
    CompleteFarmingData, CustomerGlobalDataForComputation, FarmingProjectsViewModel,
};

use super::{
    entity::{
        ErcImplementation, MinterIden, OffseterIden, PaymentIden, ProjectIden, Snapshot,
        SnapshotIden, UriIden, VesterIden, Vesting, VestingIden, YielderIden,
    },
    PostgresError,
};

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
            .column((ProjectIden::Table, ProjectIden::Slot))
            .column((ProjectIden::Table, ProjectIden::Address))
            .column((YielderIden::Table, YielderIden::Address))
            .column((OffseterIden::Table, OffseterIden::Address))
            .column((VesterIden::Table, VesterIden::Address))
            .from(ProjectIden::Table)
            .left_join(
                YielderIden::Table,
                Expr::col((YielderIden::Table, YielderIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
            )
            .left_join(
                VesterIden::Table,
                Expr::col((VesterIden::Table, VesterIden::Id))
                    .equals((YielderIden::Table, YielderIden::VesterId)),
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
            ])
            .column((OffseterIden::Table, OffseterIden::Address))
            .columns([
                (YielderIden::Table, YielderIden::Id),
                (YielderIden::Table, YielderIden::Address),
            ])
            .column((VesterIden::Table, VesterIden::Address))
            .columns([
                (MinterIden::Table, MinterIden::Id),
                (MinterIden::Table, MinterIden::TotalValue),
            ])
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
                VesterIden::Table,
                Expr::col((VesterIden::Table, VesterIden::Id))
                    .equals((YielderIden::Table, YielderIden::VesterId)),
            )
            .left_join(
                MinterIden::Table,
                Expr::col((MinterIden::Table, MinterIden::ProjectId))
                    .equals((ProjectIden::Table, ProjectIden::Id)),
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

    pub async fn get_vestings(&self, yielder: Uuid) -> Result<Vec<Vesting>, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;
        let (sql, values) = Query::select()
            .from(VestingIden::Table)
            .columns([VestingIden::Id, VestingIden::Amount, VestingIden::Time])
            .and_where(Expr::col((VestingIden::Table, VestingIden::YielderId)).eq(yielder))
            .build_postgres(PostgresQueryBuilder);

        match client.query(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.into_iter().map(|row| row.into()).collect()),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }

    pub async fn get_total_value(&self, project_id: Uuid) -> Result<f64, PostgresError> {
        let client = self.db_client_pool.clone().get().await?;

        let (sql, values) = Query::select()
            .from(MinterIden::Table)
            .expr(Expr::col((MinterIden::Table, MinterIden::TotalValue)).sum())
            .and_where(Expr::col((MinterIden::Table, MinterIden::ProjectId)).eq(project_id))
            .build_postgres(PostgresQueryBuilder);

        match client.query_one(sql.as_str(), &values.as_params()).await {
            Ok(res) => Ok(res.get::<usize, f64>(0)),
            Err(e) => {
                error!("{:#?}", e);
                Err(PostgresError::TokioPostgresError(e))
            }
        }
    }
}
