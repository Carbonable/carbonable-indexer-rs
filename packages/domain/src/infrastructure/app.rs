use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(name = "carbonable-indexer")]
#[command(subcommand_required = true)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Index {
        #[arg(env = "NETWORK")]
        network: String,
        #[arg(env = "GATEWAY")]
        gateway: String,
        #[arg(env = "DATABASE_URL")]
        database_uri: String,
        #[arg(env = "APIBARA_URI")]
        apibara_uri: String,
        #[arg(env = "APIBARA_TOKEN")]
        apibara_token: String,
        #[arg(long)]
        starting_block: Option<u64>,
        #[arg(long)]
        batch_size: Option<u64>,
        #[arg(long)]
        force: bool,
    },
    Seed {
        #[arg(env = "NETWORK")]
        network: String,
        #[arg(env = "GATEWAY")]
        gateway: String,
        #[arg(env = "DATABASE_URL")]
        database_uri: String,
    },
    EventStore {
        #[arg(env = "NETWORK")]
        network: String,
        #[arg(env = "GATEWAY")]
        gateway: String,
        #[arg(env = "DATABASE_URL")]
        database_uri: String,
        #[arg(long)]
        flush: bool,
    },
}

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(env = "NETWORK")]
    pub network: String,
    #[arg(env = "GATEWAY")]
    pub gateway: String,
    #[arg(env = "DATABASE_URL")]
    pub database_uri: String,
    #[arg(env = "APIBARA_URI")]
    pub apibara_uri: String,
    #[arg(env = "APIBARA_TOKEN")]
    pub apibara_token: String,
}
