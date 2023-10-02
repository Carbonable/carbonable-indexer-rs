use actix_web::ResponseError;
use carbonable_domain::{
    domain::project::ProjectError,
    infrastructure::{postgres::PostgresError, starknet::model::ModelError},
};
use deadpool_postgres::PoolError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServerResponse<T> {
    Data {
        data: T,
    },
    Error {
        code: usize,
        error_message: String,
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    ProjectError(#[from] ProjectError),
    #[error(transparent)]
    PostgresError(#[from] PostgresError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    PoolError(#[from] PoolError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

impl ResponseError for ApiError {}
