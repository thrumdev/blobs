use crate::{cli::test::BuildParams, run_maybe_quiet};
use duct::cmd;
use tracing::info;

// TODO: https://github.com/thrumdev/blobs/issues/225

pub fn build(params: BuildParams) -> anyhow::Result<()> {
    if params.skip {
        return Ok(());
    }

    // `it is advisable to use CARGO environmental variable to get the right cargo`
    // quoted by xtask readme
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    info!("Start building sugondat-node");
    run_maybe_quiet(
        cmd!(&cargo, "build", "-p", "sugondat-node", "--release"),
        params.quiet,
    )?;

    info!("Start building sugondat-node");
    run_maybe_quiet(
        cmd!(&cargo, "build", "-p", "sugondat-shim", "--release"),
        params.quiet,
    )?;

    info!("Start building sovereign demo-rollup");

    run_maybe_quiet(
        cmd!(&cargo, "build", "--release").dir("demo/sovereign/demo-rollup"),
        params.quiet,
    )?;

    Ok(())
}
