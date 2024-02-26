use crate::cli::{Cli, Commands};
use crate::key;
use anyhow::Context as _;
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
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
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

/// Reads the namespace from a given namespace specifier and checks its validity against known schemas.
///
/// The original namespace format is a 16-byte vector. so we support both the original format and
/// a more human-readable format, which is an unsigned 128-bit integer. To distinguish between the
/// two, the byte vector must be prefixed with `0x`.
///
/// The integer is interpreted as big-endian.
fn read_namespace(namespace: &str) -> anyhow::Result<ikura_nmt::Namespace> {
    let namespace = match namespace.strip_prefix("0x") {
        Some(hex) => {
            let namespace = hex::decode(hex)?;
            let namespace: [u8; 16] = namespace.try_into().map_err(|e: Vec<u8>| {
                anyhow::anyhow!("namespace must be 16 bytes long, but was {}", e.len())
            })?;
            ikura_nmt::Namespace::from_raw_bytes(namespace)
        }
        None => {
            let namespace = namespace
                .parse::<u128>()
                .with_context(|| format!("cannot parse namespace id '{}'", namespace))?;
            ikura_nmt::Namespace::from_u128_be(namespace)
        }
    };

    ikura_primitives::namespace::validate(&namespace.to_raw_bytes())
        .map_err(|e| anyhow::anyhow!("cannot validate the namespace, {}", e))?;
    Ok(namespace)
}
