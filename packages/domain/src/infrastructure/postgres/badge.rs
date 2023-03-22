use deadpool_postgres::Pool;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_postgres::PostgresBinder;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

use crate::infrastructure::starknet::model::{StarknetValue, StarknetValueResolver};

use super::{entity::BadgeIden, PostgresError};

#[derive(Debug)]
pub struct PostgresBadge {
    pub db_client_pool: Arc<Pool>,
}

impl PostgresBadge {
    pub fn new(db_client_pool: Arc<Pool>) -> Self {
        Self { db_client_pool }
    }

    pub async fn create(
        &self,
        address: &str,
        mut data: HashMap<String, StarknetValue>,
        implementation_id: Option<Uuid>,
        uri_id: Option<Uuid>,
    ) -> Result<(), PostgresError> {
        let client = self.db_client_pool.get().await?;
        let id = uuid::Uuid::new_v4();

        let (sql, values) = Query::insert()
            .into_table(BadgeIden::Table)
            .columns([
                BadgeIden::Id,
                BadgeIden::Address,
                BadgeIden::Name,
                BadgeIden::Owner,
                BadgeIden::ImplementationId,
                BadgeIden::UriId,
            ])
            .values([
                id.into(),
                address.into(),
                data.get_mut("name")
                    .expect("should have name")
                    .resolve("string")
                    .into(),
                data.get_mut("owner")
                    .expect("should have owner")
                    .resolve("address")
                    .into(),
                implementation_id.into(),
                uri_id.into(),
            ])?
            .build_postgres(PostgresQueryBuilder);
        let _res = client.execute(sql.as_str(), &values.as_params()).await?;
        Ok(())
    }
}
