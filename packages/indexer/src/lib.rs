pub mod filters;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("failed to configure out stream filters")]
    FilterConfigurationFailed,
    #[error(transparent)]
    FailedToReadDataContent(#[from] Box<dyn std::error::Error>),
}
