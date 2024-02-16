mod build;
mod cli;
mod logging;
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
    set_path(params.ci)?;

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

fn set_path(ci: bool) -> anyhow::Result<()> {
    let path = std::env::var("PATH").unwrap_or_else(|_| "".to_string());
    let new_path = if ci {
        // To ensure persistent storage between runs,
        // all cargo binaries are compiled in the following folder in ci
        format!("/cargo_target/release/:{}", path)
    } else {
        // adding target/release/ and demo/sovereign/target/release to path
        // so that zombienet will find sugondat-node as command
        // and the test can be shared between local testing and ci
        #[rustfmt::skip]
        let project_dir = duct::cmd!(
            "sh", "-c",
            "cargo locate-project | jq -r '.root' | grep -oE '^.*/'"
        )
        .stdout_capture()
        .run()?;
        let project_dir = std::str::from_utf8(&project_dir.stdout)?.trim();

        format!(
            "{}target/release/:{}demo/sovereign/target/release:{}",
            project_dir, project_dir, path
        )
    };
    std::env::set_var("PATH", new_path);
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
