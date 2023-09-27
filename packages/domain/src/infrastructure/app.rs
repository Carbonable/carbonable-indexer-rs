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
    #[arg(env = "APIBARA_URI")]
    pub apibara_uri: String,
    #[arg(env = "APIBARA_TOKEN")]
    pub apibara_token: String,
    #[arg(long, default_value_t = 1, env = "STARTING_BLOCK")]
    pub starting_block: u64,
    #[arg(long, default_value_t = 10)]
    pub batch_size: u64,
    #[arg(long, default_value_t = false)]
    pub only_seed: bool,
    #[arg(long, default_value_t = false)]
    pub only_index: bool,
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Error, Debug)]
pub enum ConfigureApplicationError {
    #[error(transparent)]
    ConfigurationFailed(#[from] Box<dyn std::error::Error>),
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
}

pub async fn configure_application() -> Result<Args, ConfigureApplicationError> {
    Ok(Args::parse())
}
