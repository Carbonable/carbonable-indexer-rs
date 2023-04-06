use thiserror::Error;
use tokio::task::JoinError;

use crate::infrastructure::starknet::model::ModelError;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("invalid erc implementation for project")]
    InvalidErcImplementation,
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
}
