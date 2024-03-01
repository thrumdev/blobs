mod build;
mod cli;
mod logging;
mod shim;
mod sovereign;
mod zombienet;

use clap::Parser;
use cli::{test, Cli, Commands};
use std::{os::unix::process::CommandExt, process::Command as StdCommand};
use std::{path::PathBuf, str};
use tokio::{signal, task};
use xshell::{cmd, Shell};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Test(params) => test(params).await?,
        Commands::Zombienet(params) => zombienet(params).await?,
    };

    Ok(())
}

async fn test(params: test::Params) -> anyhow::Result<()> {
    tokio::select!(
        _ = wait_interrupt() => Ok(()),
        res = test_procedure(params) => res
    )
}

async fn test_procedure(params: test::Params) -> anyhow::Result<()> {
    let project_path = obtain_project_path()?;
    init_env(&project_path, params.no_infer_bin_path)?;

    build::build(&project_path, params.build).await?;

    // the variables must be kept alive and not dropped
    // otherwise the child process will be killed
    let _zombienet = zombienet::Zombienet::try_new(&project_path, params.zombienet).await?;
    let _shim = shim::Shim::try_new(&project_path, params.shim).await?;
    let sovereign = sovereign::Sovereign::try_new(&project_path, params.sovereign).await?;

    // TODO: https://github.com/thrumdev/blobs/issues/226
    // Wait for the sovereign rollup to be ready
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    sovereign.test_sovereign_rollup().await?;

    Ok(())
}

async fn zombienet(params: crate::cli::zombienet::Params) -> anyhow::Result<()> {
    let project_path = obtain_project_path()?;
    init_env(&project_path, params.no_infer_bin_path)?;
    build::build(&project_path, params.build).await?;
    let _zombienet = zombienet::Zombienet::try_new(&project_path, params.zombienet).await?;
    wait_interrupt().await??;
    Ok(())
}

// Wait until ^C signal is delivered to this process
fn wait_interrupt() -> task::JoinHandle<anyhow::Result<()>> {
    task::spawn(async { signal::ctrl_c().await.map_err(|e| e.into()) })
}

// Extract from cargo metadata the specified key using the provided Shell
fn from_cargo_metadata(sh: &Shell, key: &str) -> anyhow::Result<String> {
    let cargo_metadata = cmd!(sh, "cargo metadata --format-version 1").read()?;
    let mut json: serde_json::Value = serde_json::from_str(&cargo_metadata)?;
    serde_json::from_value(json[key].take()).map_err(|e| e.into())
}

fn obtain_project_path() -> anyhow::Result<PathBuf> {
    let sh = xshell::Shell::new()?;
    let project_path = from_cargo_metadata(&sh, "workspace_root")?;
    Ok(PathBuf::from(project_path.trim()))
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

    let sh = xshell::Shell::new()?;

    sh.change_dir(project_path);
    let chain_target_path = from_cargo_metadata(&sh, "target_directory")?;

    sh.change_dir(project_path.join("demo/sovereign"));
    let sovereign_target_path = from_cargo_metadata(&sh, "target_directory")?;

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
    let sh = xshell::Shell::new()?;
    if let Err(_) = xshell::cmd!(sh, "sh -c")
        .arg(format!("command -v {binary}"))
        .quiet()
        .ignore_stdout()
        .run()
    {
        anyhow::bail!(error_msg);
    }
    Ok(())
}

// This is necessary because typically, a shell receiving a ctrl_c (SIGINT) signal will terminate
// the process and its children but not the processes spawned by them.
//
// In this testing tool, grandchild processes are required and need to be managed.
// Spawn a process with a group ID (pgid) and then call the `killpg` syscall
// to terminate all processes under the same pgid.
pub trait ProcessGroupId {
    // Convert Self into an std::process::Command with the specified pgid
    fn process_group(self, pgid: i32) -> StdCommand;
}

impl<'a> ProcessGroupId for xshell::Cmd<'a> {
    fn process_group(self, pgid: i32) -> StdCommand {
        let mut command = StdCommand::from(self);
        command.process_group(pgid);
        command
    }
}
