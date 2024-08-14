use actix_web::{web, Responder};
use carbonable_domain::infrastructure::postgres::event_store::get_last_stored_event_block;

use crate::{common::ApiError, AppDependencies};

/// Returns last indexed block
pub async fn get_latest_block(
    data: web::Data<AppDependencies>,
) -> Result<impl Responder, ApiError> {
    let client = data.db_client_pool.get().await?;
    let last_block_id = get_last_stored_event_block(&client).await?;

    Ok(web::Json(last_block_id))
}
