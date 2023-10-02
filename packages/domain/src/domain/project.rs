use thiserror::Error;
use tokio::task::JoinError;

use crate::infrastructure::{
    postgres::PostgresError,
    starknet::{model::ModelError, SequencerError},
};

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("invalid erc implementation for project")]
    InvalidErcImplementation,
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    SequencerError(#[from] SequencerError),
    #[error(transparent)]
    PostgresError(#[from] PostgresError),
}

pub fn format_ton<T>(value: T, ton_equivalent: T) -> T
where
    T: std::ops::Div<Output = T>,
{
    value / ton_equivalent
}
