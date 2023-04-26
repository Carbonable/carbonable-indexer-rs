use tokio::task::JoinHandle;

pub mod app;
pub mod in_memory;
pub mod postgres;
pub mod seed;
pub mod starknet;
pub mod view_model;

pub async fn flatten<T, E: std::error::Error + std::convert::From<tokio::task::JoinError>>(
    handle: JoinHandle<Result<T, E>>,
) -> Result<T, E> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(e),
        Err(err) => Err(err.into()),
    }
}
