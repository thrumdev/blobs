use crate::serve;
use anyhow::bail;
use clap::{Parser, Subcommand};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Common parameters for all subcommands.
///
/// It's not declared on the `Cli` struct with `clap(flatten)` because of how the syntax
/// `sugondat-shim -p 10 serve --node-url` looks unintuitive.
#[derive(clap::Args, Debug)]
pub struct CommonParams {
    /// The address on which the shim should listen for incoming connections from the rollup nodes.
    #[clap(short, long, default_value = "127.0.0.1", group = "listen")]
    pub address: String,

    /// The port on which the shim should listen for incoming connections from the rollup nodes.
    #[clap(
        short,
        long,
        env = "SUGONDAT_SHIM_PORT",
        default_value = "10995",
        group = "listen"
    )]
    pub port: u16,
    // TODO: e.g. --submit-key, prometheus stuff, enabled adapters, etc.
}

impl CommonParams {
    /// Whether the sovereign adapter should be enabled.
    pub fn enable_sovereign(&self) -> bool {
        true
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    Serve(serve::Params),
    Simulate,
}

pub async fn run() -> anyhow::Result<()> {
    init_logging()?;
    let cli = Cli::parse();
    match cli.command {
        Commands::Serve(params) => serve::run(params).await?,
        Commands::Simulate => {
            bail!("simulate subcommand not yet implemented")
        }
    }
    Ok(())
}

fn init_logging() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .try_init()?;
    Ok(())
}
