use tokio::task::JoinHandle;
use tracing::error;

pub mod app;
pub mod in_memory;
pub mod postgres;
pub mod seed;
pub mod starknet;
pub mod view_model;

/// Flattens error into domain error to handle Result<> spawned tasks
/// * `handle` - [JoinHandle] - Handle to a spawned task
///
pub async fn flatten<T, E: std::error::Error + std::convert::From<tokio::task::JoinError>>(
    handle: JoinHandle<Result<T, E>>,
) -> Result<T, E> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => {
            error!("flatten Ok(Err(e)) => {:#?}", e);
            Err(e)
        }
        Err(err) => {
            error!("flatten Err(err) => {:#?}", err);
            Err(err.into())
        }
    }
}
