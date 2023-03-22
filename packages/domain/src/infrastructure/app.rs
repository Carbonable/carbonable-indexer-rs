use clap::Parser;
use thiserror::Error;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(env = "NETWORK")]
    pub network: String,
    #[arg(env = "GATEWAY")]
    pub gateway: String,
    #[arg(env = "DATABASE_URI")]
    pub database_uri: String,
}

#[derive(Error, Debug)]
pub enum ConfigureApplicationError {
    #[error(transparent)]
    ConfigurationFailed(#[from] Box<dyn std::error::Error>),
}

pub async fn configure_application() -> Result<Args, ConfigureApplicationError> {
    Ok(Args::parse())
}
