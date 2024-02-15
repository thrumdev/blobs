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
    init_env(params.ci)?;

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

// Set up environment variables needed by the compilation and testing process.
//
// If ci flag is specified, all the binaries are added to the path,
// and also the path to the sovereign constant manifest is added to the env variables.
fn init_env(ci: bool) -> anyhow::Result<()> {
    if ci {
        #[rustfmt::skip]
        let project_dir = duct::cmd!(
            "sh", "-c",
            "cargo locate-project | jq -r '.root' | grep -oE '^.*/'"
        )
        .stdout_capture()
        .run()?;
        let project_dir = std::str::from_utf8(&project_dir.stdout)?.trim();

        let path = std::env::var("PATH").unwrap_or_else(|_| "".to_string());

        // To ensure persistent storage between runs,
        // all cargo binaries are compiled in the following folder in ci
        let new_path = format!("/cargo_target/release/:{}", path);
        std::env::set_var("PATH", new_path);

        std::env::set_var(
            "CONSTANTS_MANIFEST",
            format!("{}demo/sovereign/constants.json", project_dir),
        );
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

fn check_binary(binary: &'static str, error_msg: &'static str) -> anyhow::Result<()> {
    if let Err(_) = duct::cmd!("sh", "-c", format!("command -v {}", binary))
        .stdout_null()
        .run()
    {
        anyhow::bail!(error_msg);
    }
    Ok(())
}
