mod build;
mod cli;
mod shim;
mod sovereign;
mod zombienet;

use clap::Parser;
use cli::{test, Cli, Commands};

fn main() -> anyhow::Result<()> {
    init_logging()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Test(params) => test(params)?,
    }

    Ok(())
}

fn test(params: test::Params) -> anyhow::Result<()> {
    build::build(params.build)?;

    // the variables must be kept alive and not dropped
    // otherwise the child process will be killed
    #[allow(unused)]
    let zombienet = zombienet::Zombienet::try_new(params.zombienet)?;
    #[allow(unused)]
    let shim = shim::Shim::try_new(params.shim)?;
    let sovereign = sovereign::Sovereign::try_new(params.sovereign)?;

    // TODO: https://github.com/thrumdev/blobs/issues/226
    // Wait for the sovereign rollup to be ready
    std::thread::sleep(std::time::Duration::from_secs(20));

    sovereign.test_sovereign_rollup()?;

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

fn run_maybe_quiet(cmd: duct::Expression, quiet: bool) -> std::io::Result<std::process::Output> {
    match quiet {
        true => cmd.stdout_null().run(),
        false => cmd.run(),
    }
}

fn start_maybe_quiet(cmd: duct::Expression, quiet: bool) -> std::io::Result<duct::Handle> {
    match quiet {
        true => cmd.stdout_null().start(),
        false => cmd.start(),
    }
}
