use crate::cli::{Cli, Commands};
use crate::key;
use clap::Parser;

pub mod query;
pub mod serve;

pub async fn dispatch() -> anyhow::Result<()> {
    init_logging()?;
    let cli = Cli::parse();
    match cli.command {
        Commands::Serve(params) => serve::run(params).await?,
        Commands::Simulate => {
            anyhow::bail!("simulate subcommand not yet implemented")
        }
        Commands::Query(params) => query::run(params).await?,
    }
    Ok(())
}

fn init_logging() -> anyhow::Result<()> {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()?;
    Ok(())
}

fn load_key(params: crate::cli::KeyManagementParams) -> anyhow::Result<Option<key::Keypair>> {
    if params.submit_dev_alice {
        Ok(Some(key::alice()))
    } else if let Some(path) = params.submit_private_key {
        Ok(Some(key::load(path)?))
    } else {
        Ok(None)
    }
}
