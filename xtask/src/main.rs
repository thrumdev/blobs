mod build;
mod cli;
mod logging;
mod shim;
mod sovereign;
mod zombienet;

use clap::Parser;
use cli::{test, Cli, Commands};
use std::{path::PathBuf, str};

fn main() -> anyhow::Result<()> {
    init_logging()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Test(params) => test(params)?,
        Commands::Zombienet(params) => zombienet(params)?,
    }

    Ok(())
}

fn test(params: test::Params) -> anyhow::Result<()> {
    let project_path = obtain_project_path()?;

    init_env(&project_path, params.no_infer_bin_path)?;

    build::build(&project_path, params.build)?;

    // the variables must be kept alive and not dropped
    // otherwise the child process will be killed
    let _zombienet = zombienet::Zombienet::try_new(&project_path, params.zombienet)?;
    let _shim = shim::Shim::try_new(&project_path, params.shim)?;
    let sovereign = sovereign::Sovereign::try_new(&project_path, params.sovereign)?;

    // TODO: https://github.com/thrumdev/blobs/issues/226
    // Wait for the sovereign rollup to be ready
    std::thread::sleep(std::time::Duration::from_secs(20));

    sovereign.test_sovereign_rollup()?;

    Ok(())
}

fn zombienet(params: crate::cli::zombienet::Params) -> anyhow::Result<()> {
    let project_path = obtain_project_path()?;
    build::build(&project_path, params.build)?;
    let _zombienet = zombienet::Zombienet::try_new(&project_path, params.zombienet)?;
    wait_interrupt();
    Ok(())
}

fn obtain_project_path() -> anyhow::Result<PathBuf> {
    #[rustfmt::skip]
    let project_path = duct::cmd!(
        "sh", "-c",
        "cargo metadata --format-version 1 | jq -r '.workspace_root'"
    )
    .stdout_capture()
    .run()?;
    Ok(PathBuf::from(str::from_utf8(&project_path.stdout)?.trim()))
}

/// Blocks until ^C signal is delivered to this process. Uses global resource, don't proliferate.
fn wait_interrupt() {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .unwrap();
    let _ = rx.recv();
}

// Set up environment variables needed by the compilation and testing process.
//
// Add the sovereign constant manifest position through the
// CONSTANTS_MANIFEST new env variable and if no_infer_bin_path is not specified
// add to the path all required binaries.
fn init_env(project_path: &PathBuf, no_infer_bin_path: bool) -> anyhow::Result<()> {
    let path = project_path.join("demo/sovereign/constants.json");
    if !path.exists() {
        anyhow::bail!(
            "The `constants.json` file for Sovereign does not exist,\n \
                   or it is not in the expected position, `demo/sovereign/constants.json`"
        )
    }
    std::env::set_var("CONSTANTS_MANIFEST", path);

    if no_infer_bin_path {
        return Ok(());
    }

    let path = std::env::var("PATH").unwrap_or_else(|_| "".to_string());

    #[rustfmt::skip]
    let chain_target_path = duct::cmd!(
        "sh", "-c",
        "cargo metadata --format-version 1 | jq -r '.target_directory'"
    )
    .stdout_capture()
    .run()?;
    let chain_target_path = str::from_utf8(&chain_target_path.stdout)?.trim();

    #[rustfmt::skip]
    let sovereign_target_path = duct::cmd!(
        "sh", "-c",
        "cd demo/sovereign && cargo metadata --format-version 1 | jq -r '.target_directory'"
    )
    .stdout_capture()
    .run()?;
    let sovereign_target_path = str::from_utf8(&sovereign_target_path.stdout)?.trim();

    std::env::set_var(
        "PATH",
        format!("{chain_target_path}/release/:{sovereign_target_path}/release/:{path}"),
    );

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
