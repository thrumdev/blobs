use crate::{cli::test::BuildParams, logging::create_with_logs};
use duct::cmd;

// TODO: https://github.com/thrumdev/blobs/issues/225

pub fn build(params: BuildParams) -> anyhow::Result<()> {
    if params.skip {
        return Ok(());
    }

    tracing::info!("Building logs redirected {}", params.log_path);
    let with_logs = create_with_logs(params.log_path);

    // `it is advisable to use CARGO environmental variable to get the right cargo`
    // quoted by xtask readme
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    with_logs(
        "Building sugondat-node",
        cmd!(&cargo, "build", "-p", "sugondat-node", "--release"),
    )
    .run()?;

    with_logs(
        "Building sugondat-shim",
        cmd!(&cargo, "build", "-p", "sugondat-shim", "--release"),
    )
    .run()?;

    with_logs(
        "Building sovereign demo-rollup",
        cmd!(&cargo, "build", "--release").dir("demo/sovereign/demo-rollup"),
    )
    .run()?;

    Ok(())
}
