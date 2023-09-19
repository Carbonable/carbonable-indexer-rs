use actix_web::{web, Responder};
use carbonable_domain::infrastructure::postgres::event_store::get_last_dispatched_block;

use crate::{common::ApiError, AppDependencies};

pub async fn get_latest_block(
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let db_client_pool = data.db_client_pool.clone();
    let mut last_block_id = data.configuration.starting_block;
    last_block_id = get_last_dispatched_block(&db_client_pool, &last_block_id).await?;

    Ok(web::Json(last_block_id))
}

